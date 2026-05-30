using System.Net;
using System.Net.Http.Headers;
using System.Net.Http.Json;
using System.Text.Json;
using ArrghServer.Api;
using Microsoft.Extensions.Configuration;
using Xunit;

namespace ArrghServer.Tests;

[Trait("Category", TestCategories.Integration)]
public class LogsTests
{
    static AppFactory NewFactory() => new();

    void Authorize(HttpClient client, ArrghServer.Data.Models.User user)
    {
        var config = new ConfigurationBuilder()
            .AddInMemoryCollection(new Dictionary<string, string?> { ["JwtSecret"] = AppFactory.JwtSecret })
            .Build();
        var token = Auth.CreateToken(user.Id, user.Username, user.Role, user.AllowExplicit, config);
        client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue("Bearer", token);
    }

    // ── GET /api/logs ─────────────────────────────────────────────────────────

    [Fact]
    public async Task GetLogs_ReturnsEmptyArray_WhenNoEntries()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var admin = await Seed.UserAsync(db, Fake.AdminUser());
        Authorize(client, admin);

        var res = await client.GetFromJsonAsync<JsonElement[]>("/api/logs");
        Assert.NotNull(res);
        Assert.Empty(res);
    }

    [Fact]
    public async Task GetLogs_Unauthorized_WithoutToken()
    {
        var (client, _) = NewFactory().CreateClientWithDb();
        var res = await client.GetAsync("/api/logs");
        Assert.Equal(HttpStatusCode.Unauthorized, res.StatusCode);
    }

    [Fact]
    public async Task GetLogs_MemberCanAccess()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var member = await Seed.UserAsync(db, Fake.MemberUser());
        Authorize(client, member);

        var res = await client.GetAsync("/api/logs");
        Assert.Equal(HttpStatusCode.OK, res.StatusCode);
    }

    // ── GET /api/logs/level ───────────────────────────────────────────────────

    [Fact]
    public async Task GetLevel_ReturnsDefaultInfoLevel()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var admin = await Seed.UserAsync(db, Fake.AdminUser());
        Authorize(client, admin);

        var res = await client.GetFromJsonAsync<JsonElement>("/api/logs/level");
        Assert.Equal("INFO", res.GetProperty("level").GetString());
    }

    [Fact]
    public async Task GetLevel_Unauthorized_WithoutToken()
    {
        var (client, _) = NewFactory().CreateClientWithDb();
        var res = await client.GetAsync("/api/logs/level");
        Assert.Equal(HttpStatusCode.Unauthorized, res.StatusCode);
    }

    // ── PATCH /api/logs/level ─────────────────────────────────────────────────

    [Fact]
    public async Task SetLevel_Admin_UpdatesLevel()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var admin = await Seed.UserAsync(db, Fake.AdminUser());
        Authorize(client, admin);

        var patch = await client.PatchAsJsonAsync("/api/logs/level", new { level = "debug" });
        Assert.Equal(HttpStatusCode.NoContent, patch.StatusCode);

        var res = await client.GetFromJsonAsync<JsonElement>("/api/logs/level");
        Assert.Equal("DEBUG", res.GetProperty("level").GetString());
    }

    [Fact]
    public async Task SetLevel_Admin_PersistsAcrossRequests()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var admin = await Seed.UserAsync(db, Fake.AdminUser());
        Authorize(client, admin);

        await client.PatchAsJsonAsync("/api/logs/level", new { level = "warn" });

        var res = await client.GetFromJsonAsync<JsonElement>("/api/logs/level");
        Assert.Equal("WARN", res.GetProperty("level").GetString());
    }

    [Fact]
    public async Task SetLevel_Member_Forbidden()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var member = await Seed.UserAsync(db, Fake.MemberUser());
        Authorize(client, member);

        var res = await client.PatchAsJsonAsync("/api/logs/level", new { level = "debug" });
        Assert.Equal(HttpStatusCode.Forbidden, res.StatusCode);
    }

    [Fact]
    public async Task SetLevel_Unauthorized_WithoutToken()
    {
        var (client, _) = NewFactory().CreateClientWithDb();
        var res = await client.PatchAsJsonAsync("/api/logs/level", new { level = "debug" });
        Assert.Equal(HttpStatusCode.Unauthorized, res.StatusCode);
    }

    [Fact]
    public async Task SetLevel_Admin_UnprocessableEntity_InvalidLevel()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var admin = await Seed.UserAsync(db, Fake.AdminUser());
        Authorize(client, admin);

        var res = await client.PatchAsJsonAsync("/api/logs/level", new { level = "trace" });
        Assert.Equal(HttpStatusCode.UnprocessableEntity, res.StatusCode);
    }
}
