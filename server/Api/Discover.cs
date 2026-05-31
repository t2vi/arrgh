using System.Security.Claims;
using System.Text.Json.Serialization;
using ArrghServer.Data;
using ArrghServer.Data.Models;
using ArrghServer.Services;
using Microsoft.EntityFrameworkCore;

namespace ArrghServer.Api;

// Authority order: MU → AniList → MangaDex → NovelUpdates → EHentai

public static class Discover
{
    public static RouteGroupBuilder MapDiscoverRoutes(this RouteGroupBuilder group)
    {
        group.MapGet("/", Search).RequireAuthorization()
            .WithSummary("Search titles across all metadata authorities (MU, AniList, MangaDex, NovelUpdates, E-Hentai)")
            .WithDescription("Fan-out parallel search. Results deduped by designated authority per content_type (manga→MU, manhwa→AniList, manhua→MangaDex, novel→NovelUpdates, hentai→E-Hentai). E-Hentai gated on allow_explicit claim. Returns 502 only if all authorities fail.");
        group.MapGet("/trending/manga", TrendingManga).RequireAuthorization()
            .WithSummary("Trending manga from MangaUpdates latest releases");
        group.MapGet("/trending/manhwa", TrendingManhwa).RequireAuthorization()
            .WithSummary("Trending manhwa from AniList (KR, non-adult)");
        group.MapGet("/trending/manhua", TrendingManhua).RequireAuthorization()
            .WithSummary("Trending manhua from AniList (CN, non-adult)");
        group.MapGet("/trending/adult-manhwa", TrendingAdultManhwa).RequireAuthorization()
            .WithSummary("Trending adult manhwa from AniList (KR, isAdult). Requires allow_explicit.");
        group.MapPost("/add", AddManga).RequireAuthorization()
            .WithSummary("Add a title to the library")
            .WithDescription("Accepts source+source_id (any authority) or legacy mangaupdates_id. Deduplicates by metadata_source+metadata_source_id. Backward compatible with old mangaupdates_id-only clients.");
        return group;
    }

    // ── GET /api/discover ─────────────────────────────────────────────────────

    static async Task<IResult> Search(
        string q, ClaimsPrincipal principal,
        AppDbContext db, MangaUpdatesService mu,
        AniListService aniList, MangaDexMetaService mdMeta,
        NovelUpdatesService novelUpdates, WuxiaWorldMetaService wuxiaWorld, EHentaiService eHentai,
        IHttpClientFactory httpFactory, IConfiguration config)
    {
        var userId = principal.FindFirstValue(ClaimTypes.NameIdentifier)!;
        var allowExplicit = principal.FindFirstValue("allow_explicit") == "true";
        var downloadDir = config["DownloadDir"] ?? "./downloads";

        // Fan-out HTTP calls in parallel — NO DB access inside tasks (EF DbContext is not thread-safe)
        // ADR 0031: MU is manga-authority only — filter to manga/one-shot before dedup
        var muTask = SafeSearch(() => mu.SearchAsync(q)
            .ContinueWith(t => (IEnumerable<DiscoverResult>)FilterMuScope(
                t.Result.Select(s => MuToDiscoverResult(s, false, null))).ToList(),
                TaskContinuationOptions.OnlyOnRanToCompletion));
        var alTask = SafeSearch(() => aniList.SearchAsync(q, allowExplicit)
            .ContinueWith(t => (IEnumerable<DiscoverResult>)t.Result.Select(s => new DiscoverResult
            {
                MangaupdatesId = s.SourceId, Title = s.Title, Description = s.Description,
                CoverUrl = s.CoverUrl, Status = s.Status, Author = s.Author, Year = s.Year,
                ContentType = s.ContentType, Source = "anilist",
                IsExplicit = s.IsAdult,
            }).ToList(), TaskContinuationOptions.OnlyOnRanToCompletion));
        var mdTask = SafeSearch(() => mdMeta.SearchAsync(q)
            .ContinueWith(t => (IEnumerable<DiscoverResult>)t.Result.Select(s => new DiscoverResult
            {
                MangaupdatesId = s.SourceId, Title = s.Title, Description = s.Description,
                CoverUrl = s.CoverUrl, Status = s.Status, Author = s.Author, Year = s.Year,
                ContentType = s.ContentType, Source = "mangadex",
            }).ToList(), TaskContinuationOptions.OnlyOnRanToCompletion));
        var nuTask = SafeSearch(() => novelUpdates.SearchAsync(q)
            .ContinueWith(t => (IEnumerable<DiscoverResult>)t.Result.Select(s => new DiscoverResult
            {
                MangaupdatesId = s.SourceId, Title = s.Title, CoverUrl = s.CoverUrl,
                Status = s.Status, ContentType = "novel", Source = "novelupdates",
            }).ToList(), TaskContinuationOptions.OnlyOnRanToCompletion));
        // WuxiaWorld: parallel novel authority, no CF protection. Deduped against NU (NU wins).
        var wwTask = SafeSearch(() => wuxiaWorld.SearchAsync(q)
            .ContinueWith(t => (IEnumerable<DiscoverResult>)t.Result.Select(s => new DiscoverResult
            {
                MangaupdatesId = s.SourceId, Title = s.Title, CoverUrl = s.CoverUrl,
                Status = s.Status, Author = s.Author, ContentType = "novel", Source = "wuxiaworld",
            }).ToList(), TaskContinuationOptions.OnlyOnRanToCompletion));
        var ehTask = allowExplicit
            ? SafeSearch(() => eHentai.SearchAsync(q)
                .ContinueWith(t => (IEnumerable<DiscoverResult>)t.Result.Select(s => new DiscoverResult
                {
                    MangaupdatesId = s.SourceId, Title = s.Title, CoverUrl = s.CoverUrl,
                    Tags = s.Tags is not null ? string.Join(",", s.Tags) : null,
                    ContentType = "hentai", Status = "complete", Source = "ehentai", IsExplicit = true,
                }).ToList(), TaskContinuationOptions.OnlyOnRanToCompletion))
            : Task.FromResult<IEnumerable<DiscoverResult>?>(null);

        await Task.WhenAll(muTask, alTask, mdTask, nuTask, wwTask, ehTask);

        // Collect all results from succeeded tasks
        // 502 only if ALL queried authorities failed (threw); empty-but-succeeded → 200 []
        var raw = new List<DiscoverResult>();
        var anySucceeded = false;
        foreach (var t in new[] { muTask, alTask, mdTask, nuTask, wwTask, ehTask })
        {
            if (t.Result is not null) anySucceeded = true;
            if (t.Result is { } r) raw.AddRange(r);
        }
        if (!anySucceeded) return Results.StatusCode(502);

        // Seed cover cache for MU results (fire-and-forget, not DB reads)
        var muResults = muTask.Result;
        if (muResults is not null)
            foreach (var s in (await mu.SearchAsync(q).ContinueWith(t => t.IsCompletedSuccessfully ? t.Result : new List<MuSeries>())))
                if (s.CoverUrl is not null)
                    _ = SeedAndCacheCoverAsync(db, httpFactory, s.Title, s.CoverUrl, downloadDir, "mangaupdates", s.SeriesId.ToString());

        // Merge + dedup + sort by authority
        var merged = MergeFanOut(raw);

        // Sequential library check on merged results (DB access — single-threaded)
        foreach (var result in merged)
        {
            var (inLibrary, libraryId) = await CheckInLibraryAsync(db, userId,
                result.Source, result.MangaupdatesId, result.Title, result.ContentType);
            result.InLibrary = inLibrary;
            result.LibraryId = libraryId;
        }

        // Enrich covers from local cache
        foreach (var result in merged)
            await EnrichCoverAsync(db, result);

        return Results.Ok(merged);
    }

    // ── Source matching (ADR 0013 + ADR 0016) ────────────────────────────────
    // After add: query external_sources by content_type, call plugin-host /search,
    // match by normalized title, create title_sources + sync chapters via ChapterSync.

    static async Task MatchSourcesAsync(
        AppDbContext db, IHttpClientFactory httpFactory,
        string titleId, string? titleName, string? contentType, string pluginHostUrl)
    {
        if (string.IsNullOrWhiteSpace(contentType) || string.IsNullOrWhiteSpace(titleName)) return;

        var sources = await db.ExternalSources
            .Where(s => s.Enabled && s.SourceKey != null &&
                        EF.Functions.Like(s.ContentTypes, $"%{contentType}%"))
            .OrderBy(s => s.Priority)
            .ToListAsync();

        if (sources.Count == 0) return;

        var http = httpFactory.CreateClient();
        var normTarget = Media.NormalizeTitle(titleName);

        var aliases = await db.TitleAliases
            .Where(a => a.TitleId == titleId)
            .Select(a => a.Alias)
            .ToListAsync();
        var normAliases = aliases.Select(Media.NormalizeTitle).ToList();

        foreach (var source in sources)
        {
            try
            {
                var searchUrl = $"{pluginHostUrl.TrimEnd('/')}/{source.SourceKey}/search?q={Uri.EscapeDataString(titleName)}";
                var resp = await http.GetAsync(searchUrl);
                if (!resp.IsSuccessStatusCode)
                {
                    await AppendSyncLogAsync(db, titleId, $"Source {source.SourceKey} unavailable ({(int)resp.StatusCode})");
                    continue;
                }

                var results = await resp.Content.ReadFromJsonAsync<List<PluginSearchResult>>(
                    new System.Text.Json.JsonSerializerOptions { PropertyNameCaseInsensitive = true });
                if (results is null || results.Count == 0)
                {
                    await AppendSyncLogAsync(db, titleId, $"No results from {source.SourceKey}");
                    await AppendSyncWarningAsync(db, titleId, source.SourceKey!, $"No results from {source.SourceKey}");
                    continue;
                }

                var match = results.FirstOrDefault(r => {
                    var normResult = Media.NormalizeTitle(r.Title ?? "");
                    return TitleMatches(normResult, normTarget) ||
                           normAliases.Any(na => TitleMatches(normResult, na));
                });
                if (match is null)
                {
                    await AppendSyncLogAsync(db, titleId, $"No title match on {source.SourceKey} (searched \"{titleName}\")");
                    await AppendSyncWarningAsync(db, titleId, source.SourceKey!, $"No title match on {source.SourceKey}");
                    continue;
                }

                var sourceId = match.Id;
                if (string.IsNullOrEmpty(sourceId)) continue;

                await AppendSyncLogAsync(db, titleId, $"Matched {source.SourceKey}:{sourceId} — syncing chapters…");

                // Upsert title_source (idempotent)
                if (!await db.TitleSources.AnyAsync(ts => ts.TitleId == titleId && ts.Source == source.SourceKey))
                {
                    db.TitleSources.Add(new Data.Models.TitleSource
                    {
                        Id = Guid.NewGuid().ToString(), TitleId = titleId,
                        Source = source.SourceKey!, SourceId = sourceId, DiscoveredAt = DateTime.UtcNow,
                    });
                    await db.SaveChangesAsync();
                }

                // Sync chapters (creates chapter rows + chapter_sources)
                var chapterCount = await ChapterSync.SyncFromSourceAsync(db, http, pluginHostUrl, titleId, contentType, source.SourceKey!, sourceId);
                await AppendSyncLogAsync(db, titleId, $"Synced {chapterCount} chapter(s) from {source.SourceKey}");
            }
            catch (Exception ex)
            {
                await AppendSyncLogAsync(db, titleId, $"Error from {source.SourceKey}: {ex.Message}");
                await AppendSyncWarningAsync(db, titleId, source.SourceKey!, ex.Message);
            }
        }
    }

    record PluginSearchResult(
        [property: System.Text.Json.Serialization.JsonPropertyName("id")]    string? Id,
        [property: System.Text.Json.Serialization.JsonPropertyName("title")] string? Title);

    static Task<IEnumerable<DiscoverResult>?> SafeSearch(Func<Task<IEnumerable<DiscoverResult>>> fn)
    {
        try
        {
            return fn().ContinueWith(t =>
                t.IsCompletedSuccessfully ? (IEnumerable<DiscoverResult>?)t.Result : null);
        }
        catch
        {
            return Task.FromResult<IEnumerable<DiscoverResult>?>(null);
        }
    }

    // ── GET /api/discover/trending/{lane} ────────────────────────────────────

    static async Task<IResult> TrendingManga(
        ClaimsPrincipal principal,
        AppDbContext db, MangaUpdatesService mu, TrendingCacheService cache)
    {
        var userId = principal.FindFirstValue(ClaimTypes.NameIdentifier)!;
        const string lane = "manga";

        var cached = cache.GetFresh(lane);
        if (cached is null)
        {
            try
            {
                var fresh = await mu.LatestReleasesAsync();
                cached = fresh.Select(s => MuToDiscoverResult(s, false, null)).ToList();
                if (cached.Count > 0) cache.Set(lane, cached);
                else cached = cache.GetStale(lane) ?? [];
            }
            catch
            {
                cached = cache.GetStale(lane) ?? [];
            }
        }

        var results = await EnrichAndCheckLibraryAsync(db, userId, cached.Take(6).ToList());
        return Results.Ok(results);
    }

    static async Task<IResult> TrendingManhwa(
        ClaimsPrincipal principal,
        AppDbContext db, AniListService aniList, TrendingCacheService cache)
    {
        var userId = principal.FindFirstValue(ClaimTypes.NameIdentifier)!;
        const string lane = "manhwa";

        var cached = cache.GetFresh(lane);
        if (cached is null)
        {
            try
            {
                var fresh = await aniList.TrendingAsync("KR", isAdult: false);
                cached = fresh.Select(AniListToDiscoverResult).ToList();
                if (cached.Count > 0) cache.Set(lane, cached);
                else cached = cache.GetStale(lane) ?? [];
            }
            catch
            {
                cached = cache.GetStale(lane) ?? [];
            }
        }

        var results = await EnrichAndCheckLibraryAsync(db, userId, cached.Take(6).ToList());
        return Results.Ok(results);
    }

    static async Task<IResult> TrendingManhua(
        ClaimsPrincipal principal,
        AppDbContext db, AniListService aniList, TrendingCacheService cache)
    {
        var userId = principal.FindFirstValue(ClaimTypes.NameIdentifier)!;
        const string lane = "manhua";

        var cached = cache.GetFresh(lane);
        if (cached is null)
        {
            try
            {
                var fresh = await aniList.TrendingAsync("CN", isAdult: false);
                cached = fresh.Select(AniListToDiscoverResult).ToList();
                if (cached.Count > 0) cache.Set(lane, cached);
                else cached = cache.GetStale(lane) ?? [];
            }
            catch
            {
                cached = cache.GetStale(lane) ?? [];
            }
        }

        var results = await EnrichAndCheckLibraryAsync(db, userId, cached.Take(6).ToList());
        return Results.Ok(results);
    }

    static async Task<IResult> TrendingAdultManhwa(
        ClaimsPrincipal principal,
        AppDbContext db, AniListService aniList, TrendingCacheService cache)
    {
        var allowExplicit = principal.FindFirstValue("allow_explicit") == "true";
        if (!allowExplicit) return Results.Forbid();

        var userId = principal.FindFirstValue(ClaimTypes.NameIdentifier)!;
        const string lane = "adult-manhwa";

        var cached = cache.GetFresh(lane);
        if (cached is null)
        {
            try
            {
                var fresh = await aniList.TrendingAsync("KR", isAdult: true);
                cached = fresh.Select(AniListToDiscoverResult).ToList();
                if (cached.Count > 0) cache.Set(lane, cached);
                else cached = cache.GetStale(lane) ?? [];
            }
            catch
            {
                cached = cache.GetStale(lane) ?? [];
            }
        }

        var results = await EnrichAndCheckLibraryAsync(db, userId, cached.Take(6).ToList());
        return Results.Ok(results);
    }

    static DiscoverResult AniListToDiscoverResult(AniListSeries s) => new()
    {
        MangaupdatesId = s.SourceId, Title = s.Title, Description = s.Description,
        CoverUrl = s.CoverUrl, Status = s.Status, Author = s.Author, Year = s.Year,
        ContentType = s.ContentType, Source = "anilist", IsExplicit = s.IsAdult,
    };

    static async Task<List<DiscoverResult>> EnrichAndCheckLibraryAsync(
        AppDbContext db, string userId, List<DiscoverResult> items)
    {
        foreach (var r in items)
        {
            var (inLibrary, libraryId) = await CheckInLibraryAsync(db, userId, r.Source, r.MangaupdatesId, r.Title, r.ContentType);
            r.InLibrary = inLibrary;
            r.LibraryId = libraryId;
            await EnrichCoverAsync(db, r);
        }
        return items;
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

        // Resolve metadata source: explicit source wins, fall back to mangaupdates_id
        var metaSource = body.Source ?? (body.MangaupdatesId is not null ? "mangaupdates" : null);
        var metaSourceId = body.SourceId ?? body.MangaupdatesId;

        // Check existing: by source+source_id first, then legacy mangaupdates_id
        string? existing = null;
        if (metaSource is not null && metaSourceId is not null)
            existing = await db.Titles
                .Where(t => t.MetadataSource == metaSource && t.MetadataSourceId == metaSourceId)
                .Select(t => (string?)t.Id)
                .FirstOrDefaultAsync();
        if (existing is null && body.MangaupdatesId is not null)
            existing = await db.Titles
                .Where(t => t.MangaupdatesId == body.MangaupdatesId)
                .Select(t => (string?)t.Id)
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
            var isExplicit = body.IsExplicit == true || body.ContentType == "hentai" || IsHentaiTag(body.Tags) ||
                body.Tags?.Split(',').Any(t => t.Trim().Equals("adult", StringComparison.OrdinalIgnoreCase)) == true;
            var cleanTitle = StripSearchQualifier(body.Title) ?? body.Title;

            db.Titles.Add(new Title
            {
                Id = titleId,
                MangaupdatesId = body.MangaupdatesId,
                MetadataSource = metaSource,
                MetadataSourceId = metaSourceId,
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

        // Fire-and-forget: cover download + authority-routed metadata + source matching
        var capturedTitleId = titleId;
        var capturedMuId = body.MangaupdatesId;
        var capturedMetaSource = metaSource;
        var capturedMetaSourceId = metaSourceId;
        var capturedTitle = body.Title;
        var capturedCover = resolvedCoverUrl;
        var capturedContentType = body.ContentType;
        var downloadDir = config["DownloadDir"] ?? "./downloads";
        var pluginHostUrl = config["PluginHostUrl"] ?? "http://plugin-host:4000";

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

            // 2. Fetch aliases from the correct metadata authority (ADR 0031 routing switch)
            switch (capturedMetaSource)
            {
                case "mangaupdates":
                    await AppendSyncLogAsync(bgDb, capturedTitleId, "Fetching metadata from MangaUpdates…");
                    if (ulong.TryParse(capturedMetaSourceId ?? capturedMuId, out var muIdNum))
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
                    break;

                case "anilist":
                    await AppendSyncLogAsync(bgDb, capturedTitleId, "Fetching synonyms from AniList…");
                    if (!string.IsNullOrEmpty(capturedMetaSourceId))
                    {
                        try
                        {
                            var bgAniList = scope.ServiceProvider.GetRequiredService<AniListService>();
                            var synonyms = await bgAniList.GetSynonymsAsync(capturedMetaSourceId);
                            if (synonyms.Count > 0)
                            {
                                await bgDb.TitleAliases.Where(a => a.TitleId == capturedTitleId).ExecuteDeleteAsync();
                                foreach (var syn in synonyms)
                                    bgDb.TitleAliases.Add(new TitleAlias
                                    {
                                        Id = Guid.NewGuid().ToString(),
                                        TitleId = capturedTitleId,
                                        Alias = syn,
                                    });
                                await bgDb.SaveChangesAsync();
                                await AppendSyncLogAsync(bgDb, capturedTitleId,
                                    $"Loaded {synonyms.Count} synonym(s)");
                            }
                        }
                        catch { /* best-effort */ }
                    }
                    break;

                case "mangadex":
                case "novelupdates":
                case "ehentai":
                    // Aliases via these authorities are not yet implemented — title name is sufficient for source matching
                    await AppendSyncLogAsync(bgDb, capturedTitleId, $"Metadata source: {capturedMetaSource}");
                    break;

                default:
                    await AppendSyncLogAsync(bgDb, capturedTitleId, $"Metadata source: {capturedMetaSource ?? "none"}");
                    break;
            }

            // 3. Source matching: find plugin-host sources by content_type, match by title, seed chapter_sources
            await MatchSourcesAsync(bgDb, httpFactory, capturedTitleId, capturedTitle, capturedContentType, pluginHostUrl);

            await bgDb.Titles.Where(t => t.Id == capturedTitleId)
                .ExecuteUpdateAsync(s => s.SetProperty(t => t.SyncStatus, "ready"));
            await AppendSyncLogAsync(bgDb, capturedTitleId, "Sync complete");
        });

        var item = await Titles.FetchTitleAsync(db, titleId, userId);
        return Results.Ok(item);
    }

    // ── Shared helpers ────────────────────────────────────────────────────────

    static async Task<(bool InLibrary, string? LibraryId)> CheckInLibraryAsync(
        AppDbContext db, string userId,
        string? metaSource, string? muId,
        string? title = null, string? contentType = null)
    {
        // 1. By metadata source+id (most precise)
        if (metaSource is not null && muId is not null)
        {
            var id = await db.Titles
                .Where(t => t.MetadataSource == metaSource && t.MetadataSourceId == muId
                            && t.UserTitles.Any(ut => ut.UserId == userId))
                .Select(t => (string?)t.Id)
                .FirstOrDefaultAsync();
            if (id is not null) return (true, id);
        }

        // 2. Legacy: by mangaupdates_id
        if (muId is not null)
        {
            var id = await db.Titles
                .Where(t => t.MangaupdatesId == muId && t.UserTitles.Any(ut => ut.UserId == userId))
                .Select(t => (string?)t.Id)
                .FirstOrDefaultAsync();
            if (id is not null) return (true, id);
        }

        // 3. By normalized title + content_type (catches AniList/NU titles with no MU id)
        if (title is not null && contentType is not null)
        {
            var normTitle = Media.NormalizeTitle(title);
            var allMatching = await db.Titles
                .Where(t => t.ContentType == contentType && t.UserTitles.Any(ut => ut.UserId == userId))
                .Select(t => new { t.Id, Norm = t.TitleName })
                .ToListAsync();
            var match = allMatching.FirstOrDefault(t => Media.NormalizeTitle(t.Norm) == normTitle);
            if (match is not null) return (true, match.Id);
        }

        return (false, null);
    }

    // ── Authority helpers (unit-testable) ─────────────────────────────────────

    public static readonly IList<string> AuthorityOrder =
        ["mangaupdates", "anilist", "mangadex", "novelupdates", "wuxiaworld", "ehentai"];

    // ADR 0031: MangaUpdates is manga-authority only. Strip non-manga before dedup so MU novel/manhwa/manhua
    // results can't survive when the designated authority returns nothing.
    public static IEnumerable<DiscoverResult> FilterMuScope(IEnumerable<DiscoverResult> muResults) =>
        muResults.Where(r => r.ContentType?.ToLowerInvariant() is "manga" or "one-shot");

    public static string DesignatedAuthority(string contentType) => contentType.ToLowerInvariant() switch
    {
        "manhwa" => "anilist",
        "manhua" => "mangadex",
        "novel" or "web novel" or "light novel" => "novelupdates",
        "hentai" => "ehentai",
        _ => "mangaupdates",
    };

    public static List<DiscoverResult> Deduplicate(IList<DiscoverResult> results)
    {
        var groups = results
            .GroupBy(r => (Media.NormalizeTitle(r.Title), r.ContentType?.ToLowerInvariant() ?? ""))
            .ToList();

        var out2 = new List<DiscoverResult>(groups.Count);
        foreach (var g in groups)
        {
            if (g.Count() == 1) { out2.Add(g.First()); continue; }
            var contentType = g.Key.Item2;
            var designated = DesignatedAuthority(contentType);
            var winner = g.FirstOrDefault(r => r.Source == designated) ?? g.First();
            out2.Add(winner);
        }
        return out2;
    }

    public static List<DiscoverResult> MergeFanOut(IList<DiscoverResult> results)
    {
        var deduped = Deduplicate(results);
        return deduped
            .OrderBy(r => {
                var idx = AuthorityOrder.IndexOf(r.Source);
                return idx < 0 ? int.MaxValue : idx;
            })
            .ToList();
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

    static async Task AppendSyncWarningAsync(AppDbContext db, string titleId, string pluginId, string message)
    {
        try
        {
            db.SyncWarnings.Add(new Data.Models.SyncWarning
            {
                Id = Guid.NewGuid().ToString(),
                TitleId = titleId,
                PluginId = pluginId,
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
    [JsonPropertyName("is_explicit")]     public bool IsExplicit { get; set; }
}

public class AddMangaBody
{
    [JsonPropertyName("mangaupdates_id")] public string? MangaupdatesId { get; set; }
    [JsonPropertyName("source")]          public string? Source { get; set; }
    [JsonPropertyName("source_id")]       public string? SourceId { get; set; }
    [JsonPropertyName("title")]           public string Title { get; set; } = null!;
    [JsonPropertyName("description")]     public string? Description { get; set; }
    [JsonPropertyName("cover_url")]       public string? CoverUrl { get; set; }
    [JsonPropertyName("status")]          public string Status { get; set; } = "unknown";
    [JsonPropertyName("author")]          public string? Author { get; set; }
    [JsonPropertyName("year")]            public int? Year { get; set; }
    [JsonPropertyName("tags")]            public string? Tags { get; set; }
    [JsonPropertyName("content_type")]    public string ContentType { get; set; } = "manga";
    [JsonPropertyName("is_explicit")]     public bool? IsExplicit { get; set; }
}
