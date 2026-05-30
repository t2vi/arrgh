using System.Text.Json.Serialization;
using ArrghServer.Data;
using ArrghServer.Data.Models;
using Microsoft.EntityFrameworkCore;

namespace ArrghServer.Api;

// Shared chapter sync logic — called from Discover (on add) and Titles (on manual sync).
// Calls plugin-host /{source}/manga/{sourceId}/chapters, upserts chapters by number,
// then seeds chapter_sources. Dedup key: (title_id, number).

internal static class ChapterSync
{
    public static async Task<int> SyncFromSourceAsync(
        AppDbContext db,
        HttpClient http,
        string pluginHostUrl,
        string titleId,
        string contentType,
        string source,
        string sourceId)
    {
        var url = $"{pluginHostUrl.TrimEnd('/')}/{source}/manga/{Uri.EscapeDataString(sourceId)}/chapters";
        // Let network/HTTP errors propagate — callers (SyncTitleAsync) track anyError per source.
        var chapResp = await http.GetAsync(url);
        if (!chapResp.IsSuccessStatusCode)
            throw new HttpRequestException($"plugin-host {source} returned {(int)chapResp.StatusCode}");

        var pluginChapters = await chapResp.Content.ReadFromJsonAsync<List<PluginChapter>>(
            new System.Text.Json.JsonSerializerOptions { PropertyNameCaseInsensitive = true });
        if (pluginChapters is null || pluginChapters.Count == 0) return 0;

        var fmt = contentType == "novel" ? "text" : "pages";
        var now = DateTime.UtcNow;

        // Load existing chapters for dedup
        var existing = await db.Chapters
            .Where(c => c.TitleId == titleId)
            .ToDictionaryAsync(c => c.Number);

        foreach (var pc in pluginChapters)
        {
            var num = (double)pc.Number;
            if (double.IsNaN(num) || double.IsInfinity(num)) continue;

            if (!existing.ContainsKey(num))
            {
                var ch = new Chapter
                {
                    Id = Guid.NewGuid().ToString(),
                    TitleId = titleId,
                    Number = num,
                    Volume = pc.Volume.HasValue ? (double?)pc.Volume.Value : null,
                    ChapterTitle = pc.Title,
                    ChapterFormat = fmt,
                    IsNew = false,
                    PageCount = 0,
                    Downloaded = false,
                    CreatedAt = now,
                };
                db.Chapters.Add(ch);
                existing[num] = ch;
            }
        }
        await db.SaveChangesAsync();

        // Reload to get stable IDs after save
        db.ChangeTracker.Clear();
        var byNum = await db.Chapters
            .Where(c => c.TitleId == titleId)
            .ToDictionaryAsync(c => c.Number);

        // Seed chapter_sources (idempotent)
        var chapterIds = byNum.Values.Select(c => c.Id).ToList();
        var existingSources = chapterIds.Count == 0
            ? []
            : await db.ChapterSources
                .Where(cs => chapterIds.Contains(cs.ChapterId) && cs.Source == source)
                .Select(cs => cs.ChapterId)
                .ToListAsync();

        var existingPairs = existingSources.Select(id => (id, source)).ToHashSet();
        var pendingChapterIds = new HashSet<string>();

        foreach (var pc in pluginChapters)
        {
            var num = (double)pc.Number;
            if (!byNum.TryGetValue(num, out var ch)) continue;
            var srcId = pc.SourceId ?? pc.Id ?? "";
            if (string.IsNullOrEmpty(srcId)) continue;
            if (existingPairs.Contains((ch.Id, source))) continue;
            if (!pendingChapterIds.Add(ch.Id)) continue; // skip duplicate chapter_id within this batch

            db.ChapterSources.Add(new ChapterSource
            {
                Id = Guid.NewGuid().ToString(),
                ChapterId = ch.Id,
                Source = source,
                SourceId = srcId,
            });
        }
        await db.SaveChangesAsync();
        return pluginChapters.Count;
    }

    internal record PluginChapter(
        [property: JsonPropertyName("source_id")] string? SourceId,
        [property: JsonPropertyName("id")]        string? Id,
        [property: JsonPropertyName("number")]    decimal Number,
        [property: JsonPropertyName("volume")]    decimal? Volume,
        [property: JsonPropertyName("title")]     string? Title
    );
}
