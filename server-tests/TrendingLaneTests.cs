using System.Net;
using System.Net.Http.Headers;
using System.Net.Http.Json;
using System.Text.Json;
using ArrghServer.Api;
using ArrghServer.Services;
using Microsoft.AspNetCore.Hosting;
using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Microsoft.Extensions.Hosting;
using Xunit;

namespace ArrghServer.Tests;

// TDD: ADR 0032 — Trending Lane Fan-Out.
// Tests define the new per-lane contract. All FAIL until implementation.
// Checklist:
//   - TrendingCacheService keyed by lane (List<DiscoverResult> not List<MuSeries>)
//   - AniListService.TrendingAsync(country, isAdult, limit)
//   - Routes: /trending/manga, /trending/manhwa, /trending/manhua, /trending/adult-manhwa
//   - /trending/adult-manhwa → 403 when !allow_explicit
//   - All lanes return [] (not 502) when source fails and no stale

[Trait("Category", TestCategories.Integration)]
public class TrendingLaneTests
{
    static TrendingLaneFactory NewFactory(
        string muReleasesJson  = """{"results":[]}""",
        string anilistJson     = """{"data":{"Page":{"media":[]}}}""",
        string? muDetailJson   = null,
        bool muReleasesThrows  = false,
        bool anilistThrows     = false)
        => new(muReleasesJson, anilistJson, muDetailJson, muReleasesThrows, anilistThrows);

    void Authorize(HttpClient client, ArrghServer.Data.Models.User user, bool allowExplicit = false)
    {
        var config = new ConfigurationBuilder()
            .AddInMemoryCollection(new Dictionary<string, string?> { ["JwtSecret"] = AppFactory.JwtSecret })
            .Build();
        var token = Auth.CreateToken(user.Id, user.Username, user.Role, allowExplicit, config);
        client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue("Bearer", token);
    }

    static DiscoverResult MakeResult(string title, string contentType, bool isExplicit = false) =>
        new() { MangaupdatesId = "1", Title = title, Status = "ongoing",
                ContentType = contentType, Source = "test", IsExplicit = isExplicit };

    // ── /trending/manga ───────────────────────────────────────────────────────

    [Fact]
    public async Task TrendingManga_Unauthorized_WithoutToken()
    {
        var (client, _) = NewFactory().CreateClientWithDb();
        var res = await client.GetAsync("/api/discover/trending/manga");
        Assert.Equal(HttpStatusCode.Unauthorized, res.StatusCode);
    }

    [Fact]
    public async Task TrendingManga_ReturnsMuResults_MappedToDiscoverResult()
    {
        var releasesJson = """{"results":[{"metadata":{"series":{"series_id":1}}}]}""";
        var detailJson = """
            {"series_id":1,"title":"Berserk","type":"Manga","status":"ongoing",
             "description":null,"image":null,"year":null,"authors":null,"genres":null}
            """;
        var factory = NewFactory(releasesJson, muDetailJson: detailJson);
        var (client, db) = factory.CreateClientWithDb();
        var user = await Seed.UserAsync(db, Fake.AdminUser());
        Authorize(client, user);

        var res = await client.GetFromJsonAsync<JsonElement[]>("/api/discover/trending/manga");
        Assert.NotNull(res);
        Assert.NotEmpty(res);
        Assert.Equal("Berserk", res[0].GetProperty("title").GetString());
        Assert.Equal("manga", res[0].GetProperty("content_type").GetString());
    }

    [Fact]
    public async Task TrendingManga_ReturnsAtMostSixResults()    
    {
        var factory = NewFactory();
        var (client, db) = factory.CreateClientWithDb();
        var cache = factory.Services.GetRequiredService<TrendingCacheService>();
        // Seed 10 items in cache — response must be capped at 6
        var items = Enumerable.Range(1, 10)
            .Select(i => new DiscoverResult
            {
                MangaupdatesId = i.ToString(), Title = $"Title {i}",
                Status = "ongoing", ContentType = "manga", Source = "mangaupdates",
            }).ToList();
        cache.Set("manga", items);
        var user = await Seed.UserAsync(db, Fake.AdminUser());
        Authorize(client, user);

        var res = await client.GetFromJsonAsync<JsonElement[]>("/api/discover/trending/manga");
        Assert.NotNull(res);
        Assert.True(res.Length <= 6, $"Expected ≤6 results, got {res.Length}");
    }

    [Fact]
    public async Task TrendingManga_ServesStaleCache_WhenMuFails()
    {
        var factory = NewFactory(muReleasesThrows: true);
        var (client, db) = factory.CreateClientWithDb();
        var cache = factory.Services.GetRequiredService<TrendingCacheService>();
        cache.Set("manga", [MakeResult("Cached Manga", "manga")]);

        var user = await Seed.UserAsync(db, Fake.AdminUser());
        Authorize(client, user);

        var res = await client.GetFromJsonAsync<JsonElement[]>("/api/discover/trending/manga");
        Assert.NotNull(res);
        Assert.Single(res);
        Assert.Equal("Cached Manga", res[0].GetProperty("title").GetString());
    }

    [Fact]
    public async Task TrendingManga_ReturnsEmptyArray_WhenMuFailsAndNoStale()
    {
        var (client, db) = NewFactory(muReleasesThrows: true).CreateClientWithDb();
        var user = await Seed.UserAsync(db, Fake.AdminUser());
        Authorize(client, user);

        var res = await client.GetAsync("/api/discover/trending/manga");
        Assert.Equal(HttpStatusCode.OK, res.StatusCode);
        var body = await res.Content.ReadFromJsonAsync<JsonElement[]>();
        Assert.NotNull(body);
        Assert.Empty(body);
    }

    // ── /trending/manhwa ──────────────────────────────────────────────────────

    [Fact]
    public async Task TrendingManhwa_Unauthorized_WithoutToken()
    {
        var (client, _) = NewFactory().CreateClientWithDb();
        var res = await client.GetAsync("/api/discover/trending/manhwa");
        Assert.Equal(HttpStatusCode.Unauthorized, res.StatusCode);
    }

    [Fact]
    public async Task TrendingManhwa_ReturnsAniListKrResults()
    {
        var anilist = AniListTrendingResponse(new AniListMediaTrending(1, new AniListTitleTrending("Solo Leveling"), "MANGA", "KR"));
        var (client, db) = NewFactory(anilistJson: anilist).CreateClientWithDb();
        var user = await Seed.UserAsync(db, Fake.AdminUser());
        Authorize(client, user);

        var res = await client.GetFromJsonAsync<JsonElement[]>("/api/discover/trending/manhwa");
        Assert.NotNull(res);
        Assert.NotEmpty(res);
        Assert.Equal("Solo Leveling", res[0].GetProperty("title").GetString());
        Assert.Equal("manhwa", res[0].GetProperty("content_type").GetString());
    }

    [Fact]
    public async Task TrendingManhwa_ServesStale_WhenAniListFails()
    {
        var factory = NewFactory(anilistThrows: true);
        var (client, db) = factory.CreateClientWithDb();
        var cache = factory.Services.GetRequiredService<TrendingCacheService>();
        cache.Set("manhwa", [MakeResult("Cached Manhwa", "manhwa")]);

        var user = await Seed.UserAsync(db, Fake.AdminUser());
        Authorize(client, user);

        var res = await client.GetFromJsonAsync<JsonElement[]>("/api/discover/trending/manhwa");
        Assert.NotNull(res);
        Assert.Single(res);
        Assert.Equal("Cached Manhwa", res[0].GetProperty("title").GetString());
    }

    [Fact]
    public async Task TrendingManhwa_ReturnsEmptyArray_WhenAniListFailsAndNoStale()
    {
        var (client, db) = NewFactory(anilistThrows: true).CreateClientWithDb();
        var user = await Seed.UserAsync(db, Fake.AdminUser());
        Authorize(client, user);

        var res = await client.GetAsync("/api/discover/trending/manhwa");
        Assert.Equal(HttpStatusCode.OK, res.StatusCode);
        var body = await res.Content.ReadFromJsonAsync<JsonElement[]>();
        Assert.NotNull(body);
        Assert.Empty(body);
    }

    // ── /trending/manhua ──────────────────────────────────────────────────────

    [Fact]
    public async Task TrendingManhua_Unauthorized_WithoutToken()
    {
        var (client, _) = NewFactory().CreateClientWithDb();
        var res = await client.GetAsync("/api/discover/trending/manhua");
        Assert.Equal(HttpStatusCode.Unauthorized, res.StatusCode);
    }

    [Fact]
    public async Task TrendingManhua_ReturnsAniListCnResults()
    {
        var anilist = AniListTrendingResponse(new AniListMediaTrending(2, new AniListTitleTrending("Martial Peak"), "MANGA", "CN"));
        var (client, db) = NewFactory(anilistJson: anilist).CreateClientWithDb();
        var user = await Seed.UserAsync(db, Fake.AdminUser());
        Authorize(client, user);

        var res = await client.GetFromJsonAsync<JsonElement[]>("/api/discover/trending/manhua");
        Assert.NotNull(res);
        Assert.NotEmpty(res);
        Assert.Equal("Martial Peak", res[0].GetProperty("title").GetString());
        Assert.Equal("manhua", res[0].GetProperty("content_type").GetString());
    }

    // ── /trending/adult-manhwa ────────────────────────────────────────────────

    [Fact]
    public async Task TrendingAdultManhwa_Returns403_WhenUserLacksExplicitPermission()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = await Seed.UserAsync(db, Fake.AdminUser());
        Authorize(client, user, allowExplicit: false);

        var res = await client.GetAsync("/api/discover/trending/adult-manhwa");
        Assert.Equal(HttpStatusCode.Forbidden, res.StatusCode);
    }

    [Fact]
    public async Task TrendingAdultManhwa_ReturnsExplicitKrResults_WhenUserHasExplicitPermission()
    {
        var anilist = AniListTrendingResponse(new AniListMediaTrending(3, new AniListTitleTrending("Adult Manhwa Title"), "MANGA", "KR", IsAdult: true));
        var (client, db) = NewFactory(anilistJson: anilist).CreateClientWithDb();
        var user = await Seed.UserAsync(db, Fake.AdminUser());
        Authorize(client, user, allowExplicit: true);

        var res = await client.GetFromJsonAsync<JsonElement[]>("/api/discover/trending/adult-manhwa");
        Assert.NotNull(res);
        Assert.NotEmpty(res);
        Assert.True(res[0].GetProperty("is_explicit").GetBoolean());
    }

    [Fact]
    public async Task TrendingAdultManhwa_ServesStale_WhenAniListFails()
    {
        var factory = NewFactory(anilistThrows: true);
        var (client, db) = factory.CreateClientWithDb();
        var cache = factory.Services.GetRequiredService<TrendingCacheService>();
        cache.Set("adult-manhwa", [MakeResult("Cached Adult Manhwa", "manhwa", isExplicit: true)]);

        var user = await Seed.UserAsync(db, Fake.AdminUser());
        Authorize(client, user, allowExplicit: true);

        var res = await client.GetFromJsonAsync<JsonElement[]>("/api/discover/trending/adult-manhwa");
        Assert.NotNull(res);
        Assert.Single(res);
        Assert.Equal("Cached Adult Manhwa", res[0].GetProperty("title").GetString());
    }

    // ── Cache isolation ───────────────────────────────────────────────────────

    [Fact]
    public async Task TrendingLanes_CacheIndependent_MangaStaleDoesNotLeakToManhwa()
    {
        var factory = NewFactory(muReleasesThrows: true, anilistThrows: true);
        var (client, db) = factory.CreateClientWithDb();
        var cache = factory.Services.GetRequiredService<TrendingCacheService>();
        cache.Set("manga", [MakeResult("Cached Manga", "manga")]);

        var user = await Seed.UserAsync(db, Fake.AdminUser());
        Authorize(client, user);

        var mangaRes = await client.GetFromJsonAsync<JsonElement[]>("/api/discover/trending/manga");
        Assert.NotNull(mangaRes);
        Assert.NotEmpty(mangaRes);

        var manhwaRes = await client.GetFromJsonAsync<JsonElement[]>("/api/discover/trending/manhwa");
        Assert.NotNull(manhwaRes);
        Assert.Empty(manhwaRes);
    }

    // ── AniList fixture helpers ───────────────────────────────────────────────

    record AniListMediaTrending(int Id, AniListTitleTrending Title, string Format,
        string CountryOfOrigin, bool IsAdult = false);
    record AniListTitleTrending(string Romaji, string? English = null);

    static string AniListTrendingResponse(params AniListMediaTrending[] items)
    {
        var mediaList = items.Select(m => new
        {
            id = m.Id,
            title = new { romaji = m.Title.Romaji, english = m.Title.English },
            isAdult = m.IsAdult,
            format = m.Format,
            countryOfOrigin = m.CountryOfOrigin,
            status = "FINISHED",
            description = (string?)null,
            coverImage = new { large = (string?)null },
            startDate = new { year = (int?)null },
            staff = new { nodes = Array.Empty<object>() },
            synonyms = Array.Empty<string>(),
        });
        return JsonSerializer.Serialize(new { data = new { Page = new { media = mediaList } } });
    }
}

// ── TrendingLaneFactory ───────────────────────────────────────────────────────

public class TrendingLaneFactory : AppFactory
{
    readonly string _muReleasesJson;
    readonly string? _muDetailJson;
    readonly string _anilistJson;
    readonly bool _muReleasesThrows;
    readonly bool _anilistThrows;

    public TrendingLaneFactory(
        string muReleasesJson  = """{"results":[]}""",
        string anilistJson     = """{"data":{"Page":{"media":[]}}}""",
        string? muDetailJson   = null,
        bool muReleasesThrows  = false,
        bool anilistThrows     = false)
    {
        _muReleasesJson   = muReleasesJson;
        _anilistJson      = anilistJson;
        _muDetailJson     = muDetailJson;
        _muReleasesThrows = muReleasesThrows;
        _anilistThrows    = anilistThrows;
    }

    protected override IHost CreateHost(IHostBuilder builder)
    {
        var releases = _muReleasesJson;
        var detail   = _muDetailJson;
        var anilist  = _anilistJson;
        var muThrows = _muReleasesThrows;
        var alThrows = _anilistThrows;

        builder.ConfigureServices(services =>
        {
            services.RemoveAll<IHttpClientFactory>();
            services.AddSingleton<IHttpClientFactory>(
                new TrendingLaneFakeFactory(releases, anilist, detail, muThrows, alThrows));
        });
        return base.CreateHost(builder);
    }
}

file class TrendingLaneFakeFactory(
    string muReleasesJson, string anilistJson, string? muDetailJson,
    bool muThrows, bool alThrows) : IHttpClientFactory
{
    public HttpClient CreateClient(string name) =>
        new(new TrendingLaneFakeHandler(muReleasesJson, anilistJson, muDetailJson, muThrows, alThrows));
}

file class TrendingLaneFakeHandler(
    string muReleasesJson, string anilistJson, string? muDetailJson,
    bool muThrows, bool alThrows) : HttpMessageHandler
{
    protected override Task<HttpResponseMessage> SendAsync(
        HttpRequestMessage req, CancellationToken ct)
    {
        var host = req.RequestUri?.Host ?? "";
        var path = req.RequestUri?.PathAndQuery ?? "";

        if (host.Contains("mangaupdates"))
        {
            if (path.Contains("/releases/search"))
            {
                if (muThrows) throw new HttpRequestException("MU unavailable");
                return Task.FromResult(Json(muReleasesJson));
            }
            if (path.Contains("/series/"))
            {
                var body = muDetailJson ??
                    "{\"series_id\":1,\"title\":\"Test\",\"type\":\"Manga\",\"status\":\"ongoing\",\"description\":null,\"image\":null,\"year\":null,\"authors\":null,\"genres\":null}";
                return Task.FromResult(Json(body));
            }
        }

        if (host.Contains("anilist"))
        {
            if (alThrows) throw new HttpRequestException("AniList unavailable");
            return Task.FromResult(Json(anilistJson));
        }

        return Task.FromResult(new HttpResponseMessage(System.Net.HttpStatusCode.OK)
            { Content = new StringContent("[]") });
    }

    static HttpResponseMessage Json(string json) => new(System.Net.HttpStatusCode.OK)
        { Content = new StringContent(json, System.Text.Encoding.UTF8, "application/json") };
}
