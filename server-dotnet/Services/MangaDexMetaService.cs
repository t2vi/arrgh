using System.Net.Http.Json;
using System.Text.Json;

namespace ArrghServer.Services;

public record MangaDexSeries(
    string SourceId,
    string Title,
    string? Description,
    string? CoverUrl,
    string Status,
    string ContentType,
    string? Author,
    int? Year
);

public class MangaDexMetaService(IHttpClientFactory httpFactory)
{
    const string Base = "https://api.mangadex.org";

    public async Task<List<MangaDexSeries>> SearchAsync(string q)
    {
        var http = httpFactory.CreateClient();
        var url = $"{Base}/manga?title={Uri.EscapeDataString(q)}&limit=25&contentRating[]=safe&contentRating[]=suggestive&includes[]=author&order[relevance]=desc";
        var resp = await http.GetAsync(url);
        resp.EnsureSuccessStatusCode();
        var body = await resp.Content.ReadAsStringAsync();
        if (string.IsNullOrWhiteSpace(body))
            throw new InvalidOperationException("MangaDex returned empty response");
        var json = JsonSerializer.Deserialize<JsonElement>(body);
        return ParseResponse(json);
    }

    static List<MangaDexSeries> ParseResponse(JsonElement root)
    {
        var results = new List<MangaDexSeries>();
        if (!root.TryGetProperty("data", out var data)) return results;

        foreach (var item in data.EnumerateArray())
        {
            var entry = MapEntry(item);
            if (entry is not null) results.Add(entry);
        }
        return results;
    }

    internal static MangaDexSeries? MapEntry(JsonElement item)
    {
        var id = item.TryGetProperty("id", out var idEl) && idEl.ValueKind == JsonValueKind.String
            ? idEl.GetString() : null;
        if (id is null) return null;

        if (!item.TryGetProperty("attributes", out var attrs)) return null;

        var origLang = attrs.TryGetProperty("originalLanguage", out var langEl) && langEl.ValueKind == JsonValueKind.String
            ? langEl.GetString() : null;

        var contentType = MapContentType(origLang);
        if (contentType != "manhua") return null; // MangaDex authority only for manhua

        // Title: prefer English, fallback to first available
        string? title = null;
        if (attrs.TryGetProperty("title", out var titleEl) && titleEl.ValueKind == JsonValueKind.Object)
        {
            if (titleEl.TryGetProperty("en", out var en) && en.ValueKind == JsonValueKind.String)
                title = en.GetString();
            if (string.IsNullOrEmpty(title))
                foreach (var prop in titleEl.EnumerateObject())
                    if (prop.Value.ValueKind == JsonValueKind.String && !string.IsNullOrEmpty(prop.Value.GetString()))
                    {
                        title = prop.Value.GetString();
                        break;
                    }
        }
        if (string.IsNullOrEmpty(title)) return null;

        string? description = null;
        if (attrs.TryGetProperty("description", out var descEl) && descEl.ValueKind == JsonValueKind.Object)
            if (descEl.TryGetProperty("en", out var en) && en.ValueKind == JsonValueKind.String)
                description = en.GetString().NullIfEmpty();

        var statusRaw = attrs.TryGetProperty("status", out var stEl) && stEl.ValueKind == JsonValueKind.String
            ? stEl.GetString() : null;
        var status = MapStatus(statusRaw);

        int? year = null;
        if (attrs.TryGetProperty("year", out var yEl) && yEl.ValueKind == JsonValueKind.Number)
            year = yEl.GetInt32();

        string? author = null;
        if (item.TryGetProperty("relationships", out var rels) && rels.ValueKind == JsonValueKind.Array)
            foreach (var rel in rels.EnumerateArray())
            {
                if (!rel.TryGetProperty("type", out var typeEl) || typeEl.GetString() != "author") continue;
                if (rel.TryGetProperty("attributes", out var relAttrs) && relAttrs.ValueKind == JsonValueKind.Object)
                    if (relAttrs.TryGetProperty("name", out var nameEl) && nameEl.ValueKind == JsonValueKind.String)
                    {
                        author = nameEl.GetString();
                        break;
                    }
            }

        return new MangaDexSeries(id, title, description, null, status, contentType, author, year);
    }

    internal static string MapContentType(string? origLang) => origLang switch
    {
        "zh" or "zh-hk" or "zh-ro" => "manhua",
        _ => "other",
    };

    static string MapStatus(string? s) => s?.ToLowerInvariant() switch
    {
        "completed" => "complete",
        "ongoing" => "ongoing",
        "cancelled" => "cancelled",
        "hiatus" => "hiatus",
        _ => "unknown",
    };
}
