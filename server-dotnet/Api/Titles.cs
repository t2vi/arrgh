using System.Linq.Expressions;
using System.Security.Claims;
using System.Text.Json;
using System.Text.Json.Serialization;
using ArrghServer.Data;
using ArrghServer.Data.Models;
using ArrghServer.Services;
using Microsoft.AspNetCore.Mvc;
using Microsoft.EntityFrameworkCore;

namespace ArrghServer.Api;

public static class Titles
{
    public static RouteGroupBuilder MapTitlesRoutes(this RouteGroupBuilder group)
    {
        group.MapGet("/", ListTitles).RequireAuthorization();
        group.MapGet("/new-releases", NewReleases).RequireAuthorization();
        group.MapGet("/{id}", GetTitle).RequireAuthorization();
        group.MapDelete("/{id}", RemoveTitle).RequireAuthorization();
        group.MapPatch("/{id}", PatchTitle).RequireAuthorization();
        group.MapPost("/{id}/sync", SyncTitle).RequireAuthorization();
        group.MapGet("/{id}/sync-log", GetSyncLog).RequireAuthorization();
        group.MapPost("/{id}/refresh-metadata", RefreshMetadata).RequireAuthorization();
        return group;
    }

    // -------------------------------------------------------------------------

    static async Task<IResult> ListTitles(
        ClaimsPrincipal principal,
        AppDbContext db,
        [FromQuery] int? page,
        [FromQuery] int? limit,
        [FromQuery] string? search)
    {
        var userId = principal.FindFirstValue(ClaimTypes.NameIdentifier)!;
        var allowExplicit = principal.FindFirstValue("allow_explicit") == "true";
        var p = Math.Max(page ?? 1, 1);
        var l = Math.Min(limit ?? 20, 100);

        var baseQuery = db.Titles
            .Where(m => m.UserTitles.Any(ut => ut.UserId == userId)
                     && (m.IsExplicit == false || allowExplicit));

        if (!string.IsNullOrEmpty(search))
            baseQuery = baseQuery.Where(m => EF.Functions.Like(m.TitleName, $"%{search}%"));

        var total = await baseQuery.CountAsync();

        var ordered = string.IsNullOrEmpty(search)
            ? baseQuery.OrderByDescending(m => m.UpdatedAt)
            : baseQuery.OrderBy(m => m.TitleName);

        var items = await ordered
            .Skip((p - 1) * l)
            .Take(l)
            .Select(TitleProjection(userId, db))
            .ToListAsync();

        return Results.Ok(new { items, total, page = p, limit = l });
    }

    static async Task<IResult> NewReleases(ClaimsPrincipal principal, AppDbContext db)
    {
        var userId = principal.FindFirstValue(ClaimTypes.NameIdentifier)!;
        var allowExplicit = principal.FindFirstValue("allow_explicit") == "true";

        var items = await db.Chapters
            .Where(c => c.IsNew
                     && c.Title.UserTitles.Any(ut => ut.UserId == userId)
                     && (c.Title.IsExplicit == false || allowExplicit))
            .OrderByDescending(c => c.CreatedAt)
            .Take(30)
            .Select(c => new NewReleaseItem
            {
                ChapterId = c.Id,
                ChapterNumber = c.Number,
                ChapterTitle = c.ChapterTitle,
                ChapterCreatedAt = c.CreatedAt,
                Downloaded = c.Downloaded,
                MangaId = c.TitleId,
                MangaTitle = c.Title.TitleName,
                CoverUrl = c.Title.CoverUrl,
            })
            .ToListAsync();

        return Results.Ok(items);
    }

    static async Task<IResult> GetTitle(string id, ClaimsPrincipal principal, AppDbContext db)
    {
        var userId = principal.FindFirstValue(ClaimTypes.NameIdentifier)!;
        var allowExplicit = principal.FindFirstValue("allow_explicit") == "true";

        var title = await db.Titles
            .Where(m => m.Id == id
                     && m.UserTitles.Any(ut => ut.UserId == userId)
                     && (m.IsExplicit == false || allowExplicit))
            .Select(TitleProjection(userId, db))
            .FirstOrDefaultAsync();

        if (title is null) return Results.NotFound();
        return Results.Ok(title);
    }

    internal static Task<TitleListItem?> FetchTitleAsync(AppDbContext db, string titleId, string userId) =>
        db.Titles.Where(t => t.Id == titleId).Select(TitleProjection(userId, db)).FirstOrDefaultAsync();

    // Returns an expression tree so EF can translate the full projection to SQL.
    // Must be Expression<Func<...>>, NOT a plain method — a plain method is evaluated
    // client-side after loading the entity, leaving navigations empty.
    static Expression<Func<Title, TitleListItem>> TitleProjection(string userId, AppDbContext db) =>
        m => new TitleListItem
        {
            Id = m.Id,
            Title = m.TitleName,
            Description = m.Description,
            CoverUrl = m.CoverUrl,
            Status = m.Status,
            IsLocal = !m.TitleSources.Any(),
            LocalPath = m.LocalPath,
            Author = m.Author,
            Year = m.Year,
            Tags = m.Tags,
            SyncStatus = m.SyncStatus,
            ContentType = m.ContentType,
            IsExplicit = m.IsExplicit,
            AutoDownload = m.AutoDownload,
            ReaderMode = db.UserTitleSettings
                .Where(s => s.TitleId == m.Id && s.UserId == userId)
                .Select(s => s.ReaderMode)
                .FirstOrDefault(),
            DownloadDir = m.DownloadDir,
            CreatedAt = m.CreatedAt,
            UpdatedAt = m.UpdatedAt,
            TotalChapters = m.Chapters.LongCount(),
            DownloadedChapters = m.Chapters.LongCount(c => c.Downloaded),
            ChaptersRead = db.ReadProgresses.LongCount(rp =>
                rp.UserId == userId && rp.Completed && rp.Chapter.TitleId == m.Id),
            HasSyncWarnings = m.SyncWarnings.Any(),
        };

    static async Task<IResult> RemoveTitle(
        string id,
        ClaimsPrincipal principal,
        AppDbContext db,
        [FromQuery] bool? delete_files)
    {
        var userId = principal.FindFirstValue(ClaimTypes.NameIdentifier)!;
        var isAdmin = principal.FindFirstValue(ClaimTypes.Role) == "admin";

        var userTitle = await db.UserTitles.FirstOrDefaultAsync(ut => ut.UserId == userId && ut.TitleId == id);
        if (userTitle is null) return Results.NotFound();

        db.UserTitles.Remove(userTitle);
        await db.SaveChangesAsync();

        var remaining = await db.UserTitles.CountAsync(ut => ut.TitleId == id);
        if (remaining == 0)
        {
            if ((delete_files ?? false) && isAdmin)
            {
                var chapterPaths = await db.Chapters
                    .Where(c => c.TitleId == id && c.LocalPath != null)
                    .Select(c => c.LocalPath!)
                    .ToListAsync();

                var coverUrl = await db.Titles
                    .Where(t => t.Id == id)
                    .Select(t => t.CoverUrl)
                    .FirstOrDefaultAsync();

                foreach (var path in chapterPaths.Where(p => !p.StartsWith("http")))
                    try { File.Delete(path); } catch { }

                if (coverUrl is not null && !coverUrl.StartsWith("http"))
                    try { File.Delete(coverUrl); } catch { }
            }

            var title = await db.Titles.FindAsync(id);
            if (title is not null)
            {
                db.Titles.Remove(title);
                await db.SaveChangesAsync();
            }
        }

        return Results.NoContent();
    }

    static async Task<IResult> PatchTitle(string id, PatchTitleBody body, ClaimsPrincipal principal, AppDbContext db)
    {
        var userId = principal.FindFirstValue(ClaimTypes.NameIdentifier)!;
        var isAdmin = principal.FindFirstValue(ClaimTypes.Role) == "admin";

        var owns = await db.UserTitles.AnyAsync(ut => ut.UserId == userId && ut.TitleId == id);
        if (!owns) return Results.NotFound();

        var title = await db.Titles.FindAsync(id);
        if (title is null) return Results.NotFound();

        if (body.AutoDownload is not null)
            title.AutoDownload = body.AutoDownload;

        if (body.ReaderMode.HasValue)
        {
            var rm = body.ReaderMode.Value.ValueKind == JsonValueKind.Null
                ? null
                : body.ReaderMode.Value.GetString();
            if (rm is not null && rm != "paged" && rm != "scroll")
                return Results.UnprocessableEntity();

            var settings = await db.UserTitleSettings.FindAsync(userId, id);
            if (settings is null)
                db.UserTitleSettings.Add(new() { UserId = userId, TitleId = id, ReaderMode = rm });
            else
                settings.ReaderMode = rm;
        }

        if (body.DownloadDir.HasValue)
            title.DownloadDir = body.DownloadDir.Value.ValueKind == JsonValueKind.Null
                ? null
                : body.DownloadDir.Value.GetString();

        if (body.IsExplicit is not null)
        {
            if (!isAdmin) return Results.Forbid();
            title.IsExplicit = body.IsExplicit.Value;
        }

        if (body.CoverUrl is not null)
        {
            if (!isAdmin) return Results.Forbid();
            title.CoverUrl = string.IsNullOrEmpty(body.CoverUrl) ? null : body.CoverUrl;
        }

        if (body.ContentType is not null)
        {
            if (!isAdmin) return Results.Forbid();
            if (!new[] { "manga", "manhwa", "manhua", "novel" }.Contains(body.ContentType))
                return Results.UnprocessableEntity();

            if (title.ContentType != body.ContentType)
            {
                title.ContentType = body.ContentType;
                await db.SaveChangesAsync();
                _ = Task.Run(() => ReMatchSourcesAsync(db, id, body.ContentType));
                return Results.NoContent();
            }
        }

        await db.SaveChangesAsync();
        return Results.NoContent();
    }

    static async Task<IResult> SyncTitle(string id, ClaimsPrincipal principal, AppDbContext db)
    {
        var userId = principal.FindFirstValue(ClaimTypes.NameIdentifier)!;

        var exists = await db.Titles.AnyAsync(m =>
            m.Id == id && m.UserTitles.Any(ut => ut.UserId == userId));
        if (!exists) return Results.NotFound();

        var sourceLinks = await db.TitleSources.Where(ts => ts.TitleId == id).ToListAsync();
        if (sourceLinks.Count == 0) return Results.NotFound();

        await db.Titles.Where(t => t.Id == id)
            .ExecuteUpdateAsync(s => s.SetProperty(t => t.SyncStatus, "syncing"));
        await db.SyncLogs.Where(l => l.TitleId == id).ExecuteDeleteAsync();

        _ = Task.Run(() => SyncTitleAsync(db, id, sourceLinks.Select(s => (s.Source, s.SourceId)).ToList()));

        return Results.StatusCode(202);
    }

    static async Task<IResult> GetSyncLog(string id, ClaimsPrincipal principal, AppDbContext db)
    {
        var userId = principal.FindFirstValue(ClaimTypes.NameIdentifier)!;

        var owns = await db.UserTitles.AnyAsync(ut => ut.UserId == userId && ut.TitleId == id);
        if (!owns) return Results.NotFound();

        var entries = await db.SyncLogs
            .Where(l => l.TitleId == id)
            .OrderBy(l => l.CreatedAt)
            .Select(l => new { l.Id, l.Message, l.CreatedAt })
            .ToListAsync();

        return Results.Ok(entries);
    }

    static async Task<IResult> RefreshMetadata(string id, ClaimsPrincipal principal, AppDbContext db, MangaUpdatesService mu)
    {
        var userId = principal.FindFirstValue(ClaimTypes.NameIdentifier)!;

        var title = await db.Titles.FirstOrDefaultAsync(m =>
            m.Id == id && m.UserTitles.Any(ut => ut.UserId == userId));
        if (title is null) return Results.NotFound();
        if (title.MangaupdatesId is null) return Results.UnprocessableEntity();
        if (!ulong.TryParse(title.MangaupdatesId, out var muId)) return Results.UnprocessableEntity();

        var series = await mu.SeriesDetailAsync(muId);
        if (series is null) return Results.NotFound();

        if (series.CoverUrl is not null && title.CoverUrl is null)
            title.CoverUrl = series.CoverUrl;

        await db.TitleAliases.Where(a => a.TitleId == id).ExecuteDeleteAsync();
        foreach (var alias in series.AssociatedNames)
            db.TitleAliases.Add(new() { Id = Guid.NewGuid().ToString(), TitleId = id, Alias = alias });

        await db.SyncLogs.Where(l => l.TitleId == id).ExecuteDeleteAsync();
        await db.SaveChangesAsync();

        _ = Task.Run(() => RefreshMatchAsync(db, id, series.Title, series.AssociatedNames));

        return Results.StatusCode(202);
    }

    // -------------------------------------------------------------------------
    // Background stubs — replaced when plugin system is ported (ADR 0030)

    static async Task ReMatchSourcesAsync(AppDbContext db, string titleId, string newContentType)
    {
        // TODO: port from Rust titles.rs patch_title content_type branch
        await db.Titles.Where(t => t.Id == titleId)
            .ExecuteUpdateAsync(s => s.SetProperty(t => t.SyncStatus, "ready"));
    }

    static async Task SyncTitleAsync(AppDbContext db, string titleId, List<(string Source, string SourceId)> links)
    {
        // TODO: port from Rust titles.rs sync_title — call plugin host per source link
        await db.Titles.Where(t => t.Id == titleId)
            .ExecuteUpdateAsync(s => s.SetProperty(t => t.SyncStatus, "ready"));
    }

    static async Task RefreshMatchAsync(AppDbContext db, string titleId, string titleName, IList<string> aliases)
    {
        // TODO: port from Rust titles.rs refresh_metadata — re-run source matching for warned sources
        await Task.CompletedTask;
    }
}

// -------------------------------------------------------------------------
// DTOs

public class TitleListItem
{
    public string Id { get; set; } = null!;
    public string Title { get; set; } = null!;
    public string? Description { get; set; }
    public string? CoverUrl { get; set; }
    public string Status { get; set; } = null!;
    public bool IsLocal { get; set; }
    public string? LocalPath { get; set; }
    public string? Author { get; set; }
    public int? Year { get; set; }
    public string? Tags { get; set; }
    public string SyncStatus { get; set; } = null!;
    public string ContentType { get; set; } = null!;
    public bool IsExplicit { get; set; }
    public bool? AutoDownload { get; set; }
    public string? ReaderMode { get; set; }
    public string? DownloadDir { get; set; }
    public DateTime CreatedAt { get; set; }
    public DateTime UpdatedAt { get; set; }
    public long TotalChapters { get; set; }
    public long DownloadedChapters { get; set; }
    public long ChaptersRead { get; set; }
    public bool HasSyncWarnings { get; set; }
}

public class NewReleaseItem
{
    public string ChapterId { get; set; } = null!;
    public double ChapterNumber { get; set; }
    public string? ChapterTitle { get; set; }
    public DateTime ChapterCreatedAt { get; set; }
    public bool Downloaded { get; set; }
    public string MangaId { get; set; } = null!;
    public string MangaTitle { get; set; } = null!;
    public string? CoverUrl { get; set; }
}

// ReaderMode/DownloadDir: JsonElement? distinguishes absent (no-op) from JSON null (clear).
// Known limitation: System.Text.Json maps JSON null → C# null for JsonElement? (HasValue=false),
// so clearing is indistinguishable from omitting — tracked as a follow-up.
public class PatchTitleBody
{
    [JsonPropertyName("auto_download")]  public bool? AutoDownload { get; set; }
    [JsonPropertyName("reader_mode")]    public JsonElement? ReaderMode { get; set; }
    [JsonPropertyName("download_dir")]   public JsonElement? DownloadDir { get; set; }
    [JsonPropertyName("is_explicit")]    public bool? IsExplicit { get; set; }
    [JsonPropertyName("cover_url")]      public string? CoverUrl { get; set; }
    [JsonPropertyName("content_type")]   public string? ContentType { get; set; }
}

