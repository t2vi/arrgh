using System.IO.Compression;
using System.Text.Json;
using System.Text.Json.Serialization;
using ArrghServer.Data;
using ArrghServer.Data.Models;
using Microsoft.EntityFrameworkCore;

namespace ArrghServer.Services;

/// <summary>
/// Background service: polls download_queue for pending items and processes them.
/// Mirrors Rust downloader/mod.rs tick() logic.
/// </summary>
public class DownloaderService(
    IServiceScopeFactory scopeFactory,
    IHttpClientFactory httpClientFactory,
    IConfiguration config,
    ILogger<DownloaderService> logger) : BackgroundService
{
    protected override async Task ExecuteAsync(CancellationToken stoppingToken)
    {
        while (!stoppingToken.IsCancellationRequested)
        {
            try { await TickAsync(stoppingToken); }
            catch (Exception ex) when (ex is not OperationCanceledException)
            { logger.LogDebug("downloader tick error: {Error}", ex.Message); }

            await Task.Delay(TimeSpan.FromSeconds(3), stoppingToken);
        }
    }

    private async Task TickAsync(CancellationToken ct)
    {
        using var scope = scopeFactory.CreateScope();
        var db = scope.ServiceProvider.GetRequiredService<AppDbContext>();

        // Claim one pending item (no distributed lock needed — SQLite serialises writes)
        var item = await db.DownloadQueue
            .Where(q => q.Status == "pending")
            .OrderBy(q => q.CreatedAt)
            .FirstOrDefaultAsync(ct);

        if (item is null) return;

        item.Status = "downloading";
        item.UpdatedAt = DateTime.UtcNow;
        await db.SaveChangesAsync(ct);

        await ProcessAsync(db, item, ct);
    }

    private async Task ProcessAsync(AppDbContext db, DownloadQueueItem item, CancellationToken ct)
    {
        // Load chapter + title info
        var info = await db.Chapters
            .Where(c => c.Id == item.ChapterId)
            .Select(c => new
            {
                c.ChapterFormat,
                c.Title.ContentType,
                c.Title.DownloadDir,
                TitleName = c.Title.TitleName,
            })
            .FirstOrDefaultAsync(ct);

        if (info is null)
        {
            await FailAsync(db, item.Id, "chapter not found", ct);
            return;
        }

        // Sources ordered by external_sources.priority (lower = preferred)
        var sources = await db.ChapterSources
            .Where(cs => cs.ChapterId == item.ChapterId)
            .GroupJoin(
                db.ExternalSources,
                cs => cs.Source,
                es => es.SourceKey,
                (cs, esGroup) => new { cs.Source, cs.SourceId, Priority = esGroup.Min(es => (int?)es.Priority) ?? 100 })
            .OrderBy(x => x.Priority)
            .ToListAsync(ct);

        if (sources.Count == 0)
        {
            await FailAsync(db, item.Id, "no chapter sources", ct);
            return;
        }

        var isText = info.ChapterFormat == "text";
        var ext = isText ? ".md" : ".cbz";
        var fileName = $"Ch. {item.ChapterNum:0000.#}{ext}";
        var dest = string.IsNullOrEmpty(info.DownloadDir)
            ? Path.Combine(
                config["DownloadDir"] ?? "./downloads",
                $"_{info.ContentType}",
                SanitizeTitle(info.TitleName),
                fileName)
            : Path.Combine(info.DownloadDir, fileName);

        var pluginHostUrl = config["PluginHostUrl"] ?? "http://plugin-host:4000";
        var http = httpClientFactory.CreateClient();
        http.DefaultRequestHeaders.UserAgent.ParseAdd("arrgh-server/1.0");

        string? lastError = null;
        foreach (var src in sources)
        {
            try
            {
                int pageCount;
                if (isText)
                {
                    await DownloadTextAsync(http, pluginHostUrl, src.Source, src.SourceId, dest, ct);
                    pageCount = 1;
                }
                else
                {
                    pageCount = await DownloadCbzAsync(http, pluginHostUrl, src.Source, src.SourceId, dest, item.Id, db, ct);
                }

                // Success
                var now = DateTime.UtcNow;
                await db.Chapters
                    .Where(c => c.Id == item.ChapterId)
                    .ExecuteUpdateAsync(s => s
                        .SetProperty(c => c.Downloaded, true)
                        .SetProperty(c => c.LocalPath, dest)
                        .SetProperty(c => c.PageCount, pageCount), ct);

                await db.DownloadQueue
                    .Where(q => q.Id == item.Id)
                    .ExecuteUpdateAsync(s => s
                        .SetProperty(q => q.Status, "done")
                        .SetProperty(q => q.UpdatedAt, now), ct);

                logger.LogInformation("downloaded: {Title} Ch.{Num} ({Source})", info.TitleName, item.ChapterNum, src.Source);
                return;
            }
            catch (Exception ex)
            {
                lastError = ex.Message;
                logger.LogDebug("source {Source} failed for Ch.{Num}: {Error}", src.Source, item.ChapterNum, ex.Message);
            }
        }

        await FailAsync(db, item.Id, lastError ?? "all sources failed", ct);
    }

    private async Task DownloadTextAsync(
        HttpClient http, string pluginHostUrl, string source, string sourceId, string dest, CancellationToken ct)
    {
        var url = $"{pluginHostUrl.TrimEnd('/')}/{source}/chapter/{Uri.EscapeDataString(sourceId)}/text";
        var res = await http.GetAsync(url, ct);
        if (!res.IsSuccessStatusCode)
            throw new HttpRequestException($"GET {url} → {(int)res.StatusCode} {res.ReasonPhrase}");
        var text = await res.Content.ReadAsStringAsync(ct);

        Directory.CreateDirectory(Path.GetDirectoryName(dest)!);
        await File.WriteAllTextAsync(dest, text, ct);
    }

    private async Task<int> DownloadCbzAsync(
        HttpClient http, string pluginHostUrl, string source, string sourceId, string dest,
        string queueId, AppDbContext db, CancellationToken ct)
    {
        var url = $"{pluginHostUrl.TrimEnd('/')}/{source}/chapter/{Uri.EscapeDataString(sourceId)}/pages";
        var res = await http.GetAsync(url, ct);
        if (!res.IsSuccessStatusCode)
            throw new HttpRequestException($"GET {url} → {(int)res.StatusCode} {res.ReasonPhrase}");

        var json = await res.Content.ReadAsStringAsync(ct);
        var pages = ParsePageUrls(json);

        if (pages.Count == 0)
            throw new InvalidOperationException($"source returned 0 pages for chapter {sourceId}");

        await db.DownloadQueue
            .Where(q => q.Id == queueId)
            .ExecuteUpdateAsync(s => s
                .SetProperty(q => q.PagesTotal, pages.Count)
                .SetProperty(q => q.PagesDownloaded, 0), ct);

        Directory.CreateDirectory(Path.GetDirectoryName(dest)!);

        using var fs = new FileStream(dest, FileMode.Create, FileAccess.Write);
        using var zip = new ZipArchive(fs, ZipArchiveMode.Create, leaveOpen: false);

        for (var i = 0; i < pages.Count; i++)
        {
            var (pageUrl, referer) = pages[i];
            using var req = new HttpRequestMessage(HttpMethod.Get, pageUrl);
            if (referer is not null) req.Headers.Add("Referer", referer);
            using var imgRes = await http.SendAsync(req, ct);
            if (!imgRes.IsSuccessStatusCode)
                throw new HttpRequestException($"GET {pageUrl} → {(int)imgRes.StatusCode} {imgRes.ReasonPhrase}");
            var bytes = await imgRes.Content.ReadAsByteArrayAsync(ct);

            var ext = (pageUrl.Split('?')[0].Split('.').LastOrDefault() ?? "jpg").ToLowerInvariant();
            var entry = zip.CreateEntry($"{i:0000}.{ext}", CompressionLevel.NoCompression);
            using var entryStream = entry.Open();
            await entryStream.WriteAsync(bytes, ct);

            await db.DownloadQueue
                .Where(q => q.Id == queueId)
                .ExecuteUpdateAsync(s => s.SetProperty(q => q.PagesDownloaded, i + 1), ct);
        }

        return pages.Count;
    }

    private async Task FailAsync(AppDbContext db, string queueId, string error, CancellationToken ct)
    {
        logger.LogError("download failed for queue item {Id}: {Error}", queueId, error);
        await db.DownloadQueue
            .Where(q => q.Id == queueId)
            .ExecuteUpdateAsync(s => s
                .SetProperty(q => q.Status, "error")
                .SetProperty(q => q.Error, error)
                .SetProperty(q => q.UpdatedAt, DateTime.UtcNow), ct);
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    private static List<(string Url, string? Referer)> ParsePageUrls(string json)
    {
        var result = new List<(string, string?)>();
        using var doc = JsonDocument.Parse(json);
        foreach (var el in doc.RootElement.EnumerateArray())
        {
            if (el.ValueKind == JsonValueKind.String)
            {
                result.Add((el.GetString()!, null));
            }
            else if (el.ValueKind == JsonValueKind.Object)
            {
                var url = el.GetProperty("url").GetString()!;
                string? referer = el.TryGetProperty("referer", out var r) ? r.GetString() : null;
                result.Add((url, referer));
            }
        }
        return result;
    }

    private static string SanitizeTitle(string title) =>
        string.Concat(title.Select(c => Path.GetInvalidFileNameChars().Contains(c) ? '_' : c))
              .Trim('_', ' ');
}
