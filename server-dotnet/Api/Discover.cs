using System.Security.Claims;
using System.Text.Json.Serialization;
using ArrghServer.Data;
using ArrghServer.Data.Models;
using ArrghServer.Services;
using Microsoft.EntityFrameworkCore;

namespace ArrghServer.Api;

public static class Discover
{
    public static RouteGroupBuilder MapDiscoverRoutes(this RouteGroupBuilder group)
    {
        group.MapGet("/", Search).RequireAuthorization();
        group.MapGet("/trending", Trending).RequireAuthorization();
        group.MapPost("/add", AddManga).RequireAuthorization();
        return group;
    }

    // ── GET /api/discover ─────────────────────────────────────────────────────

    static async Task<IResult> Search(
        string q, ClaimsPrincipal principal,
        AppDbContext db, MangaUpdatesService mu, IHttpClientFactory httpFactory, IConfiguration config)
    {
        var userId = principal.FindFirstValue(ClaimTypes.NameIdentifier)!;
        var allowExplicit = principal.FindFirstValue("allow_explicit") == "true";

        List<MuSeries> series;
        try { series = await mu.SearchAsync(q); }
        catch { return Results.StatusCode(502); }

        if (series.Count > 0)
        {
            var results = new List<DiscoverResult>(series.Count);
            foreach (var s in series)
            {
                var muId = s.SeriesId.ToString();
                var (inLibrary, libraryId) = await CheckInLibraryAsync(db, userId, muId);
                var result = MuToDiscoverResult(s, inLibrary, libraryId);
                await EnrichCoverAsync(db, result);

                if (s.CoverUrl is not null)
                    _ = SeedAndCacheCoverAsync(db, httpFactory, s.Title, s.CoverUrl,
                        config["DownloadDir"] ?? "./downloads", "mangaupdates", muId);

                results.Add(result);
            }
            return Results.Ok(results);
        }

        // MU returned nothing — E-Hentai fallback (explicit users only)
        // E-Hentai client not yet ported; return empty for now.
        return Results.Ok(Array.Empty<DiscoverResult>());
    }

    // ── GET /api/discover/trending ────────────────────────────────────────────

    static async Task<IResult> Trending(
        ClaimsPrincipal principal,
        AppDbContext db, MangaUpdatesService mu, TrendingCacheService cache, IHttpClientFactory httpFactory, IConfiguration config)
    {
        var userId = principal.FindFirstValue(ClaimTypes.NameIdentifier)!;

        var series = cache.GetFresh();
        if (series is null)
        {
            List<MuSeries> fresh;
            try { fresh = await mu.LatestReleasesAsync(); }
            catch
            {
                series = cache.GetStale();
                if (series is null) return Results.StatusCode(502);
                fresh = series;
            }

            if (fresh.Count > 0)
            {
                cache.Set(fresh);
                series = fresh;
            }
            else
            {
                series = cache.GetStale() ?? [];
            }
        }

        var results = new List<DiscoverResult>(series.Count);
        foreach (var s in series)
        {
            var muId = s.SeriesId.ToString();
            var (inLibrary, libraryId) = await CheckInLibraryAsync(db, userId, muId);
            var result = MuToDiscoverResult(s, inLibrary, libraryId);
            await EnrichCoverAsync(db, result);
            results.Add(result);
        }

        return Results.Ok(results);
    }

    // ── POST /api/discover/add ────────────────────────────────────────────────

    static async Task<IResult> AddManga(
        AddMangaBody body, ClaimsPrincipal principal,
        AppDbContext db, MangaUpdatesService mu, IHttpClientFactory httpFactory,
        IConfiguration config, IServiceScopeFactory scopeFactory)
    {
        var userId = principal.FindFirstValue(ClaimTypes.NameIdentifier)!;
        var now = DateTime.UtcNow;

        // Resolve meta-cover path → CDN URL
        string? resolvedCoverUrl = body.CoverUrl;
        if (resolvedCoverUrl?.StartsWith("/api/media/meta-cover?key=") == true)
        {
            var encodedKey = resolvedCoverUrl["/api/media/meta-cover?key=".Length..];
            var key = Uri.UnescapeDataString(encodedKey);
            resolvedCoverUrl = await db.TitleMeta
                .Where(m => m.TitleKey == key)
                .Select(m => m.CoverCdnUrl)
                .FirstOrDefaultAsync();
        }

        // Check existing (same mangaupdates_id)
        var existing = await db.Titles
            .Where(t => t.MangaupdatesId == body.MangaupdatesId)
            .Select(t => t.Id)
            .FirstOrDefaultAsync();

        string titleId;
        if (existing is not null)
        {
            titleId = existing;
            await db.Titles.Where(t => t.Id == titleId).ExecuteUpdateAsync(s => s
                .SetProperty(t => t.Description, t => t.Description ?? body.Description)
                .SetProperty(t => t.Author,       t => t.Author ?? body.Author)
                .SetProperty(t => t.Year,         t => t.Year ?? body.Year)
                .SetProperty(t => t.Tags,         t => t.Tags ?? body.Tags));
        }
        else
        {
            titleId = Guid.NewGuid().ToString();
            var isExplicit = IsHentaiTag(body.Tags) ||
                body.Tags?.Split(',').Any(t => t.Trim().Equals("adult", StringComparison.OrdinalIgnoreCase)) == true;
            var cleanTitle = StripSearchQualifier(body.Title) ?? body.Title;

            db.Titles.Add(new Title
            {
                Id = titleId,
                MangaupdatesId = body.MangaupdatesId,
                TitleName = cleanTitle,
                Description = body.Description,
                CoverUrl = resolvedCoverUrl,
                Status = body.Status,
                Author = body.Author,
                Year = body.Year,
                Tags = body.Tags,
                SyncStatus = "syncing",
                ContentType = body.ContentType,
                IsExplicit = isExplicit,
                CreatedAt = now,
                UpdatedAt = now,
            });
            await db.SaveChangesAsync();
        }

        // Subscribe user (idempotent)
        if (!await db.UserTitles.AnyAsync(ut => ut.UserId == userId && ut.TitleId == titleId))
        {
            db.UserTitles.Add(new UserTitle { UserId = userId, TitleId = titleId, AddedAt = now });
            await db.SaveChangesAsync();
        }

        // Fire-and-forget: cover download + MU aliases + source stub
        var capturedTitleId = titleId;
        var capturedMuId = body.MangaupdatesId;
        var capturedTitle = body.Title;
        var capturedCover = resolvedCoverUrl;
        var downloadDir = config["DownloadDir"] ?? "./downloads";

        _ = Task.Run(async () =>
        {
            using var scope = scopeFactory.CreateScope();
            var bgDb = scope.ServiceProvider.GetRequiredService<AppDbContext>();
            var bgMu = scope.ServiceProvider.GetRequiredService<MangaUpdatesService>();

            await bgDb.SyncLogs.Where(l => l.TitleId == capturedTitleId).ExecuteDeleteAsync();

            // 1. Download cover
            if (capturedCover is not null)
            {
                await AppendSyncLogAsync(bgDb, capturedTitleId, "Downloading cover…");
                await DownloadCoverAsync(bgDb, httpFactory, capturedTitleId, capturedCover, downloadDir);
            }

            // 2. Fetch MU aliases
            await AppendSyncLogAsync(bgDb, capturedTitleId, "Fetching metadata from MangaUpdates…");
            if (ulong.TryParse(capturedMuId, out var muIdNum))
            {
                try
                {
                    var series = await bgMu.SeriesDetailAsync(muIdNum);
                    if (series is not null && series.AssociatedNames.Count > 0)
                    {
                        await bgDb.TitleAliases.Where(a => a.TitleId == capturedTitleId).ExecuteDeleteAsync();
                        foreach (var alias in series.AssociatedNames)
                            bgDb.TitleAliases.Add(new TitleAlias
                            {
                                Id = Guid.NewGuid().ToString(),
                                TitleId = capturedTitleId,
                                Alias = alias,
                            });
                        await bgDb.SaveChangesAsync();
                        await AppendSyncLogAsync(bgDb, capturedTitleId,
                            $"Loaded {series.AssociatedNames.Count} alternate title(s)");
                    }
                }
                catch { /* best-effort */ }
            }

            // 3. Source matching (stub — source registry not yet ported, ADR 0030)
            await AppendSyncLogAsync(bgDb, capturedTitleId, "No sources available for this title type");
            // When ported: iterate external_sources, call plugin-host search, match by title_matches

            await bgDb.Titles.Where(t => t.Id == capturedTitleId)
                .ExecuteUpdateAsync(s => s.SetProperty(t => t.SyncStatus, "ready"));
            await AppendSyncLogAsync(bgDb, capturedTitleId, "Sync complete");
        });

        var item = await Titles.FetchTitleAsync(db, titleId, userId);
        return Results.Ok(item);
    }

    // ── Shared helpers ────────────────────────────────────────────────────────

    static async Task<(bool InLibrary, string? LibraryId)> CheckInLibraryAsync(
        AppDbContext db, string userId, string muId)
    {
        var id = await db.Titles
            .Where(t => t.MangaupdatesId == muId && t.UserTitles.Any(ut => ut.UserId == userId))
            .Select(t => (string?)t.Id)
            .FirstOrDefaultAsync();
        return (id is not null, id);
    }

    static async Task EnrichCoverAsync(AppDbContext db, DiscoverResult result)
    {
        var key = Media.NormalizeTitle(result.Title);
        var meta = await db.TitleMeta
            .Where(m => m.TitleKey == key)
            .Select(m => new { m.CoverLocalPath, m.CoverCdnUrl })
            .FirstOrDefaultAsync();
        if (meta is null) return;

        if (meta.CoverLocalPath is not null)
            result.CoverUrl = $"/api/media/meta-cover?key={Uri.EscapeDataString(key)}";
        else if (result.CoverUrl is null)
            result.CoverUrl = meta.CoverCdnUrl;
    }

    static async Task SeedAndCacheCoverAsync(
        AppDbContext db, IHttpClientFactory httpFactory,
        string title, string cdnUrl, string downloadDir, string source, string sourceId)
    {
        try
        {
            var key = Media.NormalizeTitle(title);

            // RAW SQL — intentional: ON CONFLICT DO UPDATE SET cover_cdn_url = COALESCE(...)
            // preserves existing CDN URL; no EF equivalent for conditional conflict update.
            await db.Database.ExecuteSqlRawAsync(
                @"INSERT INTO title_meta (title_key, cover_cdn_url, fetched_at, source, source_id)
                  VALUES ({0}, {1}, {2}, {3}, {4})
                  ON CONFLICT(title_key) DO UPDATE SET
                    cover_cdn_url = COALESCE(title_meta.cover_cdn_url, excluded.cover_cdn_url),
                    fetched_at    = excluded.fetched_at",
                key, cdnUrl, DateTime.UtcNow, source, sourceId);

            var http = httpFactory.CreateClient();
            var req = new HttpRequestMessage(HttpMethod.Get, cdnUrl);
            req.Headers.Add("User-Agent", "Mozilla/5.0");
            var resp = await http.SendAsync(req);
            resp.EnsureSuccessStatusCode();
            var bytes = await resp.Content.ReadAsByteArrayAsync();

            var ext = (cdnUrl.Split('?')[0].Split('.').LastOrDefault() ?? "jpg").ToLowerInvariant();
            var safeName = new string(key.Select(c => char.IsLetterOrDigit(c) ? c : '_').ToArray());
            var path = Path.Combine(downloadDir, "_meta", $"{safeName}.{ext}");
            Directory.CreateDirectory(Path.GetDirectoryName(path)!);
            await File.WriteAllBytesAsync(path, bytes);

            await db.Database.ExecuteSqlRawAsync(
                "UPDATE title_meta SET cover_local_path = {0} WHERE title_key = {1}", path, key);
        }
        catch { /* best-effort */ }
    }

    static async Task DownloadCoverAsync(
        AppDbContext db, IHttpClientFactory httpFactory,
        string titleId, string cdnUrl, string downloadDir)
    {
        try
        {
            var http = httpFactory.CreateClient();
            var req = new HttpRequestMessage(HttpMethod.Get, cdnUrl);
            req.Headers.Add("User-Agent", "Mozilla/5.0");
            var resp = await http.SendAsync(req);
            resp.EnsureSuccessStatusCode();
            var bytes = await resp.Content.ReadAsByteArrayAsync();

            var ext = (cdnUrl.Split('?')[0].Split('.').LastOrDefault() ?? "jpg").ToLowerInvariant();
            var path = Path.Combine(downloadDir, "_covers", $"{titleId}.{ext}");
            Directory.CreateDirectory(Path.GetDirectoryName(path)!);
            await File.WriteAllBytesAsync(path, bytes);

            await db.Titles.Where(t => t.Id == titleId)
                .ExecuteUpdateAsync(s => s.SetProperty(t => t.CoverUrl, path));
        }
        catch { /* best-effort */ }
    }

    static async Task AppendSyncLogAsync(AppDbContext db, string titleId, string message)
    {
        try
        {
            db.SyncLogs.Add(new SyncLog
            {
                Id = Guid.NewGuid().ToString(),
                TitleId = titleId,
                Message = message,
                CreatedAt = DateTime.UtcNow,
            });
            await db.SaveChangesAsync();
        }
        catch { }
    }

    static DiscoverResult MuToDiscoverResult(MuSeries s, bool inLibrary, string? libraryId) => new()
    {
        MangaupdatesId = s.SeriesId.ToString(),
        Title = s.Title,
        Description = s.Description,
        CoverUrl = s.CoverUrl,
        Status = s.Status,
        Author = s.Author,
        Year = s.Year,
        Tags = s.Tags,
        ContentType = s.ContentType,
        InLibrary = inLibrary,
        LibraryId = libraryId,
        Source = "mangaupdates",
    };

    // ── Pure helpers — unit-testable ──────────────────────────────────────────

    internal static bool TitleMatches(string a, string b)
    {
        if (a == b) return true;
        int maxLen = Math.Max(a.Length, b.Length);
        if (maxLen == 0) return true;
        return Levenshtein(a, b) * 5 <= maxLen;
    }

    internal static int Levenshtein(string a, string b)
    {
        int m = a.Length, n = b.Length;
        var row = Enumerable.Range(0, n + 1).ToArray();
        for (int i = 1; i <= m; i++)
        {
            int prev = row[0];
            row[0] = i;
            for (int j = 1; j <= n; j++)
            {
                int old = row[j];
                row[j] = a[i - 1] == b[j - 1] ? prev : 1 + Math.Min(prev, Math.Min(row[j], row[j - 1]));
                prev = old;
            }
        }
        return row[n];
    }

    internal static string? StripSearchQualifier(string s)
    {
        s = s.Trim();
        var openIdx = s.LastIndexOf('(');
        if (openIdx < 0) return null;
        var suffix = s[openIdx..];
        if (!suffix.EndsWith(')') || suffix.Length < 3 || suffix.Length > 20) return null;
        var stripped = s[..openIdx].TrimEnd();
        return stripped.Length == 0 || stripped == s ? null : stripped;
    }

    internal static List<string> SearchCandidates(IList<string> names)
    {
        var seen = new HashSet<string>(StringComparer.Ordinal);
        var out2 = new List<string>();
        foreach (var name in names)
        {
            var stripped = StripSearchQualifier(name);
            if (stripped is not null && seen.Add(stripped)) out2.Add(stripped);
            if (seen.Add(name)) out2.Add(name);
        }
        return out2;
    }

    internal static List<string> KnownNorms(IList<string> names)
    {
        var seen = new HashSet<string>(StringComparer.Ordinal);
        var out2 = new List<string>();
        void Push(string s) { if (seen.Add(s)) out2.Add(s); }
        foreach (var name in names)
        {
            Push(Media.NormalizeTitle(name));
            var stripped = StripSearchQualifier(name);
            if (stripped is not null) Push(Media.NormalizeTitle(stripped));
        }
        return out2;
    }

    internal static bool IsHentaiTag(string? tags) =>
        tags?.Split(',').Any(t => t.Trim().Equals("hentai", StringComparison.OrdinalIgnoreCase)) ?? false;
}

// ── DTOs ──────────────────────────────────────────────────────────────────────

public class DiscoverResult
{
    [JsonPropertyName("mangaupdates_id")] public string MangaupdatesId { get; set; } = null!;
    [JsonPropertyName("title")]           public string Title { get; set; } = null!;
    [JsonPropertyName("description")]     public string? Description { get; set; }
    [JsonPropertyName("cover_url")]       public string? CoverUrl { get; set; }
    [JsonPropertyName("status")]          public string Status { get; set; } = null!;
    [JsonPropertyName("author")]          public string? Author { get; set; }
    [JsonPropertyName("year")]            public int? Year { get; set; }
    [JsonPropertyName("tags")]            public string? Tags { get; set; }
    [JsonPropertyName("content_type")]    public string ContentType { get; set; } = null!;
    [JsonPropertyName("in_library")]      public bool InLibrary { get; set; }
    [JsonPropertyName("library_id")]      public string? LibraryId { get; set; }
    [JsonPropertyName("source")]          public string Source { get; set; } = null!;
}

public class AddMangaBody
{
    [JsonPropertyName("mangaupdates_id")] public string MangaupdatesId { get; set; } = null!;
    [JsonPropertyName("title")]           public string Title { get; set; } = null!;
    [JsonPropertyName("description")]     public string? Description { get; set; }
    [JsonPropertyName("cover_url")]       public string? CoverUrl { get; set; }
    [JsonPropertyName("status")]          public string Status { get; set; } = "unknown";
    [JsonPropertyName("author")]          public string? Author { get; set; }
    [JsonPropertyName("year")]            public int? Year { get; set; }
    [JsonPropertyName("tags")]            public string? Tags { get; set; }
    [JsonPropertyName("content_type")]    public string ContentType { get; set; } = "manga";
}
