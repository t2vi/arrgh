using System.Text.Json;
using System.Text.Json.Serialization;

namespace ArrghServer.Services;

public record WuxiaWorldMetaSeries(
    string SourceId,
    string Title,
    string? CoverUrl,
    string Status,
    string? Author
);

public class WuxiaWorldMetaService(IHttpClientFactory httpFactory)
{
    const string ApiBase = "https://www.wuxiaworld.com";
    const string UA = "Mozilla/5.0";

    public async Task<List<WuxiaWorldMetaSeries>> SearchAsync(string q)
    {
        var http = httpFactory.CreateClient();
        http.DefaultRequestHeaders.Add("User-Agent", UA);
        http.DefaultRequestHeaders.Add("Accept", "application/json");

        var url = $"{ApiBase}/api/novels/search?query={Uri.EscapeDataString(q)}&pageSize=20";
        var resp = await http.GetAsync(url);
        resp.EnsureSuccessStatusCode();

        var data = await resp.Content.ReadFromJsonAsync<WuxiaSearchResponse>(
            new JsonSerializerOptions { PropertyNameCaseInsensitive = true });

        return data?.Items?.Select(s => new WuxiaWorldMetaSeries(
            SourceId: s.Slug ?? s.Id.ToString() ?? "",
            Title: s.Name ?? "",
            CoverUrl: s.CoverUrl,
            Status: MapStatus(s.Status, s.Tags),
            Author: null  // not in search response
        )).Where(s => !string.IsNullOrEmpty(s.SourceId) && !string.IsNullOrEmpty(s.Title))
          .ToList() ?? [];
    }

    static string MapStatus(int? status, List<string>? tags)
    {
        if (tags?.Contains("Completed", StringComparer.OrdinalIgnoreCase) == true) return "complete";
        if (tags?.Contains("Ongoing", StringComparer.OrdinalIgnoreCase) == true) return "ongoing";
        return status switch { 0 => "complete", 1 => "ongoing", _ => "unknown" };
    }

    record WuxiaSearchResponse(
        [property: JsonPropertyName("items")] List<WuxiaItem>? Items
    );

    record WuxiaItem(
        [property: JsonPropertyName("id")]       int? Id,
        [property: JsonPropertyName("slug")]     string? Slug,
        [property: JsonPropertyName("name")]     string? Name,
        [property: JsonPropertyName("coverUrl")] string? CoverUrl,
        [property: JsonPropertyName("status")]   int? Status,
        [property: JsonPropertyName("tags")]     List<string>? Tags
    );
}
