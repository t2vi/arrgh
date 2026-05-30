using System.Security.Claims;
using System.Text.Json.Serialization;
using ArrghServer.Data;
using ArrghServer.Data.Models;
using Microsoft.EntityFrameworkCore;

namespace ArrghServer.Api;

public static class Plugins
{
    public static RouteGroupBuilder MapPluginsRoutes(this RouteGroupBuilder group)
    {
        group.MapGet("/index", GetIndex).RequireAuthorization().WithSummary("List all available plugins from the index");
        group.MapPost("/install", InstallPlugin).RequireAuthorization().WithSummary("Install a community plugin from URL");
        group.MapDelete("/{id}", DeletePlugin).RequireAuthorization().WithSummary("Remove a community plugin (bundled plugins cannot be removed)");
        return group;
    }

    // ── GET /api/plugins/index ────────────────────────────────────────────────

    static async Task<IResult> GetIndex(IConfiguration config, IHttpClientFactory http)
    {
        var url = config["PluginIndexUrl"] ?? "file:///app/plugin-index.json";
        var entries = await FetchIndexAsync(url, http);
        if (entries is null) return Results.StatusCode(502);
        return Results.Ok(entries);
    }

    // ── POST /api/plugins/install ─────────────────────────────────────────────

    static async Task<IResult> InstallPlugin(
        InstallBody body, ClaimsPrincipal principal,
        AppDbContext db, IConfiguration config, IHttpClientFactory httpFactory)
    {
        if (!Sources.IsAdmin(principal)) return Results.Forbid();

        var pluginIndexUrl = config["PluginIndexUrl"] ?? "file:///app/plugin-index.json";
        var pluginHostUrl = config["PluginHostUrl"] ?? "http://localhost:4000";

        var index = await FetchIndexAsync(pluginIndexUrl, httpFactory);
        if (index is null) return Results.StatusCode(502);

        var entry = index.FirstOrDefault(e => e.Id == body.PluginId);
        if (entry is null) return Results.NotFound();

        if (string.IsNullOrWhiteSpace(entry.DownloadUrl))
            return Results.UnprocessableEntity();

        var effectiveUrl = $"{pluginHostUrl.TrimEnd('/')}/{body.PluginId}";

        // Check duplicate before calling plugin-host (saves a round-trip on conflicts)
        var already = await db.ExternalSources.AnyAsync(s => s.BaseUrl == effectiveUrl);
        if (already) return Results.Conflict();

        // Tell plugin-host to download + load the bundle
        var http = httpFactory.CreateClient();
        var installUrl = $"{pluginHostUrl.TrimEnd('/')}/plugins/install";
        HttpResponseMessage installRes;
        try
        {
            installRes = await http.PostAsJsonAsync(installUrl, new { url = entry.DownloadUrl });
        }
        catch
        {
            return Results.StatusCode(502);
        }
        if (!installRes.IsSuccessStatusCode)
            return Results.StatusCode(502);

        db.ExternalSources.Add(new ExternalSource
        {
            Id = Guid.NewGuid().ToString(),
            Name = entry.Name,
            BaseUrl = effectiveUrl,
            ContentTypes = string.Join(",", entry.ContentTypes),
            Enabled = true,
            DefaultExplicit = entry.DefaultExplicit,
            IsCommunity = true,
            CreatedAt = DateTime.UtcNow,
        });
        await db.SaveChangesAsync();

        return Results.StatusCode(201);
    }

    // ── DELETE /api/plugins/{id} ──────────────────────────────────────────────

    static async Task<IResult> DeletePlugin(
        string id, ClaimsPrincipal principal,
        AppDbContext db, IConfiguration config, IHttpClientFactory httpFactory)
    {
        if (!Sources.IsAdmin(principal)) return Results.Forbid();

        var pluginHostUrl = config["PluginHostUrl"] ?? "http://localhost:4000";
        var effectiveUrl = $"{pluginHostUrl.TrimEnd('/')}/{id}";

        var source = await db.ExternalSources.FirstOrDefaultAsync(s => s.BaseUrl == effectiveUrl);
        if (source is null) return Results.NotFound();
        if (!source.IsCommunity) return Results.Forbid();

        // Tell plugin-host to unload — best-effort, don't fail on error
        try
        {
            var http = httpFactory.CreateClient();
            await http.DeleteAsync($"{pluginHostUrl.TrimEnd('/')}/plugins/{id}");
        }
        catch { }

        db.ExternalSources.Remove(source);
        await db.SaveChangesAsync();

        return Results.NoContent();
    }

    // ── Internal helpers ──────────────────────────────────────────────────────

    internal static async Task<List<PluginIndexEntry>?> FetchIndexAsync(string url, IHttpClientFactory httpFactory)
    {
        try
        {
            if (url.StartsWith("file://"))
            {
                var path = url["file://".Length..];
                var text = await File.ReadAllTextAsync(path);
                return System.Text.Json.JsonSerializer.Deserialize<List<PluginIndexEntry>>(text,
                    new System.Text.Json.JsonSerializerOptions { PropertyNameCaseInsensitive = true });
            }

            var http = httpFactory.CreateClient();
            return await http.GetFromJsonAsync<List<PluginIndexEntry>>(url,
                new System.Text.Json.JsonSerializerOptions { PropertyNameCaseInsensitive = true });
        }
        catch
        {
            return null;
        }
    }
}

// ── DTOs ──────────────────────────────────────────────────────────────────────

public class PluginIndexEntry
{
    [JsonPropertyName("id")]               public string Id { get; set; } = null!;
    [JsonPropertyName("name")]             public string Name { get; set; } = null!;
    [JsonPropertyName("description")]      public string? Description { get; set; }
    [JsonPropertyName("version")]          public string Version { get; set; } = null!;
    [JsonPropertyName("download_url")]     public string? DownloadUrl { get; set; }
    [JsonPropertyName("bundled")]          public bool? Bundled { get; set; }
    [JsonPropertyName("default_explicit")] public bool DefaultExplicit { get; set; }
    [JsonPropertyName("content_types")]    public List<string> ContentTypes { get; set; } = [];
}

public class InstallBody
{
    [JsonPropertyName("plugin_id")] public string PluginId { get; set; } = null!;
}
