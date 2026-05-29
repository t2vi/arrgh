using System.Net;
using System.Net.Http.Headers;
using System.Net.Http.Json;
using System.Text.Json;
using ArrghServer.Api;
using ArrghServer.Data.Models;
using Microsoft.AspNetCore.Hosting;
using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Microsoft.EntityFrameworkCore;
using Microsoft.Extensions.Hosting;
using Xunit;

namespace ArrghServer.Tests;

// ── Integration tests ─────────────────────────────────────────────────────────

[Trait("Category", TestCategories.Integration)]
public class PluginsTests : IDisposable
{
    // Temp file backed plugin index for tests
    readonly string _indexFile = Path.GetTempFileName();

    public PluginsTests()
    {
        var index = JsonSerializer.Serialize(new[]
        {
            new
            {
                id = "mangadex",
                name = "MangaDex",
                description = "MangaDex source",
                version = "1.0.0",
                download_url = "http://fake-cdn/mangadex.js",
                bundled = false,
                default_explicit = false,
                content_types = new[] { "manga", "manhwa" },
            },
            new
            {
                id = "no-url-plugin",
                name = "NoUrl",
                description = (string?)null,
                version = "1.0.0",
                download_url = "",
                bundled = false,
                default_explicit = false,
                content_types = new[] { "manga" },
            },
        });
        File.WriteAllText(_indexFile, index);
    }

    public void Dispose()
    {
        if (File.Exists(_indexFile)) File.Delete(_indexFile);
    }

    PluginsFactory NewFactory(HttpStatusCode pluginHostStatus = HttpStatusCode.OK) =>
        new(_indexFile, pluginHostStatus);

    void Authorize(HttpClient client, User user)
    {
        var config = new ConfigurationBuilder()
            .AddInMemoryCollection(new Dictionary<string, string?> { ["JwtSecret"] = AppFactory.JwtSecret })
            .Build();
        var token = Auth.CreateToken(user.Id, user.Username, user.Role, user.AllowExplicit, config);
        client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue("Bearer", token);
    }

    // ── GET /api/plugins/index ────────────────────────────────────────────────

    [Fact]
    public async Task GetIndex_ReturnsEntries()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var admin = await Seed.UserAsync(db, Fake.AdminUser());
        Authorize(client, admin);

        var res = await client.GetFromJsonAsync<JsonElement[]>("/api/plugins/index");
        Assert.NotNull(res);
        Assert.Equal(2, res.Length);
        Assert.Equal("mangadex", res[0].GetProperty("id").GetString());
    }

    [Fact]
    public async Task GetIndex_Unauthorized_WithoutToken()
    {
        var (client, _) = NewFactory().CreateClientWithDb();
        var res = await client.GetAsync("/api/plugins/index");
        Assert.Equal(HttpStatusCode.Unauthorized, res.StatusCode);
    }

    [Fact]
    public async Task GetIndex_MemberCanAccess()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var member = await Seed.UserAsync(db, Fake.MemberUser());
        Authorize(client, member);

        var res = await client.GetAsync("/api/plugins/index");
        Assert.Equal(HttpStatusCode.OK, res.StatusCode);
    }

    // ── POST /api/plugins/install ─────────────────────────────────────────────

    [Fact]
    public async Task InstallPlugin_Unauthorized_WithoutToken()
    {
        var (client, _) = NewFactory().CreateClientWithDb();
        var res = await client.PostAsJsonAsync("/api/plugins/install", new { plugin_id = "mangadex" });
        Assert.Equal(HttpStatusCode.Unauthorized, res.StatusCode);
    }

    [Fact]
    public async Task InstallPlugin_Forbidden_ForMember()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var member = await Seed.UserAsync(db, Fake.MemberUser());
        Authorize(client, member);

        var res = await client.PostAsJsonAsync("/api/plugins/install", new { plugin_id = "mangadex" });
        Assert.Equal(HttpStatusCode.Forbidden, res.StatusCode);
    }

    [Fact]
    public async Task InstallPlugin_NotFound_UnknownPlugin()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var admin = await Seed.UserAsync(db, Fake.AdminUser());
        Authorize(client, admin);

        var res = await client.PostAsJsonAsync("/api/plugins/install", new { plugin_id = "nonexistent" });
        Assert.Equal(HttpStatusCode.NotFound, res.StatusCode);
    }

    [Fact]
    public async Task InstallPlugin_UnprocessableEntity_NoDownloadUrl()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var admin = await Seed.UserAsync(db, Fake.AdminUser());
        Authorize(client, admin);

        var res = await client.PostAsJsonAsync("/api/plugins/install", new { plugin_id = "no-url-plugin" });
        Assert.Equal(HttpStatusCode.UnprocessableEntity, res.StatusCode);
    }

    [Fact]
    public async Task InstallPlugin_Conflict_WhenAlreadyInstalled()
    {
        var factory = NewFactory();
        var (client, db) = factory.CreateClientWithDb();
        var admin = await Seed.UserAsync(db, Fake.AdminUser());
        Authorize(client, admin);

        // Seed an existing source matching the effective URL
        var pluginHostUrl = factory.Services.GetRequiredService<IConfiguration>()["PluginHostUrl"]!;
        db.ExternalSources.Add(new ExternalSource
        {
            Id = Guid.NewGuid().ToString(),
            Name = "MangaDex",
            BaseUrl = $"{pluginHostUrl.TrimEnd('/')}/mangadex",
            ContentTypes = "manga",
            IsCommunity = true,
            CreatedAt = DateTime.UtcNow,
        });
        await db.SaveChangesAsync();
        db.ChangeTracker.Clear();

        var res = await client.PostAsJsonAsync("/api/plugins/install", new { plugin_id = "mangadex" });
        Assert.Equal(HttpStatusCode.Conflict, res.StatusCode);
    }

    [Fact]
    public async Task InstallPlugin_BadGateway_WhenPluginHostFails()
    {
        var (client, db) = NewFactory(HttpStatusCode.InternalServerError).CreateClientWithDb();
        var admin = await Seed.UserAsync(db, Fake.AdminUser());
        Authorize(client, admin);

        var res = await client.PostAsJsonAsync("/api/plugins/install", new { plugin_id = "mangadex" });
        Assert.Equal(HttpStatusCode.BadGateway, res.StatusCode);
    }

    [Fact]
    public async Task InstallPlugin_Created_OnSuccess()
    {
        var (client, db) = NewFactory(HttpStatusCode.OK).CreateClientWithDb();
        var admin = await Seed.UserAsync(db, Fake.AdminUser());
        Authorize(client, admin);

        var res = await client.PostAsJsonAsync("/api/plugins/install", new { plugin_id = "mangadex" });
        Assert.Equal(HttpStatusCode.Created, res.StatusCode);

        db.ChangeTracker.Clear();
        Assert.True(await db.ExternalSources.AnyAsync(s => s.Name == "MangaDex" && s.IsCommunity));
    }

    // ── DELETE /api/plugins/{id} ──────────────────────────────────────────────

    [Fact]
    public async Task DeletePlugin_Unauthorized_WithoutToken()
    {
        var (client, _) = NewFactory().CreateClientWithDb();
        var res = await client.DeleteAsync("/api/plugins/mangadex");
        Assert.Equal(HttpStatusCode.Unauthorized, res.StatusCode);
    }

    [Fact]
    public async Task DeletePlugin_Forbidden_ForMember()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var member = await Seed.UserAsync(db, Fake.MemberUser());
        Authorize(client, member);

        var res = await client.DeleteAsync("/api/plugins/mangadex");
        Assert.Equal(HttpStatusCode.Forbidden, res.StatusCode);
    }

    [Fact]
    public async Task DeletePlugin_NotFound_Unknown()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var admin = await Seed.UserAsync(db, Fake.AdminUser());
        Authorize(client, admin);

        var res = await client.DeleteAsync("/api/plugins/nonexistent");
        Assert.Equal(HttpStatusCode.NotFound, res.StatusCode);
    }

    [Fact]
    public async Task DeletePlugin_Forbidden_NonCommunitySource()
    {
        var factory = NewFactory();
        var (client, db) = factory.CreateClientWithDb();
        var admin = await Seed.UserAsync(db, Fake.AdminUser());
        Authorize(client, admin);

        var pluginHostUrl = factory.Services.GetRequiredService<IConfiguration>()["PluginHostUrl"]!;
        db.ExternalSources.Add(new ExternalSource
        {
            Id = Guid.NewGuid().ToString(),
            Name = "MangaDex",
            BaseUrl = $"{pluginHostUrl.TrimEnd('/')}/mangadex",
            ContentTypes = "manga",
            IsCommunity = false, // bundled, not deletable
            CreatedAt = DateTime.UtcNow,
        });
        await db.SaveChangesAsync();
        db.ChangeTracker.Clear();

        var res = await client.DeleteAsync("/api/plugins/mangadex");
        Assert.Equal(HttpStatusCode.Forbidden, res.StatusCode);
    }

    [Fact]
    public async Task DeletePlugin_NoContent_RemovesSource()
    {
        var factory = NewFactory(HttpStatusCode.OK);
        var (client, db) = factory.CreateClientWithDb();
        var admin = await Seed.UserAsync(db, Fake.AdminUser());
        Authorize(client, admin);

        var pluginHostUrl = factory.Services.GetRequiredService<IConfiguration>()["PluginHostUrl"]!;
        var sourceId = Guid.NewGuid().ToString();
        db.ExternalSources.Add(new ExternalSource
        {
            Id = sourceId,
            Name = "MangaDex",
            BaseUrl = $"{pluginHostUrl.TrimEnd('/')}/mangadex",
            ContentTypes = "manga",
            IsCommunity = true,
            CreatedAt = DateTime.UtcNow,
        });
        await db.SaveChangesAsync();
        db.ChangeTracker.Clear();

        var res = await client.DeleteAsync("/api/plugins/mangadex");
        Assert.Equal(HttpStatusCode.NoContent, res.StatusCode);

        db.ChangeTracker.Clear();
        Assert.False(await db.ExternalSources.AnyAsync(s => s.Id == sourceId));
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

[Trait("Category", TestCategories.Unit)]
public class PluginsLogicTests : IDisposable
{
    readonly string _indexFile = Path.GetTempFileName();
    readonly FakeHttpClientFactory _http = new(HttpStatusCode.OK);

    public PluginsLogicTests()
    {
        var index = JsonSerializer.Serialize(new[]
        {
            new { id = "mangadex", name = "MangaDex", version = "1.0.0",
                  download_url = "http://fake/mangadex.js", default_explicit = false,
                  content_types = new[] { "manga" } }
        });
        File.WriteAllText(_indexFile, index);
    }

    public void Dispose()
    {
        if (File.Exists(_indexFile)) File.Delete(_indexFile);
    }

    [Fact]
    public async Task FetchIndex_ReadsFileUrl()
    {
        var entries = await Plugins.FetchIndexAsync($"file://{_indexFile}", _http);
        Assert.NotNull(entries);
        Assert.Single(entries);
        Assert.Equal("mangadex", entries[0].Id);
    }

    [Fact]
    public async Task FetchIndex_MissingFile_ReturnsNull()
    {
        var entries = await Plugins.FetchIndexAsync("file:///nonexistent/path.json", _http);
        Assert.Null(entries);
    }
}

// ── Test helpers ──────────────────────────────────────────────────────────────

/// <summary>
/// AppFactory variant that points PluginIndexUrl at a temp file and intercepts
/// IHttpClientFactory to return a preset status code for plugin-host calls.
/// </summary>
public class PluginsFactory(string indexFile, HttpStatusCode pluginHostStatus = HttpStatusCode.OK)
    : AppFactory
{
    protected override void ConfigureWebHost(Microsoft.AspNetCore.Hosting.IWebHostBuilder builder)
    {
        base.ConfigureWebHost(builder);
        builder.ConfigureAppConfiguration(config =>
            config.AddInMemoryCollection(new Dictionary<string, string?>
            {
                ["PluginIndexUrl"] = $"file://{indexFile}",
                ["PluginHostUrl"] = "http://fake-plugin-host",
            }));
    }

    protected override IHost CreateHost(IHostBuilder builder)
    {
        builder.ConfigureServices(services =>
        {
            services.RemoveAll<IHttpClientFactory>();
            services.AddSingleton<IHttpClientFactory>(new FakeHttpClientFactory(pluginHostStatus));
        });
        return base.CreateHost(builder);
    }
}

public class FakeHttpClientFactory(HttpStatusCode status) : IHttpClientFactory
{
    public HttpClient CreateClient(string name) =>
        new(new FakeHandler(status)) { BaseAddress = null };
}

file class FakeHandler(HttpStatusCode status) : HttpMessageHandler
{
    protected override Task<HttpResponseMessage> SendAsync(
        HttpRequestMessage request, CancellationToken cancellationToken) =>
        Task.FromResult(new HttpResponseMessage(status));
}
