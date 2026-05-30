using System.Net;
using System.Net.Http.Headers;
using System.Net.Http.Json;
using System.Text.Json;
using ArrghServer.Api;
using Microsoft.Extensions.Configuration;
using Xunit;

namespace ArrghServer.Tests;

[Trait("Category", TestCategories.Integration)]
public class AuthTests
{
    // Fresh factory (and DB) per test — mirrors Rust's per-test build_state()
    static AppFactory NewFactory() => new();

    // ── /api/auth/status ─────────────────────────────────────────────────────

    [Fact]
    public async Task Status_NeedsSetup_WhenNoUsers()
    {
        var (client, _) = NewFactory().CreateClientWithDb();
        var res = await client.GetFromJsonAsync<JsonElement>("/api/auth/status");
        Assert.True(res.GetProperty("needs_setup").GetBoolean());
    }

    [Fact]
    public async Task Status_NoSetupNeeded_AfterRegister()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        await Seed.UserAsync(db);

        var res = await client.GetFromJsonAsync<JsonElement>("/api/auth/status");
        Assert.False(res.GetProperty("needs_setup").GetBoolean());
    }

    // ── /api/auth/register ───────────────────────────────────────────────────

    [Fact]
    public async Task Register_CreatesAdminAndReturnsToken()
    {
        var (client, _) = NewFactory().CreateClientWithDb();
        var res = await client.PostAsJsonAsync("/api/auth/register",
            new { username = "admin", password = "secret123" });

        Assert.Equal(HttpStatusCode.OK, res.StatusCode);
        var body = await res.Content.ReadFromJsonAsync<JsonElement>();
        Assert.NotEmpty(body.GetProperty("token").GetString()!);
        Assert.Equal("admin", body.GetProperty("role").GetString());
        Assert.True(body.GetProperty("allow_explicit").GetBoolean());
    }

    [Fact]
    public async Task Register_Forbidden_WhenUsersExist()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        await Seed.UserAsync(db);

        var res = await client.PostAsJsonAsync("/api/auth/register",
            new { username = "other", password = "secret123" });

        Assert.Equal(HttpStatusCode.Forbidden, res.StatusCode);
    }

    [Fact]
    public async Task Register_UnprocessableEntity_ShortPassword()
    {
        var (client, _) = NewFactory().CreateClientWithDb();
        var res = await client.PostAsJsonAsync("/api/auth/register",
            new { username = "admin", password = "abc" });

        Assert.Equal(HttpStatusCode.UnprocessableEntity, res.StatusCode);
    }

    [Fact]
    public async Task Register_UnprocessableEntity_EmptyUsername()
    {
        var (client, _) = NewFactory().CreateClientWithDb();
        var res = await client.PostAsJsonAsync("/api/auth/register",
            new { username = "   ", password = "secret123" });

        Assert.Equal(HttpStatusCode.UnprocessableEntity, res.StatusCode);
    }

    // ── /api/auth/login ──────────────────────────────────────────────────────

    [Fact]
    public async Task Login_ReturnsToken_ValidCredentials()
    {
        var (client, _) = NewFactory().CreateClientWithDb();
        await client.PostAsJsonAsync("/api/auth/register",
            new { username = "admin", password = "secret123" });

        var res = await client.PostAsJsonAsync("/api/auth/login",
            new { username = "admin", password = "secret123" });

        Assert.Equal(HttpStatusCode.OK, res.StatusCode);
        var body = await res.Content.ReadFromJsonAsync<JsonElement>();
        Assert.NotEmpty(body.GetProperty("token").GetString()!);
    }

    [Fact]
    public async Task Login_Unauthorized_WrongPassword()
    {
        var (client, _) = NewFactory().CreateClientWithDb();
        await client.PostAsJsonAsync("/api/auth/register",
            new { username = "admin", password = "secret123" });

        var res = await client.PostAsJsonAsync("/api/auth/login",
            new { username = "admin", password = "wrong" });

        Assert.Equal(HttpStatusCode.Unauthorized, res.StatusCode);
    }

    [Fact]
    public async Task Login_Unauthorized_UnknownUser()
    {
        var (client, _) = NewFactory().CreateClientWithDb();
        var res = await client.PostAsJsonAsync("/api/auth/login",
            new { username = "ghost", password = "secret123" });

        Assert.Equal(HttpStatusCode.Unauthorized, res.StatusCode);
    }

    // ── /api/auth/me ─────────────────────────────────────────────────────────

    [Fact]
    public async Task Me_ReturnsCurrentUser()
    {
        var (client, _) = NewFactory().CreateClientWithDb();
        var token = await RegisterAndGetToken(client, "admin", "secret123");
        client.DefaultRequestHeaders.Authorization =
            new System.Net.Http.Headers.AuthenticationHeaderValue("Bearer", token);

        var res = await client.GetFromJsonAsync<JsonElement>("/api/auth/me");
        Assert.Equal("admin", res.GetProperty("username").GetString());
        Assert.Equal("admin", res.GetProperty("role").GetString());
    }

    [Fact]
    public async Task Me_Unauthorized_NoToken()
    {
        var (client, _) = NewFactory().CreateClientWithDb();
        var res = await client.GetAsync("/api/auth/me");
        Assert.Equal(HttpStatusCode.Unauthorized, res.StatusCode);
    }

    // ── /api/users ───────────────────────────────────────────────────────────

    [Fact]
    public async Task ListUsers_Forbidden_ForMember()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var member = Fake.MemberUser();
        await Seed.UserAsync(db, member);

        var token = Auth.CreateToken(member.Id, member.Username, "member", false,
            new Microsoft.Extensions.Configuration.ConfigurationBuilder()
                .AddInMemoryCollection(new Dictionary<string, string?> { ["JwtSecret"] = AppFactory.JwtSecret })
                .Build());

        client.DefaultRequestHeaders.Authorization =
            new System.Net.Http.Headers.AuthenticationHeaderValue("Bearer", token);

        var res = await client.GetAsync("/api/users");
        Assert.Equal(HttpStatusCode.Forbidden, res.StatusCode);
    }

    [Fact]
    public async Task CreateUser_Conflict_DuplicateUsername()
    {
        var (client, _) = NewFactory().CreateClientWithDb();
        var token = await RegisterAndGetToken(client, "admin", "secret123");
        client.DefaultRequestHeaders.Authorization =
            new System.Net.Http.Headers.AuthenticationHeaderValue("Bearer", token);

        await client.PostAsJsonAsync("/api/users", new { username = "bob", password = "secret123" });
        var res = await client.PostAsJsonAsync("/api/users", new { username = "bob", password = "secret123" });

        Assert.Equal(HttpStatusCode.Conflict, res.StatusCode);
    }

    [Fact]
    public async Task DeleteUser_Forbidden_CannotDeleteSelf()
    {
        var (client, _) = NewFactory().CreateClientWithDb();
        var regRes = await client.PostAsJsonAsync("/api/auth/register",
            new { username = "admin", password = "secret123" });
        var body = await regRes.Content.ReadFromJsonAsync<JsonElement>();
        var userId = body.GetProperty("user_id").GetString()!;
        var token = body.GetProperty("token").GetString()!;

        client.DefaultRequestHeaders.Authorization =
            new System.Net.Http.Headers.AuthenticationHeaderValue("Bearer", token);

        var res = await client.DeleteAsync($"/api/users/{userId}");
        Assert.Equal(HttpStatusCode.Forbidden, res.StatusCode);
    }

    // ── /api/auth/me PATCH ───────────────────────────────────────────────────

    [Fact]
    public async Task PatchMe_ChangesPassword()
    {
        var (client, _) = NewFactory().CreateClientWithDb();
        var token = await RegisterAndGetToken(client, "admin", "oldpass1");
        client.DefaultRequestHeaders.Authorization =
            new AuthenticationHeaderValue("Bearer", token);

        var res = await client.PatchAsJsonAsync("/api/auth/me", new { password = "newpass1" });
        Assert.Equal(HttpStatusCode.OK, res.StatusCode);

        // Old password rejected
        client.DefaultRequestHeaders.Authorization = null;
        var oldLogin = await client.PostAsJsonAsync("/api/auth/login",
            new { username = "admin", password = "oldpass1" });
        Assert.Equal(HttpStatusCode.Unauthorized, oldLogin.StatusCode);

        // New password accepted
        var newLogin = await client.PostAsJsonAsync("/api/auth/login",
            new { username = "admin", password = "newpass1" });
        Assert.Equal(HttpStatusCode.OK, newLogin.StatusCode);
    }

    [Fact]
    public async Task PatchMe_UnprocessableEntity_ShortPassword()
    {
        var (client, _) = NewFactory().CreateClientWithDb();
        var token = await RegisterAndGetToken(client, "admin", "secret123");
        client.DefaultRequestHeaders.Authorization =
            new AuthenticationHeaderValue("Bearer", token);

        var res = await client.PatchAsJsonAsync("/api/auth/me", new { password = "abc" });
        Assert.Equal(HttpStatusCode.UnprocessableEntity, res.StatusCode);
    }

    // ── /api/users full CRUD ─────────────────────────────────────────────────

    [Fact]
    public async Task ListUsers_ReturnsAllUsers_ForAdmin()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var token = await RegisterAndGetToken(client, "admin", "secret123");
        client.DefaultRequestHeaders.Authorization =
            new AuthenticationHeaderValue("Bearer", token);

        await client.PostAsJsonAsync("/api/users", new { username = "bob", password = "secret123" });
        await client.PostAsJsonAsync("/api/users", new { username = "carol", password = "secret123" });

        var res = await client.GetFromJsonAsync<JsonElement>("/api/users");
        Assert.Equal(3, res.GetArrayLength()); // admin + bob + carol
    }

    [Fact]
    public async Task CreateUser_Created_ValidMember()
    {
        var (client, _) = NewFactory().CreateClientWithDb();
        var token = await RegisterAndGetToken(client, "admin", "secret123");
        client.DefaultRequestHeaders.Authorization =
            new AuthenticationHeaderValue("Bearer", token);

        var res = await client.PostAsJsonAsync("/api/users", new { username = "bob", password = "secret123" });
        Assert.Equal(HttpStatusCode.Created, res.StatusCode);

        // Created user can log in
        client.DefaultRequestHeaders.Authorization = null;
        var login = await client.PostAsJsonAsync("/api/auth/login",
            new { username = "bob", password = "secret123" });
        Assert.Equal(HttpStatusCode.OK, login.StatusCode);

        var body = await login.Content.ReadFromJsonAsync<JsonElement>();
        Assert.Equal("member", body.GetProperty("role").GetString());
        Assert.False(body.GetProperty("allow_explicit").GetBoolean());
    }

    [Fact]
    public async Task CreateUser_Forbidden_ForMember()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var member = Fake.MemberUser();
        await Seed.UserAsync(db, member);
        AuthorizeAs(client, member);

        var res = await client.PostAsJsonAsync("/api/users", new { username = "x", password = "secret123" });
        Assert.Equal(HttpStatusCode.Forbidden, res.StatusCode);
    }

    [Fact]
    public async Task PatchUser_UpdatesRole()
    {
        var (client, _) = NewFactory().CreateClientWithDb();
        var token = await RegisterAndGetToken(client, "admin", "secret123");
        client.DefaultRequestHeaders.Authorization =
            new AuthenticationHeaderValue("Bearer", token);

        var createRes = await client.PostAsJsonAsync("/api/users", new { username = "bob", password = "secret123" });
        Assert.Equal(HttpStatusCode.Created, createRes.StatusCode);

        var users = await client.GetFromJsonAsync<JsonElement>("/api/users");
        var bobId = users.EnumerateArray()
            .First(u => u.GetProperty("username").GetString() == "bob")
            .GetProperty("id").GetString()!;

        var patch = await client.PatchAsJsonAsync($"/api/users/{bobId}", new { role = "admin" });
        Assert.Equal(HttpStatusCode.NoContent, patch.StatusCode);

        var updated = users = await client.GetFromJsonAsync<JsonElement>("/api/users");
        var bob = updated.EnumerateArray().First(u => u.GetProperty("id").GetString() == bobId);
        Assert.Equal("admin", bob.GetProperty("role").GetString());
    }

    [Fact]
    public async Task PatchUser_UpdatesAllowExplicit()
    {
        var (client, _) = NewFactory().CreateClientWithDb();
        var token = await RegisterAndGetToken(client, "admin", "secret123");
        client.DefaultRequestHeaders.Authorization =
            new AuthenticationHeaderValue("Bearer", token);

        await client.PostAsJsonAsync("/api/users", new { username = "bob", password = "secret123" });
        var users = await client.GetFromJsonAsync<JsonElement>("/api/users");
        var bobId = users.EnumerateArray()
            .First(u => u.GetProperty("username").GetString() == "bob")
            .GetProperty("id").GetString()!;

        Assert.False(users.EnumerateArray()
            .First(u => u.GetProperty("id").GetString() == bobId)
            .GetProperty("allow_explicit").GetBoolean());

        await client.PatchAsJsonAsync($"/api/users/{bobId}", new { allow_explicit = true });
        var updated = await client.GetFromJsonAsync<JsonElement>("/api/users");
        var bob = updated.EnumerateArray().First(u => u.GetProperty("id").GetString() == bobId);
        Assert.True(bob.GetProperty("allow_explicit").GetBoolean());
    }

    [Fact]
    public async Task PatchUser_UnprocessableEntity_InvalidRole()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var token = await RegisterAndGetToken(client, "admin", "secret123");
        client.DefaultRequestHeaders.Authorization =
            new AuthenticationHeaderValue("Bearer", token);

        await client.PostAsJsonAsync("/api/users", new { username = "bob", password = "secret123" });
        var users = await client.GetFromJsonAsync<JsonElement>("/api/users");
        var bobId = users.EnumerateArray()
            .First(u => u.GetProperty("username").GetString() == "bob")
            .GetProperty("id").GetString()!;

        var res = await client.PatchAsJsonAsync($"/api/users/{bobId}", new { role = "superuser" });
        Assert.Equal(HttpStatusCode.UnprocessableEntity, res.StatusCode);
    }

    [Fact]
    public async Task PatchUser_NotFound_NonexistentUser()
    {
        var (client, _) = NewFactory().CreateClientWithDb();
        var token = await RegisterAndGetToken(client, "admin", "secret123");
        client.DefaultRequestHeaders.Authorization =
            new AuthenticationHeaderValue("Bearer", token);

        var res = await client.PatchAsJsonAsync("/api/users/ghost", new { role = "member" });
        Assert.Equal(HttpStatusCode.NotFound, res.StatusCode);
    }

    [Fact]
    public async Task DeleteUser_NoContent_Success()
    {
        var (client, _) = NewFactory().CreateClientWithDb();
        var token = await RegisterAndGetToken(client, "admin", "secret123");
        client.DefaultRequestHeaders.Authorization =
            new AuthenticationHeaderValue("Bearer", token);

        await client.PostAsJsonAsync("/api/users", new { username = "bob", password = "secret123" });
        var users = await client.GetFromJsonAsync<JsonElement>("/api/users");
        var bobId = users.EnumerateArray()
            .First(u => u.GetProperty("username").GetString() == "bob")
            .GetProperty("id").GetString()!;

        var res = await client.DeleteAsync($"/api/users/{bobId}");
        Assert.Equal(HttpStatusCode.NoContent, res.StatusCode);

        var after = await client.GetFromJsonAsync<JsonElement>("/api/users");
        Assert.Equal(1, after.GetArrayLength()); // only admin left
    }

    [Fact]
    public async Task DeleteUser_NotFound_NonexistentUser()
    {
        var (client, _) = NewFactory().CreateClientWithDb();
        var token = await RegisterAndGetToken(client, "admin", "secret123");
        client.DefaultRequestHeaders.Authorization =
            new AuthenticationHeaderValue("Bearer", token);

        var res = await client.DeleteAsync("/api/users/ghost");
        Assert.Equal(HttpStatusCode.NotFound, res.StatusCode);
    }

    [Fact]
    public async Task DeleteUser_Forbidden_ForMember()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var admin = Fake.AdminUser();
        var member = Fake.MemberUser();
        await Seed.UserAsync(db, admin);
        await Seed.UserAsync(db, member);
        AuthorizeAs(client, member);

        var res = await client.DeleteAsync($"/api/users/{admin.Id}");
        Assert.Equal(HttpStatusCode.Forbidden, res.StatusCode);
    }

    // ── helpers ──────────────────────────────────────────────────────────────

    static async Task<string> RegisterAndGetToken(HttpClient client, string username, string password)
    {
        var res = await client.PostAsJsonAsync("/api/auth/register", new { username, password });
        var body = await res.Content.ReadFromJsonAsync<JsonElement>();
        return body.GetProperty("token").GetString()!;
    }

    static void AuthorizeAs(HttpClient client, ArrghServer.Data.Models.User user)
    {
        var config = new ConfigurationBuilder()
            .AddInMemoryCollection(new Dictionary<string, string?> { ["JwtSecret"] = AppFactory.JwtSecret })
            .Build();
        var token = Auth.CreateToken(user.Id, user.Username, user.Role, user.AllowExplicit, config);
        client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue("Bearer", token);
    }
}
