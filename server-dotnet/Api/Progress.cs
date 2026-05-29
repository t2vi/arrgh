using System.Linq.Expressions;
using System.Security.Claims;
using System.Text.Json.Serialization;
using ArrghServer.Data;
using ArrghServer.Data.Models;
using Microsoft.EntityFrameworkCore;

namespace ArrghServer.Api;

public static class Progress
{
    public static RouteGroupBuilder MapProgressRoutes(this RouteGroupBuilder group)
    {
        // /progress/continue must be registered before /progress/{chapterId}
        group.MapGet("/continue", ContinueReading).RequireAuthorization();
        group.MapGet("/title/{titleId}", ListTitleProgress).RequireAuthorization();
        group.MapGet("/{chapterId}", GetProgress).RequireAuthorization();
        group.MapPut("/{chapterId}", UpdateProgress).RequireAuthorization();
        return group;
    }

    // -------------------------------------------------------------------------

    static async Task<IResult> ListTitleProgress(string titleId, ClaimsPrincipal principal, AppDbContext db)
    {
        var userId = principal.FindFirstValue(ClaimTypes.NameIdentifier)!;

        var rows = await db.ReadProgresses
            .Where(rp => rp.UserId == userId && rp.Chapter.TitleId == titleId)
            .Select(ReadProgressProjection)
            .ToListAsync();

        return Results.Ok(rows);
    }

    static async Task<IResult> GetProgress(string chapterId, ClaimsPrincipal principal, AppDbContext db)
    {
        var userId = principal.FindFirstValue(ClaimTypes.NameIdentifier)!;

        var progress = await db.ReadProgresses
            .Where(rp => rp.ChapterId == chapterId && rp.UserId == userId)
            .Select(ReadProgressProjection)
            .FirstOrDefaultAsync();

        if (progress is null) return Results.NotFound();
        return Results.Ok(progress);
    }

    static async Task<IResult> ContinueReading(ClaimsPrincipal principal, AppDbContext db)
    {
        var userId = principal.FindFirstValue(ClaimTypes.NameIdentifier)!;

        var items = await db.Titles
            .Where(m =>
                // Has at least one completed chapter
                db.ReadProgresses.Any(rp =>
                    rp.UserId == userId && rp.Completed && rp.Chapter.TitleId == m.Id)
                &&
                // Has at least one unread downloaded chapter remaining
                db.Chapters.Any(c =>
                    c.TitleId == m.Id && c.Downloaded &&
                    !db.ReadProgresses.Any(rp =>
                        rp.ChapterId == c.Id && rp.UserId == userId && rp.Completed)))
            .OrderByDescending(m => m.UpdatedAt)
            .Take(10)
            .Select(m => new ContinueItemDto
            {
                TitleId = m.Id,
                MangaTitle = m.TitleName,
                CoverUrl = m.CoverUrl,
                ChapterId = db.Chapters
                    .Where(c => c.TitleId == m.Id && c.Downloaded &&
                        !db.ReadProgresses.Any(rp =>
                            rp.ChapterId == c.Id && rp.UserId == userId && rp.Completed))
                    .OrderBy(c => c.Number)
                    .Select(c => c.Id)
                    .FirstOrDefault(),
                ChapterNumber = db.Chapters
                    .Where(c => c.TitleId == m.Id && c.Downloaded &&
                        !db.ReadProgresses.Any(rp =>
                            rp.ChapterId == c.Id && rp.UserId == userId && rp.Completed))
                    .OrderBy(c => c.Number)
                    .Select(c => (double?)c.Number)
                    .FirstOrDefault(),
                ChaptersRead = db.ReadProgresses.LongCount(rp =>
                    rp.UserId == userId && rp.Completed && rp.Chapter.TitleId == m.Id),
                TotalChapters = m.Chapters.LongCount(),
            })
            .ToListAsync();

        return Results.Ok(items);
    }

    static async Task<IResult> UpdateProgress(
        string chapterId,
        UpdateProgressBody body,
        ClaimsPrincipal principal,
        AppDbContext db)
    {
        var userId = principal.FindFirstValue(ClaimTypes.NameIdentifier)!;
        var id = Guid.NewGuid().ToString();
        var now = DateTime.UtcNow;

        // RAW SQL — intentional: upsert with ON CONFLICT(user_id, chapter_id).
        // No EF equivalent for conditional upsert on a composite unique constraint.
        // If this becomes hard to maintain, consider: FindAsync + conditional Add/Update.
        await db.Database.ExecuteSqlRawAsync(@"
            INSERT INTO read_progress (id, user_id, chapter_id, current_page, completed, updated_at)
            VALUES ({0}, {1}, {2}, {3}, {4}, {5})
            ON CONFLICT(user_id, chapter_id) DO UPDATE SET
                current_page = excluded.current_page,
                completed    = excluded.completed,
                updated_at   = excluded.updated_at",
            id, userId, chapterId, body.CurrentPage, body.Completed ? 1 : 0, now);

        var progress = await db.ReadProgresses
            .Where(rp => rp.ChapterId == chapterId && rp.UserId == userId)
            .Select(ReadProgressProjection)
            .FirstAsync();

        return Results.Ok(progress);
    }

    // -------------------------------------------------------------------------

    static Expression<Func<ReadProgress, ReadProgressDto>> ReadProgressProjection =>
        rp => new ReadProgressDto
        {
            Id = rp.Id,
            UserId = rp.UserId,
            ChapterId = rp.ChapterId,
            CurrentPage = rp.CurrentPage,
            Completed = rp.Completed,
            UpdatedAt = rp.UpdatedAt,
        };
}

// ── DTOs ──────────────────────────────────────────────────────────────────────

public class ReadProgressDto
{
    public string Id { get; set; } = null!;
    public string UserId { get; set; } = null!;
    public string ChapterId { get; set; } = null!;
    public int CurrentPage { get; set; }
    public bool Completed { get; set; }
    public DateTime UpdatedAt { get; set; }
}

public class ContinueItemDto
{
    public string TitleId { get; set; } = null!;
    public string MangaTitle { get; set; } = null!;
    public string? CoverUrl { get; set; }
    public string? ChapterId { get; set; }
    public double? ChapterNumber { get; set; }
    public long ChaptersRead { get; set; }
    public long TotalChapters { get; set; }
}

public class UpdateProgressBody
{
    [JsonPropertyName("current_page")] public int CurrentPage { get; set; }
    [JsonPropertyName("completed")]     public bool Completed { get; set; }
}
