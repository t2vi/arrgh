using System.Text.RegularExpressions;

namespace ArrghServer.Services;

public record NovelUpdatesSeries(
    string SourceId,
    string Title,
    string? CoverUrl,
    string Status
);

public class NovelUpdatesService(IHttpClientFactory httpFactory)
{
    const string Base = "https://www.novelupdates.com";

    public async Task<List<NovelUpdatesSeries>> SearchAsync(string q)
    {
        var http = httpFactory.CreateClient();
        http.DefaultRequestHeaders.Add("User-Agent", "Mozilla/5.0");
        var resp = await http.GetAsync($"{Base}/?s={Uri.EscapeDataString(q)}&post_type=seriesplans");
        resp.EnsureSuccessStatusCode();
        var html = await resp.Content.ReadAsStringAsync();
        if (string.IsNullOrWhiteSpace(html))
            throw new InvalidOperationException("NovelUpdates returned empty response");
        return ParseHtml(html);
    }

    internal static List<NovelUpdatesSeries> ParseHtml(string html)
    {
        var results = new List<NovelUpdatesSeries>();

        // Match each search result block: <div class="search_main_box_nu">
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
        // Title: <div class="search_title"><a href="/series/{slug}/">Title</a>
        var titleMatch = Regex.Match(block, @"search_title[^>]*>.*?href=""/series/([^/""]+)/""[^>]*>([^<]+)</a>", RegexOptions.Singleline);
        if (!titleMatch.Success) return null;

        var slug = titleMatch.Groups[1].Value.Trim();
        var title = System.Net.WebUtility.HtmlDecode(titleMatch.Groups[2].Value.Trim());
        if (string.IsNullOrEmpty(title)) return null;

        // Cover: <img src="...">
        string? coverUrl = null;
        var imgMatch = Regex.Match(block, @"search_img_nu[^>]*>.*?<img[^>]+src=""([^""]+)""", RegexOptions.Singleline);
        if (imgMatch.Success) coverUrl = imgMatch.Groups[1].Value.Trim();

        // Status
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
}
