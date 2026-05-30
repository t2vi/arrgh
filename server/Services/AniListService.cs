using System.Net.Http.Json;
using System.Text.Json;

namespace ArrghServer.Services;

public record AniListSeries(
    string SourceId,
    string Title,
    string? Description,
    string? CoverUrl,
    string Status,
    string ContentType,
    string? Author,
    int? Year
);

public class AniListService(IHttpClientFactory httpFactory)
{
    const string Endpoint = "https://graphql.anilist.co";

    const string Query = """
        query ($search: String) {
          Page(perPage: 25) {
            media(search: $search, type: MANGA) {
              id
              title { romaji english }
              format
              countryOfOrigin
              status
              description(asHtml: false)
              coverImage { large }
              startDate { year }
              staff(perPage: 5) { nodes { name { full } } }
              synonyms
            }
          }
        }
        """;

    public async Task<List<AniListSeries>> SearchAsync(string q)
    {
        var http = httpFactory.CreateClient();
        var resp = await http.PostAsJsonAsync(Endpoint, new { query = Query, variables = new { search = q } });
        resp.EnsureSuccessStatusCode();
        var body = await resp.Content.ReadAsStringAsync();
        if (string.IsNullOrWhiteSpace(body))
            throw new InvalidOperationException("AniList returned empty response");
        var json = JsonSerializer.Deserialize<JsonElement>(body);
        return ParseResponse(json);
    }

    static List<AniListSeries> ParseResponse(JsonElement root)
    {
        var results = new List<AniListSeries>();
        if (!root.TryGetProperty("data", out var data)) return results;
        if (!data.TryGetProperty("Page", out var page)) return results;
        if (!page.TryGetProperty("media", out var media)) return results;

        foreach (var item in media.EnumerateArray())
        {
            var entry = MapEntry(item);
            if (entry is not null) results.Add(entry);
        }
        return results;
    }

    internal static AniListSeries? MapEntry(JsonElement item)
    {
        var id = item.TryGetProperty("id", out var idEl) ? idEl.GetInt32().ToString() : null;
        if (id is null) return null;

        var format = item.TryGetProperty("format", out var fEl) && fEl.ValueKind == JsonValueKind.String
            ? fEl.GetString() : null;
        var country = item.TryGetProperty("countryOfOrigin", out var cEl) && cEl.ValueKind == JsonValueKind.String
            ? cEl.GetString() : null;

        var contentType = MapContentType(format, country);

        // We handle manga/manhwa/manhua — skip other formats
        if (contentType == "other") return null;

        string title = "";
        if (item.TryGetProperty("title", out var titleEl))
        {
            if (titleEl.TryGetProperty("english", out var en) && en.ValueKind == JsonValueKind.String && !string.IsNullOrEmpty(en.GetString()))
                title = en.GetString()!;
            else if (titleEl.TryGetProperty("romaji", out var ro) && ro.ValueKind == JsonValueKind.String)
                title = ro.GetString() ?? "";
        }
        if (string.IsNullOrEmpty(title)) return null;

        string? description = null;
        if (item.TryGetProperty("description", out var descEl) && descEl.ValueKind == JsonValueKind.String)
            description = descEl.GetString().NullIfEmpty();

        string? coverUrl = null;
        if (item.TryGetProperty("coverImage", out var coverEl) && coverEl.ValueKind == JsonValueKind.Object)
            if (coverEl.TryGetProperty("large", out var lEl) && lEl.ValueKind == JsonValueKind.String)
                coverUrl = lEl.GetString();

        var statusRaw = item.TryGetProperty("status", out var stEl) && stEl.ValueKind == JsonValueKind.String
            ? stEl.GetString() : null;
        var status = MapStatus(statusRaw);

        int? year = null;
        if (item.TryGetProperty("startDate", out var sdEl) && sdEl.ValueKind == JsonValueKind.Object)
            if (sdEl.TryGetProperty("year", out var yEl) && yEl.ValueKind == JsonValueKind.Number)
                year = yEl.GetInt32();

        string? author = null;
        if (item.TryGetProperty("staff", out var staffEl) && staffEl.ValueKind == JsonValueKind.Object)
            if (staffEl.TryGetProperty("nodes", out var nodesEl) && nodesEl.ValueKind == JsonValueKind.Array)
            {
                var first = nodesEl.EnumerateArray().FirstOrDefault();
                if (first.ValueKind != JsonValueKind.Undefined)
                    if (first.TryGetProperty("name", out var nameEl))
                        if (nameEl.TryGetProperty("full", out var fullEl))
                            author = fullEl.GetString();
            }

        return new AniListSeries(id, title, description, coverUrl, status, contentType, author, year);
    }

    internal static string MapContentType(string? format, string? country) => format?.ToUpperInvariant() switch
    {
        "MANHWA" => "manhwa",
        "MANHUA" => "manhua",
        "MANGA" => country == "KR" ? "manhwa" : country == "CN" || country == "TW" ? "manhua" : "manga",
        "ONE_SHOT" => "manga",
        "NOVEL" or "LIGHT_NOVEL" => "novel",
        _ => "other",
    };

    static string MapStatus(string? s) => s?.ToUpperInvariant() switch
    {
        "FINISHED" => "complete",
        "RELEASING" => "ongoing",
        "NOT_YET_RELEASED" => "upcoming",
        "CANCELLED" => "cancelled",
        _ => "unknown",
    };
}
