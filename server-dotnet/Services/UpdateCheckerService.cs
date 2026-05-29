using ArrghServer.Api;
using ArrghServer.Data;
using Microsoft.EntityFrameworkCore;

namespace ArrghServer.Services;

/// <summary>
/// Background service: polls GitHub releases API every hour when check_for_updates=true.
/// Populates UpdateCache so GET /api/version can return the latest release info.
/// </summary>
public class UpdateCheckerService(
    UpdateCache cache,
    IServiceScopeFactory scopeFactory,
    IHttpClientFactory httpClientFactory,
    ILogger<UpdateCheckerService> logger) : BackgroundService
{
    private const string Repo = "t2vi/arrgh";
    private static readonly TimeSpan Interval = TimeSpan.FromHours(1);

    protected override async Task ExecuteAsync(CancellationToken stoppingToken)
    {
        while (!stoppingToken.IsCancellationRequested)
        {
            try
            {
                await CheckAsync(stoppingToken);
            }
            catch (Exception ex) when (ex is not OperationCanceledException)
            {
                logger.LogDebug("update check failed: {Error}", ex.Message);
            }

            await Task.Delay(Interval, stoppingToken);
        }
    }

    private async Task CheckAsync(CancellationToken ct)
    {
        bool enabled;
        using (var scope = scopeFactory.CreateScope())
        {
            var db = scope.ServiceProvider.GetRequiredService<AppDbContext>();
            var val = await db.ServerSettings
                .Where(s => s.Key == "check_for_updates")
                .Select(s => s.Value)
                .FirstOrDefaultAsync(ct);
            enabled = val == "true";
        }

        if (!enabled)
        {
            cache.Clear();
            return;
        }

        var http = httpClientFactory.CreateClient();
        var url = $"https://api.github.com/repos/{Repo}/releases/latest";
        using var req = new HttpRequestMessage(HttpMethod.Get, url);
        req.Headers.Add("User-Agent", "arrgh-server");
        req.Headers.Add("Accept", "application/vnd.github+json");

        using var res = await http.SendAsync(req, ct);
        if (!res.IsSuccessStatusCode)
        {
            logger.LogDebug("GitHub API returned {Status}", res.StatusCode);
            return;
        }

        var json = await res.Content.ReadFromJsonAsync<System.Text.Json.JsonElement>(ct);
        var tag = json.GetProperty("tag_name").GetString();
        var htmlUrl = json.GetProperty("html_url").GetString();
        if (tag is null || htmlUrl is null) return;

        var version = tag.TrimStart('v');
        cache.Set(version, htmlUrl);
    }
}
