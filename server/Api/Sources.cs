using System.Security.Claims;
using System.Text.Json.Serialization;
using ArrghServer.Data;
using ArrghServer.Data.Models;
using Microsoft.EntityFrameworkCore;

namespace ArrghServer.Api;

public static class Sources
{
    public static RouteGroupBuilder MapSourcesRoutes(this RouteGroupBuilder group)
    {
        group.MapGet("/", ListSources).RequireAuthorization().WithSummary("List configured plugin sources");
        group.MapPost("/", AddSource).RequireAuthorization().WithSummary("Add a plugin source");
        group.MapPatch("/{id}", PatchSource).RequireAuthorization().WithSummary("Update source (enabled, priority, api_key…)");
        group.MapDelete("/{id}", DeleteSource).RequireAuthorization().WithSummary("Remove a source");
        return group;
    }

    // -------------------------------------------------------------------------

    static async Task<IResult> ListSources(ClaimsPrincipal principal, AppDbContext db)
    {
        // Two-step: EF loads raw fields, then client-side split of content_types CSV
        var raw = await db.ExternalSources
            .OrderBy(s => s.CreatedAt)
            .Select(s => new { s.Id, s.Name, s.BaseUrl, s.ApiKey, s.ContentTypes, s.Enabled, s.IsCommunity, s.Priority })
            .ToListAsync();

        var rows = raw.Select(s => new SourceRowDto
        {
            Id = s.Id,
            Name = s.Name,
            BaseUrl = s.BaseUrl,
            HasApiKey = s.ApiKey is not null,
            ContentTypes = s.ContentTypes
                .Split(',', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries),
            Enabled = s.Enabled,
            IsCommunity = s.IsCommunity,
            Priority = s.Priority,
        }).ToList();

        return Results.Ok(rows);
    }

    static Task<IResult> AddSource(AddSourceBody body, ClaimsPrincipal principal, IConfiguration config)
    {
        if (!IsAdmin(principal)) return Task.FromResult(Results.Forbid());

        // RAW HTTP — intentional: must probe plugin host to get source name/content_types/default_explicit.
        // Stubbed until plugin system is ported (ADR 0030).
        // When ported: GET {PluginHostUrl}/plugins/{sourceKey}/info → populate ExternalSource fields.
        return Task.FromResult(Results.StatusCode(502));
    }

    static async Task<IResult> PatchSource(string id, PatchSourceBody body, ClaimsPrincipal principal, AppDbContext db)
    {
        if (!IsAdmin(principal)) return Results.Forbid();

        var source = await db.ExternalSources.FindAsync(id);
        if (source is null) return Results.NotFound();

        source.Enabled = body.Enabled;
        if (body.Priority.HasValue) source.Priority = body.Priority.Value;
        await db.SaveChangesAsync();
        await ReloadRegistryAsync();

        return Results.NoContent();
    }

    static async Task<IResult> DeleteSource(string id, ClaimsPrincipal principal, AppDbContext db)
    {
        if (!IsAdmin(principal)) return Results.Forbid();

        var source = await db.ExternalSources.FindAsync(id);
        if (source is null) return Results.NotFound();

        db.ExternalSources.Remove(source);
        await db.SaveChangesAsync();
        await ReloadRegistryAsync();

        return Results.NoContent();
    }

    // -------------------------------------------------------------------------

    internal static bool IsAdmin(ClaimsPrincipal p) =>
        p.FindFirstValue(ClaimTypes.Role) == "admin";

    // TODO: rebuild in-memory source registry when plugin system is ported (ADR 0030)
    static Task ReloadRegistryAsync() => Task.CompletedTask;
}

// ── DTOs ──────────────────────────────────────────────────────────────────────

public class SourceRowDto
{
    public string Id { get; set; } = null!;
    public string Name { get; set; } = null!;
    public string BaseUrl { get; set; } = null!;
    public bool HasApiKey { get; set; }
    public string[] ContentTypes { get; set; } = [];
    public bool Enabled { get; set; }
    public bool IsCommunity { get; set; }
    public int Priority { get; set; }
}

public class AddSourceBody
{
    [JsonPropertyName("base_url")] public string BaseUrl { get; set; } = null!;
    [JsonPropertyName("api_key")]  public string? ApiKey { get; set; }
}

public class PatchSourceBody
{
    [JsonPropertyName("enabled")]  public bool Enabled { get; set; }
    [JsonPropertyName("priority")] public int? Priority { get; set; }
}
