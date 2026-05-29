using System.Linq.Expressions;
using System.Security.Claims;
using ArrghServer.Data;
using ArrghServer.Data.Models;
using Microsoft.EntityFrameworkCore;

namespace ArrghServer.Api;

public static class Queue
{
    public static RouteGroupBuilder MapQueueRoutes(this RouteGroupBuilder group)
    {
        group.MapGet("/", ListQueue).RequireAuthorization();
        group.MapGet("/title/{titleId}", ListTitleQueue).RequireAuthorization();
        group.MapDelete("/completed", ClearCompleted).RequireAuthorization();
        group.MapDelete("/{id}", RemoveFromQueue).RequireAuthorization();
        return group;
    }

    // -------------------------------------------------------------------------

    static async Task<IResult> ListQueue(ClaimsPrincipal principal, AppDbContext db)
    {
        var allowExplicit = IsAllowedExplicit(principal);

        var items = await db.DownloadQueue
            .Where(dq => dq.Chapter.Title.IsExplicit == false || allowExplicit)
            .OrderByDescending(dq => dq.CreatedAt)
            .Take(100)
            .Select(QueueItemProjection)
            .ToListAsync();

        return Results.Ok(items);
    }

    static async Task<IResult> ListTitleQueue(string titleId, ClaimsPrincipal principal, AppDbContext db)
    {
        var allowExplicit = IsAllowedExplicit(principal);

        var items = await db.DownloadQueue
            .Where(dq => dq.Chapter.TitleId == titleId
                      && (dq.Chapter.Title.IsExplicit == false || allowExplicit))
            .OrderBy(dq => dq.ChapterNum)
            .Select(QueueItemProjection)
            .ToListAsync();

        return Results.Ok(items);
    }

    static async Task<IResult> ClearCompleted(ClaimsPrincipal principal, AppDbContext db)
    {
        if (principal.FindFirstValue(ClaimTypes.Role) != "admin")
            return Results.Forbid();

        await db.DownloadQueue
            .Where(q => q.Status == "done" || q.Status == "cancelled" || q.Status == "error")
            .ExecuteDeleteAsync();

        return Results.NoContent();
    }

    static async Task<IResult> RemoveFromQueue(string id, ClaimsPrincipal principal, AppDbContext db)
    {
        var userId = principal.FindFirstValue(ClaimTypes.NameIdentifier)!;
        var isAdmin = principal.FindFirstValue(ClaimTypes.Role) == "admin";

        var item = await db.DownloadQueue
            .Where(q => q.Id == id)
            .Select(q => new { q.QueuedBy, q.Status })
            .FirstOrDefaultAsync();

        if (item is null) return Results.NotFound();

        if (!isAdmin && item.QueuedBy != userId)
            return Results.Forbid();

        // Delete if not in-progress; if in-progress, cancel instead
        var deleted = await db.DownloadQueue
            .Where(q => q.Id == id && q.Status != "in_progress")
            .ExecuteDeleteAsync();

        if (deleted == 0)
        {
            await db.DownloadQueue
                .Where(q => q.Id == id)
                .ExecuteUpdateAsync(s => s
                    .SetProperty(q => q.Status, "cancelled")
                    .SetProperty(q => q.UpdatedAt, DateTime.UtcNow));
        }

        return Results.NoContent();
    }

    // -------------------------------------------------------------------------

    // Admin always sees explicit content regardless of allow_explicit flag
    static bool IsAllowedExplicit(ClaimsPrincipal p) =>
        IsAllowedExplicit(p.FindFirstValue(ClaimTypes.Role), p.FindFirstValue("allow_explicit") == "true");

    // Pure overload — no ClaimsPrincipal dependency, directly unit-testable
    internal static bool IsAllowedExplicit(string? role, bool allowExplicit) =>
        allowExplicit || role == "admin";

    static Expression<Func<DownloadQueueItem, QueueItemDto>> QueueItemProjection =>
        dq => new QueueItemDto
        {
            Id = dq.Id,
            ChapterId = dq.ChapterId,
            MangaTitle = dq.MangaTitle,
            ChapterNum = dq.ChapterNum,
            Status = dq.Status,
            Error = dq.Error,
            PagesDownloaded = dq.PagesDownloaded,
            PagesTotal = dq.PagesTotal,
            CreatedAt = dq.CreatedAt,
            UpdatedAt = dq.UpdatedAt,
        };
}

// ── DTO ───────────────────────────────────────────────────────────────────────

public class QueueItemDto
{
    public string Id { get; set; } = null!;
    public string ChapterId { get; set; } = null!;
    public string MangaTitle { get; set; } = null!;
    public double ChapterNum { get; set; }
    public string Status { get; set; } = null!;
    public string? Error { get; set; }
    public int PagesDownloaded { get; set; }
    public int PagesTotal { get; set; }
    public DateTime CreatedAt { get; set; }
    public DateTime UpdatedAt { get; set; }
}
