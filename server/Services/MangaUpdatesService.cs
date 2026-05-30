using System.Net.Http.Json;
using System.Text.Json;
using ArrghServer.Api;

namespace ArrghServer.Services;

public record MuSeries(
    ulong SeriesId,
    string Title,
    string? Description,
    string? CoverUrl,
    string Status,
    string ContentType,
    string? Author,
    int? Year,
    string? Tags,
    IList<string> AssociatedNames
);

public class MangaUpdatesService(IHttpClientFactory httpFactory)
{
    const string Base = "https://api.mangaupdates.com/v1";

    public async Task<List<MuSeries>> SearchAsync(string q, int page = 1)
    {
        var http = httpFactory.CreateClient();
        var resp = await http.PostAsJsonAsync($"{Base}/series/search", new
        {
            search = q, stype = "title", page, per_page = 25,
        });
        resp.EnsureSuccessStatusCode();
        var json = await resp.Content.ReadFromJsonAsync<JsonElement>();
        return ParseSearchResponse(json);
    }

    public async Task<MuSeries?> SeriesDetailAsync(ulong id)
    {
        var http = httpFactory.CreateClient();
        var resp = await http.GetAsync($"{Base}/series/{id}");
        if (resp.StatusCode == System.Net.HttpStatusCode.NotFound) return null;
        resp.EnsureSuccessStatusCode();
        var json = await resp.Content.ReadFromJsonAsync<JsonElement>();
        return MapSeries(json);
    }

    public async Task<List<MuSeries>> LatestReleasesAsync()
    {
        var http = httpFactory.CreateClient();
        var resp = await http.PostAsJsonAsync($"{Base}/releases/search", new
        {
            per_page = 100, include_metadata = true,
        });
        resp.EnsureSuccessStatusCode();
        var json = await resp.Content.ReadFromJsonAsync<JsonElement>();

        var seen = new HashSet<ulong>();
        var seriesIds = new List<ulong>();

        foreach (var hit in json.GetProperty("results").EnumerateArray())
        {
            if (!hit.TryGetProperty("metadata", out var meta) || meta.ValueKind == JsonValueKind.Null) continue;
            if (!meta.TryGetProperty("series", out var series) || series.ValueKind == JsonValueKind.Null) continue;
            if (!series.TryGetProperty("series_id", out var idEl)) continue;
            var id = ParseFlexULong(idEl);
            if (id.HasValue && seen.Add(id.Value))
            {
                seriesIds.Add(id.Value);
                if (seriesIds.Count >= 20) break;
            }
        }

        if (seriesIds.Count == 0) return [];

        var tasks = seriesIds.Select(id => SeriesDetailAsync(id));
        var results = await Task.WhenAll(tasks);
        return results.Where(s => s is not null).Select(s => s!).ToList();
    }

    // ── Parsing ───────────────────────────────────────────────────────────────

    static List<MuSeries> ParseSearchResponse(JsonElement root)
    {
        var results = new List<MuSeries>();
        if (!root.TryGetProperty("results", out var arr)) return results;
        foreach (var hit in arr.EnumerateArray())
            if (hit.TryGetProperty("record", out var rec))
                results.Add(MapSeries(rec));
        return results;
    }

    internal static MuSeries MapSeries(JsonElement rec)
    {
        var seriesId = rec.TryGetProperty("series_id", out var sidEl) ? ParseFlexULong(sidEl) ?? 0 : 0UL;
        var title = rec.TryGetProperty("title", out var titleEl) ? titleEl.GetString() ?? "" : "";

        string? description = null;
        if (rec.TryGetProperty("description", out var descEl) && descEl.ValueKind == JsonValueKind.String)
        {
            var stripped = StripHtml(descEl.GetString()!);
            if (!string.IsNullOrEmpty(stripped)) description = stripped;
        }

        string? coverUrl = null;
        if (rec.TryGetProperty("image", out var imgEl) && imgEl.ValueKind == JsonValueKind.Object)
            if (imgEl.TryGetProperty("url", out var urlEl) && urlEl.ValueKind == JsonValueKind.Object)
                if (urlEl.TryGetProperty("original", out var origEl) && origEl.ValueKind == JsonValueKind.String)
                    coverUrl = origEl.GetString().NullIfEmpty();

        int? year = null;
        if (rec.TryGetProperty("year", out var yearEl))
            year = yearEl.ValueKind switch
            {
                JsonValueKind.String => int.TryParse(yearEl.GetString(), out var y) ? y : null,
                JsonValueKind.Number => yearEl.TryGetInt32(out var y) ? y : null,
                _ => null,
            };

        var contentType = rec.TryGetProperty("type", out var typeEl) && typeEl.ValueKind == JsonValueKind.String
            ? MapContentType(typeEl.GetString()) : "manga";

        var status = rec.TryGetProperty("status", out var statusEl) && statusEl.ValueKind == JsonValueKind.String
            ? statusEl.GetString()!.ToLowerInvariant() : "unknown";

        string? author = null;
        if (rec.TryGetProperty("authors", out var authorsEl) && authorsEl.ValueKind == JsonValueKind.Array)
        {
            var authors = authorsEl.EnumerateArray().ToList();
            var preferred = authors.FirstOrDefault(a =>
                a.TryGetProperty("type", out var t) && t.GetString() == "Author");
            var pick = preferred.ValueKind != JsonValueKind.Undefined ? preferred : authors.FirstOrDefault();
            if (pick.ValueKind != JsonValueKind.Undefined && pick.TryGetProperty("name", out var nameEl))
                author = nameEl.GetString();
        }

        string? tags = null;
        if (rec.TryGetProperty("genres", out var genresEl) && genresEl.ValueKind == JsonValueKind.Array)
        {
            var tagList = new List<string>();
            foreach (var g in genresEl.EnumerateArray())
            {
                if (!g.TryGetProperty("genre", out var gEl)) continue;
                var genre = gEl.GetString() ?? "";
                var lower = genre.ToLowerInvariant();
                tagList.Add(lower == "hentai" ? "hentai"
                    : new[] { "adult", "smut", "18+", "erotic" }.Contains(lower) ? "adult"
                    : genre);
            }
            if (tagList.Count > 0) tags = string.Join(",", tagList);
        }

        var associatedNames = new List<string>();
        if (rec.TryGetProperty("associated", out var assocEl) && assocEl.ValueKind == JsonValueKind.Array)
            foreach (var a in assocEl.EnumerateArray())
                if (a.TryGetProperty("title", out var t) && t.GetString() is { } name)
                    associatedNames.Add(name);

        return new MuSeries(seriesId, title, description, coverUrl, status, contentType, author, year, tags, associatedNames);
    }

    internal static string MapContentType(string? type) => type?.ToLowerInvariant() switch
    {
        "manhwa" => "manhwa",
        "manhua" => "manhua",
        "novel" or "web novel" or "light novel" or "oel" => "novel",
        _ => "manga",
    };

    internal static string StripHtml(string s)
    {
        var sb = new System.Text.StringBuilder(s.Length);
        bool inTag = false;
        foreach (var c in s)
        {
            if (c == '<') inTag = true;
            else if (c == '>') inTag = false;
            else if (!inTag) sb.Append(c);
        }
        return sb.ToString().Trim();
    }

    internal static ulong? ParseFlexULong(JsonElement el) => el.ValueKind switch
    {
        JsonValueKind.Number => el.TryGetUInt64(out var n) ? n : null,
        JsonValueKind.String => ulong.TryParse(el.GetString(), out var n) ? n : null,
        _ => null,
    };
}

internal static class StringExtensions
{
    public static string? NullIfEmpty(this string? s) =>
        string.IsNullOrEmpty(s) ? null : s;
}
