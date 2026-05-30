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
using Microsoft.EntityFrameworkCore;
using Xunit;

namespace ArrghServer.Tests;

[Trait("Category", TestCategories.Integration)]
public class DiscoverTests
{
    // ── Setup ─────────────────────────────────────────────────────────────────

    static DiscoverFactory NewFactory(
        string muSearchJson = """{"results":[]}""",
        string muReleasesJson = """{"results":[]}""",
        string? muDetailJson = null) =>
        new(muSearchJson, muReleasesJson, muDetailJson);

    void Authorize(HttpClient client, ArrghServer.Data.Models.User user)
    {
        var config = new ConfigurationBuilder()
            .AddInMemoryCollection(new Dictionary<string, string?> { ["JwtSecret"] = AppFactory.JwtSecret })
            .Build();
        var token = Auth.CreateToken(user.Id, user.Username, user.Role, user.AllowExplicit, config);
        client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue("Bearer", token);
    }

    static string MuSearchResponse(params object[] records)
    {
        var hits = records.Select(r => new { record = r });
        return JsonSerializer.Serialize(new { results = hits });
    }

    static object MuRecord(ulong id, string title) => new
    {
        series_id = id,
        title,
        description = (string?)null,
        image = (object?)null,
        type = "Manga",
        year = (string?)null,
        status = "ongoing",
        authors = (object?)null,
        genres = (object?)null,
        associated = (object?)null,
    };

    // ── GET /api/discover ─────────────────────────────────────────────────────

    [Fact]
    public async Task Search_Unauthorized_WithoutToken()
    {
        var (client, _) = NewFactory().CreateClientWithDb();
        var res = await client.GetAsync("/api/discover?q=test");
        Assert.Equal(HttpStatusCode.Unauthorized, res.StatusCode);
    }

    [Fact]
    public async Task Search_ReturnsEmptyList_WhenMuFails()
    {
        var factory = new DiscoverFactory(muSearchThrows: true);
        var (client, db) = factory.CreateClientWithDb();
        var user = await Seed.UserAsync(db, Fake.AdminUser());
        Authorize(client, user);

        var res = await client.GetAsync("/api/discover?q=test");
        Assert.Equal(HttpStatusCode.BadGateway, res.StatusCode);
    }

    [Fact]
    public async Task Search_ReturnsMappedResults()
    {
        var searchJson = MuSearchResponse(MuRecord(123, "Solo Leveling"));
        var (client, db) = NewFactory(muSearchJson: searchJson).CreateClientWithDb();
        var user = await Seed.UserAsync(db, Fake.AdminUser());
        Authorize(client, user);

        var res = await client.GetFromJsonAsync<JsonElement[]>("/api/discover?q=solo+leveling");
        Assert.NotNull(res);
        Assert.Single(res);
        Assert.Equal("123", res[0].GetProperty("mangaupdates_id").GetString());
        Assert.Equal("Solo Leveling", res[0].GetProperty("title").GetString());
        Assert.False(res[0].GetProperty("in_library").GetBoolean());
    }

    [Fact]
    public async Task Search_InLibrary_True_WhenAlreadyAdded()
    {
        var searchJson = MuSearchResponse(MuRecord(42, "Naruto"));
        var (client, db) = NewFactory(muSearchJson: searchJson).CreateClientWithDb();
        var user = await Seed.UserAsync(db, Fake.AdminUser());
        Authorize(client, user);

        // Seed a matching title
        var title = Fake.Title();
        title.MangaupdatesId = "42";
        await Seed.TitleAsync(db, user.Id, title);
        db.ChangeTracker.Clear();

        var res = await client.GetFromJsonAsync<JsonElement[]>("/api/discover?q=naruto");
        Assert.True(res![0].GetProperty("in_library").GetBoolean());
        Assert.Equal(title.Id, res[0].GetProperty("library_id").GetString());
    }

    // ── GET /api/discover/trending ────────────────────────────────────────────

    [Fact]
    public async Task Trending_Unauthorized_WithoutToken()
    {
        var (client, _) = NewFactory().CreateClientWithDb();
        var res = await client.GetAsync("/api/discover/trending");
        Assert.Equal(HttpStatusCode.Unauthorized, res.StatusCode);
    }

    [Fact]
    public async Task Trending_ReturnsEmptyList_WhenMuFails_AndNoCachedData()
    {
        var factory = new DiscoverFactory(muReleasesThrows: true);
        var (client, db) = factory.CreateClientWithDb();
        var user = await Seed.UserAsync(db, Fake.AdminUser());
        Authorize(client, user);

        var res = await client.GetAsync("/api/discover/trending");
        Assert.Equal(HttpStatusCode.BadGateway, res.StatusCode);
    }

    [Fact]
    public async Task Trending_ServesStaleCache_WhenMuFails()
    {
        var factory = new DiscoverFactory(muReleasesThrows: true);
        var (client, db) = factory.CreateClientWithDb();

        // Pre-populate the cache
        var cache = factory.Services.GetRequiredService<TrendingCacheService>();
        cache.Set([new MuSeries(1, "Cached Title", null, null, "ongoing", "manga", null, null, null, [])]);

        var user = await Seed.UserAsync(db, Fake.AdminUser());
        Authorize(client, user);

        var res = await client.GetFromJsonAsync<JsonElement[]>("/api/discover/trending");
        Assert.NotNull(res);
        Assert.Single(res);
        Assert.Equal("Cached Title", res[0].GetProperty("title").GetString());
    }

    // ── POST /api/discover/add ────────────────────────────────────────────────

    [Fact]
    public async Task AddManga_Unauthorized_WithoutToken()
    {
        var (client, _) = NewFactory().CreateClientWithDb();
        var res = await client.PostAsJsonAsync("/api/discover/add",
            new { mangaupdates_id = "1", title = "Test", status = "unknown", content_type = "manga" });
        Assert.Equal(HttpStatusCode.Unauthorized, res.StatusCode);
    }

    [Fact]
    public async Task AddManga_CreatesTitleAndReturnsIt()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = await Seed.UserAsync(db, Fake.AdminUser());
        Authorize(client, user);

        var res = await client.PostAsJsonAsync("/api/discover/add", new
        {
            mangaupdates_id = "999",
            title = "My Hero Academia (Manga)",
            status = "ongoing",
            content_type = "manga",
        });

        Assert.Equal(HttpStatusCode.OK, res.StatusCode);
        var body = await res.Content.ReadFromJsonAsync<JsonElement>();
        Assert.Equal("My Hero Academia", body.GetProperty("title").GetString()); // qualifier stripped
        Assert.Equal("syncing", body.GetProperty("sync_status").GetString());

        db.ChangeTracker.Clear();
        Assert.True(await db.UserTitles.AnyAsync(ut => ut.UserId == user.Id));
    }

    [Fact]
    public async Task AddManga_DuplicateMuId_SubscribesUserAndReturnsExisting()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = await Seed.UserAsync(db, Fake.AdminUser());
        Authorize(client, user);

        // First add
        await client.PostAsJsonAsync("/api/discover/add", new
        {
            mangaupdates_id = "77",
            title = "Berserk",
            status = "ongoing",
            content_type = "manga",
        });

        // Second add — same MU ID
        var res = await client.PostAsJsonAsync("/api/discover/add", new
        {
            mangaupdates_id = "77",
            title = "Berserk",
            status = "ongoing",
            content_type = "manga",
        });

        Assert.Equal(HttpStatusCode.OK, res.StatusCode);
        db.ChangeTracker.Clear();
        Assert.Equal(1, await db.Titles.CountAsync(t => t.MangaupdatesId == "77"));
    }

    [Fact]
    public async Task AddManga_ExplicitTags_SetsIsExplicit()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = await Seed.UserAsync(db, Fake.AdminUser());
        Authorize(client, user);

        var res = await client.PostAsJsonAsync("/api/discover/add", new
        {
            mangaupdates_id = "666",
            title = "Adult Manga",
            status = "complete",
            content_type = "manga",
            tags = "Action,hentai,Ecchi",
        });

        Assert.Equal(HttpStatusCode.OK, res.StatusCode);
        db.ChangeTracker.Clear();
        var title = await db.Titles.FirstAsync(t => t.MangaupdatesId == "666");
        Assert.True(title.IsExplicit);
    }

    // ── Sync log — metadata source (Bug: novel plugins fell through to "unknown") ──

    [Fact]
    public async Task AddManga_WithPluginSource_SyncLogShowsSourceName_NotUnknown()
    {
        // When a title is added with source="boxnovel" (or any plugin key not in the
        // metadata-fetch switch), the sync log must say "Metadata source: boxnovel",
        // NOT "Metadata source: unknown".
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = await Seed.UserAsync(db, Fake.AdminUser());
        Authorize(client, user);

        var res = await client.PostAsJsonAsync("/api/discover/add", new
        {
            source         = "boxnovel",
            source_id      = "a-will-eternal",
            title          = "A Will Eternal",
            status         = "ongoing",
            content_type   = "novel",
            mangaupdates_id = (string?)null,
        });
        Assert.Equal(HttpStatusCode.OK, res.StatusCode);
        var body = await res.Content.ReadFromJsonAsync<System.Text.Json.JsonElement>();
        var titleId = body.GetProperty("id").GetString()!;

        // Wait for background sync to finish
        var deadline = DateTime.UtcNow.AddSeconds(10);
        while (DateTime.UtcNow < deadline)
        {
            db.ChangeTracker.Clear();
            var status = await db.Titles.Where(t => t.Id == titleId).Select(t => t.SyncStatus).FirstAsync();
            if (status is "ready" or "error") break;
            await Task.Delay(100);
        }

        db.ChangeTracker.Clear();
        var logs = await db.SyncLogs.Where(l => l.TitleId == titleId).Select(l => l.Message).ToListAsync();
        Assert.DoesNotContain(logs, m => m.Contains("unknown"));
        Assert.Contains(logs, m => m.Contains("boxnovel"));
    }

    [Theory]
    [InlineData("wuxiaworld")]
    [InlineData("boxnovel")]
    [InlineData("asurascans")]
    [InlineData("manhuafast")]
    [InlineData("mangafire")]
    public async Task AddManga_AllPluginSources_SyncLogShowsSourceName(string source)
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = await Seed.UserAsync(db, Fake.AdminUser());
        Authorize(client, user);

        var res = await client.PostAsJsonAsync("/api/discover/add", new
        {
            source,
            source_id      = "some-id",
            title          = "Test Title",
            status         = "ongoing",
            content_type   = "manga",
            mangaupdates_id = (string?)null,
        });
        var body = await res.Content.ReadFromJsonAsync<System.Text.Json.JsonElement>();
        var titleId = body.GetProperty("id").GetString()!;

        var deadline = DateTime.UtcNow.AddSeconds(10);
        while (DateTime.UtcNow < deadline)
        {
            db.ChangeTracker.Clear();
            var status = await db.Titles.Where(t => t.Id == titleId).Select(t => t.SyncStatus).FirstAsync();
            if (status is "ready" or "error") break;
            await Task.Delay(100);
        }

        db.ChangeTracker.Clear();
        var logs = await db.SyncLogs.Where(l => l.TitleId == titleId).Select(l => l.Message).ToListAsync();
        Assert.DoesNotContain(logs, m => m.Contains("unknown"));
        Assert.Contains(logs, m => m.Contains(source));
    }
}

// ── Test factory ──────────────────────────────────────────────────────────────

public class DiscoverFactory : AppFactory
{
    readonly string _searchJson;
    readonly string _releasesJson;
    readonly string? _detailJson;
    readonly bool _searchThrows;
    readonly bool _releasesThrows;

    public DiscoverFactory(
        string muSearchJson = """{"results":[]}""",
        string muReleasesJson = """{"results":[]}""",
        string? muDetailJson = null,
        bool muSearchThrows = false,
        bool muReleasesThrows = false)
    {
        _searchJson = muSearchJson;
        _releasesJson = muReleasesJson;
        _detailJson = muDetailJson;
        _searchThrows = muSearchThrows;
        _releasesThrows = muReleasesThrows;
    }

    protected override IHost CreateHost(IHostBuilder builder)
    {
        var searchJson = _searchJson;
        var releasesJson = _releasesJson;
        var detailJson = _detailJson;
        var searchThrows = _searchThrows;
        var releasesThrows = _releasesThrows;

        builder.ConfigureServices(services =>
        {
            services.RemoveAll<IHttpClientFactory>();
            services.AddSingleton<IHttpClientFactory>(
                new MuFakeHttpClientFactory(searchJson, releasesJson, detailJson, searchThrows, releasesThrows));
        });
        return base.CreateHost(builder);
    }
}

file class MuFakeHttpClientFactory(
    string searchJson, string releasesJson, string? detailJson,
    bool searchThrows, bool releasesThrows) : IHttpClientFactory
{
    public HttpClient CreateClient(string name) =>
        new(new MuFakeHandler(searchJson, releasesJson, detailJson, searchThrows, releasesThrows));
}

file class MuFakeHandler(
    string searchJson, string releasesJson, string? detailJson,
    bool searchThrows, bool releasesThrows) : HttpMessageHandler
{
    protected override Task<HttpResponseMessage> SendAsync(
        HttpRequestMessage req, CancellationToken ct)
    {
        var path = req.RequestUri?.PathAndQuery ?? "";

        if (path.Contains("/series/search"))
        {
            if (searchThrows) throw new HttpRequestException("MU unavailable");
            return Task.FromResult(Json(searchJson));
        }
        if (path.Contains("/releases/search"))
        {
            if (releasesThrows) throw new HttpRequestException("MU unavailable");
            return Task.FromResult(Json(releasesJson));
        }
        if (path.Contains("/series/"))
        {
            var body = detailJson ?? """{"series_id":1,"title":"Detail","description":null,"image":null,"type":null,"year":null,"status":null,"authors":null,"genres":null}""";
            return Task.FromResult(Json(body));
        }

        // Cover download, plugin-host, etc. — return 200 empty
        return Task.FromResult(new HttpResponseMessage(HttpStatusCode.OK)
        {
            Content = new ByteArrayContent([]),
        });
    }

    static HttpResponseMessage Json(string json) => new(HttpStatusCode.OK)
    {
        Content = new StringContent(json, System.Text.Encoding.UTF8, "application/json"),
    };
}
