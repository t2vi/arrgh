using System.Net.Http.Json;
using System.Text.Json;
using System.Text.RegularExpressions;

namespace ArrghServer.Services;

public record EHentaiSeries(
    string SourceId,
    string Title,
    string? CoverUrl,
    string[]? Tags
);

public class EHentaiService(IHttpClientFactory httpFactory)
{
    const string SearchBase = "https://e-hentai.org/?f_search=";
    const string GdataUrl   = "https://api.e-hentai.org/api.php";

    public async Task<List<EHentaiSeries>> SearchAsync(string q)
    {
        var http = httpFactory.CreateClient();
        http.DefaultRequestHeaders.Add("User-Agent", "Mozilla/5.0");

        // 1. Scrape gallery list page
        var resp = await http.GetAsync($"{SearchBase}{Uri.EscapeDataString(q)}");
        resp.EnsureSuccessStatusCode();
        var html = await resp.Content.ReadAsStringAsync();
        if (string.IsNullOrWhiteSpace(html))
            throw new InvalidOperationException("E-Hentai returned empty response");

        var galleries = ParseSearchHtml(html);
        if (galleries.Count == 0) return [];

        // 2. Fetch metadata via gdata API (EH uses "namespace" key — serialize manually)
        var gidlistJson = string.Join(",", galleries.Select(g => $"[{g.gid},{JsonSerializer.Serialize(g.token)}]"));
        var payload = $"{{\"method\":\"gdata\",\"gidlist\":[{gidlistJson}],\"namespace\":1}}";
        var gdataResp = await http.PostAsync(GdataUrl, new StringContent(payload, System.Text.Encoding.UTF8, "application/json"));
        gdataResp.EnsureSuccessStatusCode();
        var gdataJson = await gdataResp.Content.ReadFromJsonAsync<JsonElement>();

        return ParseGdata(gdataJson);
    }

    internal static List<(string gid, string token)> ParseSearchHtml(string html)
    {
        // Match gallery links: /g/{gid}/{token}/
        var matches = Regex.Matches(html, @"e-hentai\.org/g/(\d+)/([a-f0-9]+)/");
        var galleries = new List<(string, string)>();
        var seen = new HashSet<string>();

        foreach (Match m in matches)
        {
            var gid = m.Groups[1].Value;
            if (!seen.Add(gid)) continue;
            galleries.Add((gid, m.Groups[2].Value));
        }
        return galleries;
    }

    internal static List<EHentaiSeries> ParseGdata(JsonElement root)
    {
        var results = new List<EHentaiSeries>();
        if (!root.TryGetProperty("gmetadata", out var meta)) return results;

        foreach (var item in meta.EnumerateArray())
        {
            var gid = item.TryGetProperty("gid", out var gidEl) ? gidEl.GetUInt64().ToString() : null;
            if (gid is null) continue;

            var title = item.TryGetProperty("title", out var tEl) && tEl.ValueKind == JsonValueKind.String
                ? tEl.GetString() : null;
            if (string.IsNullOrEmpty(title)) continue;

            string? cover = item.TryGetProperty("thumb", out var thumbEl) && thumbEl.ValueKind == JsonValueKind.String
                ? thumbEl.GetString() : null;

            string[]? tags = null;
            if (item.TryGetProperty("tags", out var tagsEl) && tagsEl.ValueKind == JsonValueKind.Array)
                tags = tagsEl.EnumerateArray()
                    .Where(t => t.ValueKind == JsonValueKind.String)
                    .Select(t => t.GetString()!)
                    .ToArray();

            results.Add(new EHentaiSeries(gid, title, cover, tags));
        }
        return results;
    }
}
