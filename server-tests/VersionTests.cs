using System.Net.Http.Json;
using System.Text.Json;
using ArrghServer.Api;
using Xunit;

namespace ArrghServer.Tests;

[Trait("Category", TestCategories.Integration)]
public class VersionTests
{
    static AppFactory NewFactory() => new();

    // ── GET /api/version ──────────────────────────────────────────────────────

    [Fact]
    public async Task GetVersion_ReturnsCurrentVersion()
    {
        var (client, _) = NewFactory().CreateClientWithDb();
        var res = await client.GetFromJsonAsync<JsonElement>("/api/version");

        var current = res.GetProperty("current").GetString();
        Assert.NotNull(current);
        Assert.Matches(@"^\d+\.\d+\.\d+", current);
    }

    [Fact]
    public async Task GetVersion_NoAuthRequired()
    {
        var (client, _) = NewFactory().CreateClientWithDb();
        var res = await client.GetAsync("/api/version");
        Assert.Equal(System.Net.HttpStatusCode.OK, res.StatusCode);
    }

    [Fact]
    public async Task GetVersion_NoUpdateAvailable_LatestAndUrlAreNull()
    {
        // UpdateCache starts empty (no background checker in tests), so latest/release_url are null
        var (client, _) = NewFactory().CreateClientWithDb();
        var res = await client.GetFromJsonAsync<JsonElement>("/api/version");

        Assert.Equal(JsonValueKind.Null, res.GetProperty("latest").ValueKind);
        Assert.Equal(JsonValueKind.Null, res.GetProperty("release_url").ValueKind);
    }

    [Fact]
    public async Task GetVersion_UpdateAvailable_ReturnsLatestAndUrl()
    {
        var factory = NewFactory();
        var (client, _) = factory.CreateClientWithDb();

        // Inject a newer version into the cache
        var cache = (UpdateCache)factory.Services.GetService(typeof(UpdateCache))!;
        cache.Set("99.0.0", "https://github.com/t2vi/arrgh/releases/tag/v99.0.0");

        var res = await client.GetFromJsonAsync<JsonElement>("/api/version");
        Assert.Equal("99.0.0", res.GetProperty("latest").GetString());
        Assert.Equal("https://github.com/t2vi/arrgh/releases/tag/v99.0.0",
            res.GetProperty("release_url").GetString());
    }
}
