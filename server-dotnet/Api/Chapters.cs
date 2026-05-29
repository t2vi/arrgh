using System.Linq.Expressions;
using System.Security.Claims;
using ArrghServer.Data;
using ArrghServer.Data.Models;
using Microsoft.EntityFrameworkCore;

namespace ArrghServer.Api;

public static class Chapters
{
    public static RouteGroupBuilder MapChaptersRoutes(this RouteGroupBuilder group)
    {
        group.MapGet("/title/{titleId}", ListChapters).RequireAuthorization().WithSummary("List chapters for a title");
        group.MapGet("/{id}", GetChapter).RequireAuthorization().WithSummary("Get a single chapter with page list");
        group.MapGet("/{id}/text", GetChapterText).RequireAuthorization().WithSummary("Get chapter text content (novels)");
        group.MapPost("/{id}/download", QueueDownload).RequireAuthorization().WithSummary("Queue a chapter for download");
        return group;
    }

    // -------------------------------------------------------------------------

    static async Task<IResult> ListChapters(string titleId, ClaimsPrincipal principal, AppDbContext db)
    {
        var allowExplicit = principal.FindFirstValue("allow_explicit") == "true";

        var chapters = await db.Chapters
            .Where(c => c.TitleId == titleId
                     && (c.Title.IsExplicit == false || allowExplicit))
            .OrderBy(c => c.Number)
            .Select(ChapterProjection)
            .ToListAsync();

        _ = Task.Run(() => VerifyDownloadsAsync(db, titleId));

        return Results.Ok(chapters);
    }

    static async Task<IResult> GetChapter(string id, ClaimsPrincipal principal, AppDbContext db)
    {
        var allowExplicit = principal.FindFirstValue("allow_explicit") == "true";

        var chapter = await db.Chapters
            .Where(c => c.Id == id && (c.Title.IsExplicit == false || allowExplicit))
            .Select(ChapterProjection)
            .FirstOrDefaultAsync();

        if (chapter is null) return Results.NotFound();
        return Results.Ok(chapter);
    }

    static async Task<IResult> GetChapterText(string id, ClaimsPrincipal principal, AppDbContext db)
    {
        var allowExplicit = principal.FindFirstValue("allow_explicit") == "true";

        var row = await db.Chapters
            .Where(c => c.Id == id && (c.Title.IsExplicit == false || allowExplicit))
            .Select(c => new { c.LocalPath, c.Downloaded, c.ChapterFormat })
            .FirstOrDefaultAsync();

        if (row is null) return Results.NotFound();
        if (row.ChapterFormat != "text") return Results.BadRequest();
        if (!row.Downloaded) return Results.NotFound();
        if (row.LocalPath is null) return Results.NotFound();

        try
        {
            var content = await File.ReadAllTextAsync(row.LocalPath);
            return Results.Ok(new { content });
        }
        catch
        {
            return Results.NotFound();
        }
    }

    static async Task<IResult> QueueDownload(string id, ClaimsPrincipal principal, AppDbContext db)
    {
        var userId = principal.FindFirstValue(ClaimTypes.NameIdentifier)!;
        var allowExplicit = principal.FindFirstValue("allow_explicit") == "true";

        var candidate = await db.Chapters
            .Where(c => c.Id == id
                     && !c.Downloaded
                     && c.ChapterSources.Any()
                     && (c.Title.IsExplicit == false || allowExplicit))
            .Select(c => new { c.Id, c.Number, MangaTitle = c.Title.TitleName })
            .FirstOrDefaultAsync();

        if (candidate is null) return Results.NotFound();

        var queueId = Guid.NewGuid().ToString();
        var now = DateTime.UtcNow;

        // RAW SQL — intentional: SQLite upsert with conditional WHERE on status.
        // No EF equivalent for ON CONFLICT ... DO UPDATE ... WHERE.
        // If this becomes hard to maintain, consider: read existing row + conditional SaveChanges.
        await db.Database.ExecuteSqlRawAsync(@"
            INSERT INTO download_queue
                (id, chapter_id, manga_title, chapter_num, status, pages_downloaded, pages_total, created_at, updated_at, queued_by)
            VALUES ({0}, {1}, {2}, {3}, 'pending', 0, 0, {4}, {5}, {6})
            ON CONFLICT(chapter_id) DO UPDATE SET
                status = 'pending',
                error = NULL,
                queued_by = excluded.queued_by,
                updated_at = excluded.updated_at
            WHERE download_queue.status IN ('error', 'cancelled')",
            queueId, candidate.Id, candidate.MangaTitle, candidate.Number, now, now, userId);

        return Results.StatusCode(202);
    }

    // -------------------------------------------------------------------------

    // Expression tree — EF translates HasSources = c.ChapterSources.Any() to a SQL EXISTS subquery.
    // Must be Expression<Func<...>>, NOT a plain method (plain method → client-side eval → empty navigations).
    static Expression<Func<Chapter, ChapterDto>> ChapterProjection =>
        c => new ChapterDto
        {
            Id = c.Id,
            TitleId = c.TitleId,
            Title = c.ChapterTitle,
            Number = c.Number,
            Volume = c.Volume,
            LocalPath = c.LocalPath,
            PageCount = c.PageCount,
            Downloaded = c.Downloaded,
            HasSources = c.ChapterSources.Any(),
            ChapterFormat = c.ChapterFormat,
            CreatedAt = c.CreatedAt,
        };

    static async Task VerifyDownloadsAsync(AppDbContext db, string titleId)
    {
        // TODO: port from Rust indexer/local.rs verify_title_downloads
        // Resets downloaded=1 rows whose local_path file no longer exists on disk
        await Task.CompletedTask;
    }
}

// ── DTOs ──────────────────────────────────────────────────────────────────────

public class ChapterDto
{
    public string Id { get; set; } = null!;
    public string TitleId { get; set; } = null!;
    public string? Title { get; set; }
    public double Number { get; set; }
    public double? Volume { get; set; }
    public string? LocalPath { get; set; }
    public int PageCount { get; set; }
    public bool Downloaded { get; set; }
    public bool HasSources { get; set; }
    public string ChapterFormat { get; set; } = null!;
    public DateTime CreatedAt { get; set; }
}
