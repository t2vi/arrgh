using System.IO.Compression;
using System.Text.Json;
using ArrghServer.Data;
using ArrghServer.Data.Models;
using ArrghServer.Services;
using Microsoft.EntityFrameworkCore;

namespace ArrghServer.Api;

public static class Media
{
    public static RouteGroupBuilder MapMediaRoutes(this RouteGroupBuilder group)
    {
        group.MapGet("/page/{chapterId}/{page:int}", ServePage);
        group.MapGet("/cover/{titleId}", ServeCover);
        group.MapGet("/meta-cover", ServeMetaCover);
        group.MapGet("/proxy", ProxyImage);
        return group;
    }

    // ── GET /api/media/page/{chapterId}/{page} ────────────────────────────────

    static async Task<IResult> ServePage(
        string chapterId, int page,
        AppDbContext db, PageCacheService pageCache, IHttpClientFactory httpFactory)
    {
        var chapter = await db.Chapters
            .Where(c => c.Id == chapterId)
            .Select(c => new { c.LocalPath, c.Downloaded })
            .FirstOrDefaultAsync();

        if (chapter is null) return Results.NotFound();

        // Local file path — try first when downloaded
        if (chapter.Downloaded && chapter.LocalPath is not null)
        {
            var fileData = await GetChapterPageAsync(chapter.LocalPath, page);
            if (fileData is not null)
            {
                var ct = DetectContentType(fileData);
                if (ct is not null)
                    return Results.Bytes(StripJpegIcc(fileData), ct);
            }
            // Corrupt or missing — reset flag and fall through to source fetch
            await db.Chapters
                .Where(c => c.Id == chapterId)
                .ExecuteUpdateAsync(s => s
                    .SetProperty(c => c.Downloaded, false)
                    .SetProperty(c => c.LocalPath, (string?)null));
        }

        // Source links ordered by priority
        var sourceLinks = await (
            from cs in db.ChapterSources
            where cs.ChapterId == chapterId
            join es in db.ExternalSources on cs.Source equals es.SourceKey into esGroup
            from es in esGroup.DefaultIfEmpty()
            orderby es != null ? es.Priority : 100
            select new { cs.Source, cs.SourceId, BaseUrl = es != null ? es.BaseUrl : null }
        ).ToListAsync();

        if (sourceLinks.Count == 0) return Results.NotFound();

        // Try cache
        var cached = pageCache.Get(chapterId);
        List<PageUrlEntry> pages;

        if (cached is not null)
        {
            pages = cached;
        }
        else
        {
            List<PageUrlEntry>? fetched = null;
            var http = httpFactory.CreateClient();
            foreach (var link in sourceLinks)
            {
                if (link.BaseUrl is null) continue;
                try
                {
                    var url = $"{link.BaseUrl.TrimEnd('/')}/chapter/{Uri.EscapeDataString(link.SourceId)}/pages";
                    var json = await http.GetFromJsonAsync<JsonElement[]>(url);
                    if (json is null) continue;
                    fetched = ParsePageUrls(json);
                    break;
                }
                catch { continue; }
            }
            if (fetched is null) return Results.StatusCode(502);
            pageCache.Set(chapterId, fetched);
            pages = fetched;
        }

        if (page >= pages.Count) return Results.NotFound();

        var entry = pages[page];
        var client = httpFactory.CreateClient();
        var req = new HttpRequestMessage(HttpMethod.Get, entry.Url);
        req.Headers.Add("User-Agent", "Mozilla/5.0");
        if (entry.Referer is not null)
            req.Headers.Add("Referer", entry.Referer);

        HttpResponseMessage resp;
        try { resp = await client.SendAsync(req); }
        catch { return Results.StatusCode(502); }

        var bytes = await resp.Content.ReadAsByteArrayAsync();
        var contentType = resp.Content.Headers.ContentType?.MediaType ?? "image/jpeg";
        return Results.Bytes(StripJpegIcc(bytes), contentType);
    }

    // ── GET /api/media/cover/{titleId} ────────────────────────────────────────

    static async Task<IResult> ServeCover(string titleId, AppDbContext db)
    {
        var title = await db.Titles
            .Where(t => t.Id == titleId)
            .Select(t => new { t.CoverUrl, t.TitleName })
            .FirstOrDefaultAsync();

        if (title is null) return Results.NotFound();

        if (title.CoverUrl is not null)
        {
            if (title.CoverUrl.StartsWith("http") || title.CoverUrl.StartsWith("/api/"))
                return Results.Redirect(title.CoverUrl, permanent: false);

            try
            {
                var data = await File.ReadAllBytesAsync(title.CoverUrl);
                var ct = ExtFromPath(title.CoverUrl);
                return Results.Bytes(data, ct);
            }
            catch
            {
                // Local file missing — null out and try CDN fallback
                await db.Titles
                    .Where(t => t.Id == titleId)
                    .ExecuteUpdateAsync(s => s.SetProperty(t => t.CoverUrl, (string?)null));
            }
        }

        // CDN fallback via title_meta
        var key = NormalizeTitle(title.TitleName);
        var cdnUrl = await db.TitleMeta
            .Where(m => m.TitleKey == key)
            .Select(m => m.CoverCdnUrl)
            .FirstOrDefaultAsync();

        if (cdnUrl is not null)
        {
            await db.Titles
                .Where(t => t.Id == titleId)
                .ExecuteUpdateAsync(s => s.SetProperty(t => t.CoverUrl, cdnUrl));
            return Results.Redirect(cdnUrl, permanent: false);
        }

        return Results.NotFound();
    }

    // ── GET /api/media/meta-cover ─────────────────────────────────────────────

    static async Task<IResult> ServeMetaCover(string key, AppDbContext db)
    {
        var row = await db.TitleMeta
            .Where(m => m.TitleKey == key)
            .Select(m => new { m.CoverLocalPath, m.CoverCdnUrl })
            .FirstOrDefaultAsync();

        if (row is null) return Results.NotFound();

        if (row.CoverLocalPath is not null)
        {
            try
            {
                var data = await File.ReadAllBytesAsync(row.CoverLocalPath);
                if (DetectContentType(data) is not null)
                {
                    var ct = ExtFromPath(row.CoverLocalPath);
                    return Results.Bytes(data, ct);
                }
                // Corrupt — clear path
                await db.TitleMeta
                    .Where(m => m.TitleKey == key)
                    .ExecuteUpdateAsync(s => s.SetProperty(m => m.CoverLocalPath, (string?)null));
            }
            catch
            {
                await db.TitleMeta
                    .Where(m => m.TitleKey == key)
                    .ExecuteUpdateAsync(s => s.SetProperty(m => m.CoverLocalPath, (string?)null));
            }
        }

        if (row.CoverCdnUrl is not null)
            return Results.Redirect($"/api/media/proxy?url={Uri.EscapeDataString(row.CoverCdnUrl)}", permanent: false);

        return Results.NotFound();
    }

    // ── GET /api/media/proxy ──────────────────────────────────────────────────

    static async Task<IResult> ProxyImage(string url, IHttpClientFactory httpFactory)
    {
        var referer = RootDomainReferer(url);
        var http = httpFactory.CreateClient();
        var req = new HttpRequestMessage(HttpMethod.Get, url);
        req.Headers.Add("User-Agent", "Mozilla/5.0");
        if (!string.IsNullOrEmpty(referer))
            req.Headers.Add("Referer", referer);

        HttpResponseMessage resp;
        try { resp = await http.SendAsync(req); }
        catch { return Results.StatusCode(502); }

        if (!resp.IsSuccessStatusCode) return Results.StatusCode(502);

        var bytes = await resp.Content.ReadAsByteArrayAsync();
        var ct = url.Contains(".webp") ? "image/webp"
               : url.Contains(".png")  ? "image/png"
               : "image/jpeg";
        return Results.Bytes(bytes, ct);
    }

    // ── Pure helpers — unit-testable ──────────────────────────────────────────

    internal static string? DetectContentType(byte[] data)
    {
        if (data.Length < 4) return null;
        if (data[0] == 0xFF && data[1] == 0xD8) return "image/jpeg";
        if (data[0] == 0x89 && data[1] == 0x50 && data[2] == 0x4E && data[3] == 0x47) return "image/png";
        if (data.Length >= 12 &&
            data[0] == 0x52 && data[1] == 0x49 && data[2] == 0x46 && data[3] == 0x46 && // RIFF
            data[8] == 0x57 && data[9] == 0x45 && data[10] == 0x42 && data[11] == 0x50)  // WEBP
            return "image/webp";
        if (data[0] == 0x47 && data[1] == 0x49 && data[2] == 0x46 && data[3] == 0x38) return "image/gif"; // GIF8
        if (data.Length >= 8 && data[4] == 0x66 && data[5] == 0x74 && data[6] == 0x79 && data[7] == 0x70) return "image/avif"; // ftyp
        return null;
    }

    internal static byte[] StripJpegIcc(byte[] data)
    {
        if (data.Length < 4 || data[0] != 0xFF || data[1] != 0xD8) return data;

        var output = new List<byte>(data.Length) { 0xFF, 0xD8 };
        int i = 2;

        while (i < data.Length)
        {
            if (data[i] != 0xFF) { output.AddRange(data[i..]); break; }
            i++;
            while (i < data.Length && data[i] == 0xFF) i++; // padding
            if (i >= data.Length) break;

            byte marker = data[i++];

            if (marker == 0xD8) { output.AddRange((byte[])[0xFF, 0xD8]); continue; }
            if (marker == 0xD9) { output.AddRange((byte[])[0xFF, 0xD9]); break; }
            if (marker is >= 0xD0 and <= 0xD7) { output.Add(0xFF); output.Add(marker); continue; }
            if (marker == 0xDA) { output.Add(0xFF); output.Add(0xDA); output.AddRange(data[i..]); break; }

            if (i + 2 > data.Length) break;
            int segLen = (data[i] << 8) | data[i + 1];
            if (i + segLen > data.Length) break;

            // APP2 ICC_PROFILE — drop
            var iccSig = "ICC_PROFILE\0"u8;
            if (marker == 0xE2 && segLen >= 2 + iccSig.Length)
            {
                var payload = data.AsSpan(i + 2, segLen - 2);
                if (payload.StartsWith(iccSig)) { i += segLen; continue; }
            }

            output.Add(0xFF); output.Add(marker);
            output.AddRange(data[i..(i + segLen)]);
            i += segLen;
        }

        return output.ToArray();
    }

    internal static bool IsImage(string name)
    {
        var lower = name.ToLowerInvariant();
        return lower.EndsWith(".jpg") || lower.EndsWith(".jpeg") || lower.EndsWith(".png")
            || lower.EndsWith(".webp") || lower.EndsWith(".avif");
    }

    internal static string RootDomainReferer(string url)
    {
        if (!Uri.TryCreate(url, UriKind.Absolute, out var uri)) return "";
        var parts = uri.Host.Split('.');
        var root = parts.Length >= 2 ? $"{parts[^2]}.{parts[^1]}" : uri.Host;
        return $"{uri.Scheme}://{root}";
    }

    internal static string NormalizeTitle(string title)
    {
        var normalized = new string(title.Select(c => char.IsLetterOrDigit(c) ? c : ' ').ToArray());
        return string.Join(' ', normalized.Split(' ', StringSplitOptions.RemoveEmptyEntries)).ToLowerInvariant();
    }

    internal static async Task<byte[]?> GetChapterPageAsync(string path, int page)
    {
        var ext = Path.GetExtension(path).ToLowerInvariant();
        return ext is ".cbz" or ".zip"
            ? GetZipPage(path, page)
            : await GetDirPageAsync(path, page);
    }

    // ── Private helpers ───────────────────────────────────────────────────────

    static byte[]? GetZipPage(string path, int page)
    {
        if (!File.Exists(path)) return null;
        using var zip = ZipFile.OpenRead(path);
        var images = zip.Entries
            .Where(e => IsImage(e.Name))
            .OrderBy(e => e.Name)
            .ToList();
        if (page >= images.Count) return null;
        using var stream = images[page].Open();
        using var ms = new MemoryStream();
        stream.CopyTo(ms);
        return ms.ToArray();
    }

    static async Task<byte[]?> GetDirPageAsync(string dirPath, int page)
    {
        if (!Directory.Exists(dirPath)) return null;
        var files = Directory.GetFiles(dirPath)
            .Where(f => IsImage(Path.GetFileName(f)))
            .OrderBy(f => f)
            .ToList();
        if (page >= files.Count) return null;
        return await File.ReadAllBytesAsync(files[page]);
    }

    static List<PageUrlEntry> ParsePageUrls(JsonElement[] elements) =>
        elements.Select(el =>
        {
            if (el.ValueKind == JsonValueKind.String)
                return new PageUrlEntry(el.GetString()!, null);
            var url = el.GetProperty("url").GetString()!;
            var referer = el.TryGetProperty("referer", out var r) ? r.GetString() : null;
            return new PageUrlEntry(url, referer);
        }).ToList();

    static string ExtFromPath(string path) =>
        Path.GetExtension(path).ToLowerInvariant() switch
        {
            ".webp" => "image/webp",
            ".png"  => "image/png",
            _       => "image/jpeg",
        };
}
