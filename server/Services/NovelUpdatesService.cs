using System.Text.RegularExpressions;
using System.Text.Json;

namespace ArrghServer.Services;

public record NovelUpdatesSeries(
    string SourceId,
    string Title,
    string? CoverUrl,
    string Status
);

public class NovelUpdatesService(IHttpClientFactory httpFactory, IConfiguration config)
{
    // NovelUpdates is CF-protected. Search is proxied through plugin-host which has CloakBrowser.
    // Falls back to direct scraping when plugin-host is not reachable (dev without CloakBrowser).
    const string PluginId = "novelupdates";

    public async Task<List<NovelUpdatesSeries>> SearchAsync(string q)
    {
        var pluginHostUrl = config["PluginHostUrl"] ?? "http://plugin-host:4000";
        var http = httpFactory.CreateClient();

        var url = $"{pluginHostUrl.TrimEnd('/')}/{PluginId}/search?q={Uri.EscapeDataString(q)}";
        var resp = await http.GetAsync(url);
        resp.EnsureSuccessStatusCode();

        var results = await resp.Content.ReadFromJsonAsync<List<PluginSearchResult>>(
            new JsonSerializerOptions { PropertyNameCaseInsensitive = true });

        return results?.Select(r => new NovelUpdatesSeries(
            SourceId: r.Id ?? "",
            Title: r.Title ?? "",
            CoverUrl: r.CoverUrl,
            Status: r.Status ?? "unknown"
        )).Where(r => !string.IsNullOrEmpty(r.SourceId) && !string.IsNullOrEmpty(r.Title))
          .ToList() ?? [];
    }

    // ── HTML parser kept for testing / fallback reference ─────────────────────

    internal static List<NovelUpdatesSeries> ParseHtml(string html)
    {
        var results = new List<NovelUpdatesSeries>();
        var blocks = SplitBySearchBox(html);
        foreach (var block in blocks)
        {
            var entry = ParseBlock(block);
            if (entry is not null) results.Add(entry);
        }
        return results;
    }

    static List<string> SplitBySearchBox(string html)
    {
        var parts = new List<string>();
        var marker = "search_main_box_nu";
        var idx = 0;
        while (true)
        {
            var start = html.IndexOf(marker, idx, StringComparison.Ordinal);
            if (start < 0) break;
            var end = html.IndexOf(marker, start + marker.Length, StringComparison.Ordinal);
            var block = end < 0 ? html[start..] : html[start..end];
            parts.Add(block);
            idx = end < 0 ? html.Length : end;
            if (end < 0) break;
        }
        return parts;
    }

    static NovelUpdatesSeries? ParseBlock(string block)
    {
        var titleMatch = Regex.Match(block, @"search_title[^>]*>.*?href=""/series/([^/""]+)/""[^>]*>([^<]+)</a>", RegexOptions.Singleline);
        if (!titleMatch.Success) return null;

        var slug = titleMatch.Groups[1].Value.Trim();
        var title = System.Net.WebUtility.HtmlDecode(titleMatch.Groups[2].Value.Trim());
        if (string.IsNullOrEmpty(title)) return null;

        string? coverUrl = null;
        var imgMatch = Regex.Match(block, @"search_img_nu[^>]*>.*?<img[^>]+src=""([^""]+)""", RegexOptions.Singleline);
        if (imgMatch.Success) coverUrl = imgMatch.Groups[1].Value.Trim();

        var statusMatch = Regex.Match(block, @"series_latest_status[^>]*>([^<]+)<", RegexOptions.Singleline);
        var status = statusMatch.Success ? MapStatus(statusMatch.Groups[1].Value.Trim()) : "unknown";

        return new NovelUpdatesSeries(slug, title, coverUrl, status);
    }

    static string MapStatus(string s) => s.ToLowerInvariant() switch
    {
        "completed" => "complete",
        "ongoing" or "publishing" => "ongoing",
        "hiatus" => "hiatus",
        "dropped" => "cancelled",
        _ => "unknown",
    };

    record PluginSearchResult(
        [property: System.Text.Json.Serialization.JsonPropertyName("id")]        string? Id,
        [property: System.Text.Json.Serialization.JsonPropertyName("title")]     string? Title,
        [property: System.Text.Json.Serialization.JsonPropertyName("cover_url")] string? CoverUrl,
        [property: System.Text.Json.Serialization.JsonPropertyName("status")]    string? Status
    );
}
