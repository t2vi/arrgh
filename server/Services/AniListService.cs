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
    int? Year,
    bool IsAdult = false
);

public class AniListService(IHttpClientFactory httpFactory)
{
    const string Endpoint = "https://graphql.anilist.co";

    const string Query = """
        query ($search: String, $isAdult: Boolean) {
          Page(perPage: 25) {
            media(search: $search, type: MANGA, isAdult: $isAdult) {
              id
              title { romaji english }
              isAdult
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

    const string TrendingQuery = """
        query ($country: CountryCode, $isAdult: Boolean, $perPage: Int) {
          Page(perPage: $perPage) {
            media(type: MANGA, sort: [TRENDING_DESC], countryOfOrigin: $country, isAdult: $isAdult) {
              id
              title { romaji english }
              isAdult
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

    public async Task<List<AniListSeries>> TrendingAsync(string countryOfOrigin, bool isAdult, int limit = 15)
    {
        var http = httpFactory.CreateClient();
        try
        {
            var resp = await http.PostAsJsonAsync(Endpoint, new
            {
                query = TrendingQuery,
                variables = new { country = countryOfOrigin, isAdult, perPage = limit },
            });
            resp.EnsureSuccessStatusCode();
            var body = await resp.Content.ReadAsStringAsync();
            if (string.IsNullOrWhiteSpace(body)) return [];
            var json = JsonSerializer.Deserialize<JsonElement>(body);
            return ParseResponse(json);
        }
        catch { return []; }
    }

    public async Task<List<string>> GetSynonymsAsync(string mediaId)
    {
        if (!int.TryParse(mediaId, out var id)) return [];
        var http = httpFactory.CreateClient();
        const string SynonymsQuery = """
            query ($id: Int) {
              Media(id: $id, type: MANGA) {
                synonyms
                title { romaji english }
              }
            }
            """;
        try
        {
            var resp = await http.PostAsJsonAsync(Endpoint, new { query = SynonymsQuery, variables = new { id } });
            if (!resp.IsSuccessStatusCode) return [];
            var body = await resp.Content.ReadAsStringAsync();
            var json = JsonSerializer.Deserialize<JsonElement>(body);
            var result = new List<string>();
            if (!json.TryGetProperty("data", out var data)) return result;
            if (!data.TryGetProperty("Media", out var media) || media.ValueKind == JsonValueKind.Null) return result;
            if (media.TryGetProperty("synonyms", out var syns) && syns.ValueKind == JsonValueKind.Array)
                foreach (var s in syns.EnumerateArray())
                    if (s.ValueKind == JsonValueKind.String && !string.IsNullOrWhiteSpace(s.GetString()))
                        result.Add(s.GetString()!.Trim());
            if (media.TryGetProperty("title", out var titleEl))
            {
                if (titleEl.TryGetProperty("english", out var en) && en.ValueKind == JsonValueKind.String && !string.IsNullOrWhiteSpace(en.GetString()))
                    result.Add(en.GetString()!);
                if (titleEl.TryGetProperty("romaji", out var ro) && ro.ValueKind == JsonValueKind.String && !string.IsNullOrWhiteSpace(ro.GetString()))
                    result.Add(ro.GetString()!);
            }
            return result.Where(s => !string.IsNullOrWhiteSpace(s)).Distinct().ToList();
        }
        catch { return []; }
    }

    public async Task<List<AniListSeries>> SearchAsync(string q, bool allowExplicit = false)
    {
        var http = httpFactory.CreateClient();

        // Always run a non-adult query. When explicit allowed, also run adult-only and merge.
        var nonAdultTask = FetchPage(http, q, isAdult: false);
        var adultTask = allowExplicit ? FetchPage(http, q, isAdult: true) : Task.FromResult(new List<AniListSeries>());
        await Task.WhenAll(nonAdultTask, adultTask);

        var seen = new HashSet<string>();
        var merged = new List<AniListSeries>();
        foreach (var item in nonAdultTask.Result.Concat(adultTask.Result))
        {
            if (seen.Add(item.SourceId)) merged.Add(item);
        }
        return merged;
    }

    async Task<List<AniListSeries>> FetchPage(HttpClient http, string q, bool isAdult)
    {
        var resp = await http.PostAsJsonAsync(Endpoint, new { query = Query, variables = new { search = q, isAdult } });
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

        var isAdult = item.TryGetProperty("isAdult", out var iaEl) && iaEl.ValueKind == JsonValueKind.True;

        return new AniListSeries(id, title, description, coverUrl, status, contentType, author, year, isAdult);
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
