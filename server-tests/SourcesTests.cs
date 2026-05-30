using System.Net;
using System.Net.Http.Headers;
using System.Net.Http.Json;
using System.Text.Json;
using ArrghServer.Api;
using ArrghServer.Data.Models;
using Microsoft.EntityFrameworkCore;
using Microsoft.Extensions.Configuration;
using Xunit;

namespace ArrghServer.Tests;

[Trait("Category", TestCategories.Integration)]
public class SourcesTests
{
    static AppFactory NewFactory() => new();

    // ── GET /api/sources ──────────────────────────────────────────────────────

    [Fact]
    public async Task ListSources_ReturnsEmpty_WhenNone()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        Authorize(client, user);

        var res = await client.GetFromJsonAsync<JsonElement>("/api/sources");
        Assert.Equal(0, res.GetArrayLength());
    }

    [Fact]
    public async Task ListSources_ReturnsSources_WithContentTypesArray()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        db.ExternalSources.Add(new ExternalSource
        {
            Id = Guid.NewGuid().ToString(),
            Name = "MangaDex",
            BaseUrl = "http://mangadex.org",
            ContentTypes = "manga,manhwa,manhua",
            Enabled = true,
            IsCommunity = false,
            Priority = 100,
            CreatedAt = DateTime.UtcNow,
        });
        await db.SaveChangesAsync();
        Authorize(client, user);

        var res = await client.GetFromJsonAsync<JsonElement>("/api/sources");
        Assert.Equal(1, res.GetArrayLength());
        var src = res[0];
        Assert.Equal("MangaDex", src.GetProperty("name").GetString());
        Assert.False(src.GetProperty("has_api_key").GetBoolean());
        var ct = src.GetProperty("content_types").EnumerateArray().Select(x => x.GetString()).ToArray();
        Assert.Contains("manga", ct);
        Assert.Contains("manhwa", ct);
        Assert.Contains("manhua", ct);
    }

    [Fact]
    public async Task ListSources_HasApiKey_True_WhenApiKeySet()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        db.ExternalSources.Add(new ExternalSource
        {
            Id = Guid.NewGuid().ToString(),
            Name = "Private",
            BaseUrl = "http://private.example.com",
            ApiKey = "secret-key",
            ContentTypes = "manga",
            Enabled = true,
            IsCommunity = false,
            Priority = 100,
            CreatedAt = DateTime.UtcNow,
        });
        await db.SaveChangesAsync();
        Authorize(client, user);

        var res = await client.GetFromJsonAsync<JsonElement>("/api/sources");
        Assert.True(res[0].GetProperty("has_api_key").GetBoolean());
    }

    [Fact]
    public async Task ListSources_Unauthorized_NoToken()
    {
        var (client, _) = NewFactory().CreateClientWithDb();
        Assert.Equal(HttpStatusCode.Unauthorized,
            (await client.GetAsync("/api/sources")).StatusCode);
    }

    // ── POST /api/sources ─────────────────────────────────────────────────────

    [Fact]
    public async Task AddSource_Forbidden_ForMember()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var member = Fake.MemberUser();
        await Seed.UserAsync(db, member);
        Authorize(client, member);

        var res = await client.PostAsJsonAsync("/api/sources", new { base_url = "http://example.com" });
        Assert.Equal(HttpStatusCode.Forbidden, res.StatusCode);
    }

    [Fact]
    public async Task AddSource_BadGateway_WhenPluginNotPortedYet()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var admin = Fake.AdminUser();
        await Seed.UserAsync(db, admin);
        Authorize(client, admin);

        // Stub returns 502 until plugin system is ported (ADR 0030)
        var res = await client.PostAsJsonAsync("/api/sources", new { base_url = "http://example.com" });
        Assert.Equal(HttpStatusCode.BadGateway, res.StatusCode);
    }

    // ── PATCH /api/sources/{id} ───────────────────────────────────────────────

    [Fact]
    public async Task PatchSource_TogglesEnabled()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        var source = new ExternalSource
        {
            Id = Guid.NewGuid().ToString(), Name = "Test", BaseUrl = "http://test.com",
            ContentTypes = "manga", Enabled = true, IsCommunity = false,
            Priority = 100, CreatedAt = DateTime.UtcNow,
        };
        db.ExternalSources.Add(source);
        await db.SaveChangesAsync();
        Authorize(client, user);

        var res = await client.PatchAsJsonAsync($"/api/sources/{source.Id}", new { enabled = false });
        Assert.Equal(HttpStatusCode.NoContent, res.StatusCode);

        db.ChangeTracker.Clear();
        var updated = await db.ExternalSources.FindAsync(source.Id);
        Assert.False(updated!.Enabled);
    }

    [Fact]
    public async Task PatchSource_NotFound_NonexistentId()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        Authorize(client, user);

        Assert.Equal(HttpStatusCode.NotFound,
            (await client.PatchAsJsonAsync("/api/sources/ghost", new { enabled = false })).StatusCode);
    }

    [Fact]
    public async Task PatchSource_Forbidden_ForMember()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var member = Fake.MemberUser();
        await Seed.UserAsync(db, member);
        Authorize(client, member);

        Assert.Equal(HttpStatusCode.Forbidden,
            (await client.PatchAsJsonAsync("/api/sources/any", new { enabled = false })).StatusCode);
    }

    // ── DELETE /api/sources/{id} ──────────────────────────────────────────────

    [Fact]
    public async Task DeleteSource_NoContent_WhenExists()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        var source = new ExternalSource
        {
            Id = Guid.NewGuid().ToString(), Name = "Test", BaseUrl = "http://test.com",
            ContentTypes = "manga", Enabled = true, IsCommunity = false,
            Priority = 100, CreatedAt = DateTime.UtcNow,
        };
        db.ExternalSources.Add(source);
        await db.SaveChangesAsync();
        Authorize(client, user);

        var res = await client.DeleteAsync($"/api/sources/{source.Id}");
        Assert.Equal(HttpStatusCode.NoContent, res.StatusCode);

        db.ChangeTracker.Clear();
        Assert.Null(await db.ExternalSources.FindAsync(source.Id));
    }

    [Fact]
    public async Task DeleteSource_NotFound_NonexistentId()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        Authorize(client, user);

        Assert.Equal(HttpStatusCode.NotFound,
            (await client.DeleteAsync("/api/sources/ghost")).StatusCode);
    }

    [Fact]
    public async Task DeleteSource_Forbidden_ForMember()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var member = Fake.MemberUser();
        await Seed.UserAsync(db, member);
        Authorize(client, member);

        Assert.Equal(HttpStatusCode.Forbidden,
            (await client.DeleteAsync("/api/sources/any")).StatusCode);
    }

    // ── helpers ───────────────────────────────────────────────────────────────

    void Authorize(HttpClient client, User user)
    {
        var config = new ConfigurationBuilder()
            .AddInMemoryCollection(new Dictionary<string, string?> { ["JwtSecret"] = AppFactory.JwtSecret })
            .Build();
        var token = Auth.CreateToken(user.Id, user.Username, user.Role, user.AllowExplicit, config);
        client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue("Bearer", token);
    }
}

// ── Default seeding contract ─────────────────────────────────────────────────

[Trait("Category", TestCategories.Integration)]
public class DefaultSeedingTests
{
    // These tests verify the Program.cs seeding values — content_types per source.
    // Use SeededFactory so the startup code actually runs the seeding block.

    [Fact]
    public async Task Seeded_MangaDex_ContentTypes_DoesNotInclude_Manhwa()
    {
        // Manhwa chapters must only come from dedicated sources (toonily, asurascans, mangafire).
        // MangaDex uses different title formats for manhwa — including it causes false "no match" errors.
        var (_, db) = new SeededFactory().CreateClientWithDb();
        var mangadex = await db.ExternalSources.FirstOrDefaultAsync(s => s.SourceKey == "mangadex");
        Assert.NotNull(mangadex);
        Assert.DoesNotContain("manhwa", mangadex!.ContentTypes, StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public async Task Seeded_Toonily_ContentTypes_IsManhwaOnly()
    {
        var (_, db) = new SeededFactory().CreateClientWithDb();
        var toonily = await db.ExternalSources.FirstOrDefaultAsync(s => s.SourceKey == "toonily");
        Assert.NotNull(toonily);
        Assert.Contains("manhwa", toonily!.ContentTypes, StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public async Task Seeded_AsuraScans_ContentTypes_IsManhwaOnly()
    {
        var (_, db) = new SeededFactory().CreateClientWithDb();
        var asura = await db.ExternalSources.FirstOrDefaultAsync(s => s.SourceKey == "asurascans");
        Assert.NotNull(asura);
        Assert.Contains("manhwa", asura!.ContentTypes, StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public async Task Seeded_MangaFire_ContentTypes_IncludesManhwa()
    {
        var (_, db) = new SeededFactory().CreateClientWithDb();
        var mf = await db.ExternalSources.FirstOrDefaultAsync(s => s.SourceKey == "mangafire");
        Assert.NotNull(mf);
        Assert.Contains("manhwa", mf!.ContentTypes, StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public async Task Seeded_Manga18fx_IsEnabled_WithManhwaContentType()
    {
        // manga18fx is an explicit manhwa aggregator — must be seeded enabled, manhwa-only, explicit-flagged.
        var (_, db) = new SeededFactory().CreateClientWithDb();
        var src = await db.ExternalSources.FirstOrDefaultAsync(s => s.SourceKey == "manga18fx");
        Assert.NotNull(src);
        Assert.True(src!.Enabled);
        Assert.Contains("manhwa", src.ContentTypes, StringComparison.OrdinalIgnoreCase);
        Assert.True(src.DefaultExplicit);
    }

    [Fact]
    public async Task Seeded_AsuraScans_Priority_IsLastAmongManhwaSources()
    {
        // AsuraScans is CF-protected and frequently down — it should be tried last.
        var (_, db) = new SeededFactory().CreateClientWithDb();
        var sources = await db.ExternalSources
            .Where(s => s.ContentTypes.Contains("manhwa"))
            .ToListAsync();
        var asura = sources.FirstOrDefault(s => s.SourceKey == "asurascans");
        Assert.NotNull(asura);
        var maxOtherPriority = sources.Where(s => s.SourceKey != "asurascans").Max(s => s.Priority);
        Assert.True(asura!.Priority > maxOtherPriority,
            $"asurascans priority ({asura.Priority}) should be > all other manhwa sources (max={maxOtherPriority})");
    }
}

[Trait("Category", TestCategories.Integration)]
public class SourcesPatchTests
{
    void Authorize(HttpClient client, User user)
    {
        var config = new ConfigurationBuilder()
            .AddInMemoryCollection(new Dictionary<string, string?> { ["JwtSecret"] = AppFactory.JwtSecret })
            .Build();
        var token = Auth.CreateToken(user.Id, user.Username, user.Role, user.AllowExplicit, config);
        client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue("Bearer", token);
    }

    [Fact]
    public async Task PatchSource_CanUpdatePriority()
    {
        var (client, db) = new AppFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        var source = new ExternalSource
        {
            Id = Guid.NewGuid().ToString(), Name = "Test", BaseUrl = "http://test.com",
            ContentTypes = "manhwa", Enabled = true, IsCommunity = false,
            Priority = 50, CreatedAt = DateTime.UtcNow,
        };
        db.ExternalSources.Add(source);
        await db.SaveChangesAsync();
        Authorize(client, user);

        var res = await client.PatchAsJsonAsync($"/api/sources/{source.Id}", new { enabled = true, priority = 110 });
        Assert.Equal(HttpStatusCode.NoContent, res.StatusCode);

        db.ChangeTracker.Clear();
        var updated = await db.ExternalSources.FindAsync(source.Id);
        Assert.Equal(110, updated!.Priority);
    }

    [Fact]
    public async Task ListSources_ReturnsPriority_InResponse()
    {
        var (client, db) = new AppFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        db.ExternalSources.Add(new ExternalSource
        {
            Id = Guid.NewGuid().ToString(), Name = "Test", BaseUrl = "http://test.com",
            ContentTypes = "manga", Enabled = true, IsCommunity = false,
            Priority = 42, CreatedAt = DateTime.UtcNow,
        });
        await db.SaveChangesAsync();
        Authorize(client, user);

        var res = await client.GetFromJsonAsync<JsonElement>("/api/sources");
        Assert.Equal(42, res[0].GetProperty("priority").GetInt32());
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

[Trait("Category", TestCategories.Unit)]
public class SourcesLogicTests
{
    // IsAdmin used across Auth and Sources — worth a quick unit test
    // since it guards all admin-only source operations

    [Fact]
    public void IsAdmin_True_ForAdminRole()
    {
        var principal = TokenHelper.MakePrincipal("user-1", "admin", allowExplicit: false);
        Assert.True(Sources.IsAdmin(principal));
    }

    [Fact]
    public void IsAdmin_False_ForMemberRole()
    {
        var principal = TokenHelper.MakePrincipal("user-1", "member", allowExplicit: false);
        Assert.False(Sources.IsAdmin(principal));
    }

    [Fact]
    public void IsAdmin_False_WhenNoRoleClaim()
    {
        var principal = TokenHelper.MakePrincipal("user-1", null, allowExplicit: false);
        Assert.False(Sources.IsAdmin(principal));
    }
}
