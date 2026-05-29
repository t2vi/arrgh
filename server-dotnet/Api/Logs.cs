using System.Security.Claims;
using System.Text.Json.Serialization;
using ArrghServer.Data;
using ArrghServer.Services;
using Microsoft.AspNetCore.Mvc;
using Microsoft.EntityFrameworkCore;

namespace ArrghServer.Api;

public static class Logs
{
    public static RouteGroupBuilder MapLogsRoutes(this RouteGroupBuilder group)
    {
        group.MapGet("/", GetLogs).RequireAuthorization();
        group.MapGet("/level", GetLevel).RequireAuthorization();
        group.MapPatch("/level", SetLevel).RequireAuthorization();
        return group;
    }

    // -------------------------------------------------------------------------

    static IResult GetLogs(LogService logs, [FromQuery] int? limit)
    {
        var entries = logs.GetRecent(limit ?? 200);
        return Results.Ok(entries);
    }

    static IResult GetLevel(LogService logs) =>
        Results.Ok(new { level = logs.CurrentLevel });

    static async Task<IResult> SetLevel(SetLevelBody body, ClaimsPrincipal principal, LogService logs, AppDbContext db)
    {
        if (principal.FindFirstValue(ClaimTypes.Role) != "admin")
            return Results.Forbid();

        if (!logs.SetLevel(body.Level))
            return Results.UnprocessableEntity();

        // Persist level so it survives restarts
        var setting = await db.ServerSettings.FindAsync("log_level");
        if (setting is null)
            db.ServerSettings.Add(new() { Key = "log_level", Value = body.Level.ToUpperInvariant() });
        else
            setting.Value = body.Level.ToUpperInvariant();

        await db.SaveChangesAsync();
        return Results.NoContent();
    }
}

record SetLevelBody([property: JsonPropertyName("level")] string Level);
