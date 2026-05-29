using System.Net;
using System.Net.Http.Headers;
using System.Net.Http.Json;
using System.Text.Json;
using ArrghServer.Api;
using ArrghServer.Data.Models;
using Microsoft.Extensions.Configuration;
using Xunit;

namespace ArrghServer.Tests;

[Trait("Category", TestCategories.Integration)]
public class ProgressTests
{
    static AppFactory NewFactory() => new();

    // ── GET /api/progress/title/{titleId} ─────────────────────────────────────

    [Fact]
    public async Task ListTitleProgress_ReturnsProgressForUser()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        var title = await Seed.TitleAsync(db, user.Id);
        var c1 = await Seed.ChapterAsync(db, title.Id);
        var c2 = await Seed.ChapterAsync(db, title.Id);
        await Seed.MarkReadAsync(db, user.Id, c1.Id);
        Authorize(client, user);

        var res = await client.GetFromJsonAsync<JsonElement>($"/api/progress/title/{title.Id}");
        Assert.Equal(1, res.GetArrayLength());
        Assert.Equal(c1.Id, res[0].GetProperty("chapter_id").GetString());
    }

    [Fact]
    public async Task ListTitleProgress_Empty_WhenNoProgress()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        var title = await Seed.TitleAsync(db, user.Id);
        await Seed.ChapterAsync(db, title.Id);
        Authorize(client, user);

        var res = await client.GetFromJsonAsync<JsonElement>($"/api/progress/title/{title.Id}");
        Assert.Equal(0, res.GetArrayLength());
    }

    [Fact]
    public async Task ListTitleProgress_IsolatedPerUser()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user1 = Fake.AdminUser();
        var user2 = Fake.MemberUser();
        await Seed.UserAsync(db, user1);
        await Seed.UserAsync(db, user2);
        var title = await Seed.TitleAsync(db, user1.Id);
        var chapter = await Seed.ChapterAsync(db, title.Id);
        await Seed.MarkReadAsync(db, user2.Id, chapter.Id); // user2 read it, not user1
        Authorize(client, user1);

        var res = await client.GetFromJsonAsync<JsonElement>($"/api/progress/title/{title.Id}");
        Assert.Equal(0, res.GetArrayLength());
    }

    // ── GET /api/progress/{chapterId} ─────────────────────────────────────────

    [Fact]
    public async Task GetProgress_ReturnsProgress()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        var title = await Seed.TitleAsync(db, user.Id);
        var chapter = await Seed.ChapterAsync(db, title.Id);
        await Seed.MarkReadAsync(db, user.Id, chapter.Id);
        Authorize(client, user);

        var res = await client.GetFromJsonAsync<JsonElement>($"/api/progress/{chapter.Id}");
        Assert.Equal(chapter.Id, res.GetProperty("chapter_id").GetString());
        Assert.True(res.GetProperty("completed").GetBoolean());
    }

    [Fact]
    public async Task GetProgress_NotFound_WhenNoProgress()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        var title = await Seed.TitleAsync(db, user.Id);
        var chapter = await Seed.ChapterAsync(db, title.Id);
        Authorize(client, user);

        Assert.Equal(HttpStatusCode.NotFound,
            (await client.GetAsync($"/api/progress/{chapter.Id}")).StatusCode);
    }

    [Fact]
    public async Task GetProgress_Unauthorized_NoToken()
    {
        var (client, _) = NewFactory().CreateClientWithDb();
        Assert.Equal(HttpStatusCode.Unauthorized,
            (await client.GetAsync("/api/progress/any")).StatusCode);
    }

    // ── PUT /api/progress/{chapterId} ─────────────────────────────────────────

    [Fact]
    public async Task UpdateProgress_Creates_WhenNotExists()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        var title = await Seed.TitleAsync(db, user.Id);
        var chapter = await Seed.ChapterAsync(db, title.Id);
        Authorize(client, user);

        var res = await client.PutAsJsonAsync($"/api/progress/{chapter.Id}",
            new { current_page = 5, completed = false });
        Assert.Equal(HttpStatusCode.OK, res.StatusCode);

        var body = await res.Content.ReadFromJsonAsync<JsonElement>();
        Assert.Equal(5, body.GetProperty("current_page").GetInt32());
        Assert.False(body.GetProperty("completed").GetBoolean());
        Assert.Equal(chapter.Id, body.GetProperty("chapter_id").GetString());
    }

    [Fact]
    public async Task UpdateProgress_Updates_WhenAlreadyExists()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        var title = await Seed.TitleAsync(db, user.Id);
        var chapter = await Seed.ChapterAsync(db, title.Id);
        Authorize(client, user);

        await client.PutAsJsonAsync($"/api/progress/{chapter.Id}",
            new { current_page = 3, completed = false });
        var res = await client.PutAsJsonAsync($"/api/progress/{chapter.Id}",
            new { current_page = 10, completed = true });

        var body = await res.Content.ReadFromJsonAsync<JsonElement>();
        Assert.Equal(10, body.GetProperty("current_page").GetInt32());
        Assert.True(body.GetProperty("completed").GetBoolean());
    }

    [Fact]
    public async Task UpdateProgress_IsolatedPerUser()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user1 = Fake.AdminUser();
        var user2 = Fake.MemberUser();
        await Seed.UserAsync(db, user1);
        await Seed.UserAsync(db, user2);
        var title = await Seed.TitleAsync(db, user1.Id);
        var chapter = await Seed.ChapterAsync(db, title.Id);

        Authorize(client, user1);
        await client.PutAsJsonAsync($"/api/progress/{chapter.Id}",
            new { current_page = 5, completed = false });

        Authorize(client, user2);
        var res = await client.PutAsJsonAsync($"/api/progress/{chapter.Id}",
            new { current_page = 99, completed = true });

        // user2's progress doesn't affect user1's
        Authorize(client, user1);
        var u1Progress = await client.GetFromJsonAsync<JsonElement>($"/api/progress/{chapter.Id}");
        Assert.Equal(5, u1Progress.GetProperty("current_page").GetInt32());
        Assert.False(u1Progress.GetProperty("completed").GetBoolean());
    }

    // ── GET /api/progress/continue ────────────────────────────────────────────

    [Fact]
    public async Task ContinueReading_ReturnsTitlesWithUnreadChapters()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        var title = await Seed.TitleAsync(db, user.Id);

        var c1 = Fake.Chapter(title.Id); c1.Number = 1; c1.Downloaded = true;
        var c2 = Fake.Chapter(title.Id); c2.Number = 2; c2.Downloaded = true;
        await Seed.ChapterAsync(db, title.Id, c1);
        await Seed.ChapterAsync(db, title.Id, c2);

        // User read c1, c2 is unread — title should appear in continue
        await Seed.MarkReadAsync(db, user.Id, c1.Id);
        Authorize(client, user);

        var res = await client.GetFromJsonAsync<JsonElement>("/api/progress/continue");
        Assert.Equal(1, res.GetArrayLength());
        Assert.Equal(title.Id, res[0].GetProperty("title_id").GetString());
        Assert.Equal(2, res[0].GetProperty("chapter_number").GetDouble()); // next unread = c2
        Assert.Equal(1, res[0].GetProperty("chapters_read").GetInt64());
        Assert.Equal(2, res[0].GetProperty("total_chapters").GetInt64());
    }

    [Fact]
    public async Task ContinueReading_Empty_WhenNothingStarted()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        var title = await Seed.TitleAsync(db, user.Id);
        var c = Fake.Chapter(title.Id); c.Downloaded = true;
        await Seed.ChapterAsync(db, title.Id, c);
        Authorize(client, user);

        var res = await client.GetFromJsonAsync<JsonElement>("/api/progress/continue");
        Assert.Equal(0, res.GetArrayLength()); // no completed chapters → not in continue list
    }

    [Fact]
    public async Task ContinueReading_Empty_WhenAllChaptersRead()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        var title = await Seed.TitleAsync(db, user.Id);
        var c = Fake.Chapter(title.Id); c.Downloaded = true;
        await Seed.ChapterAsync(db, title.Id, c);
        await Seed.MarkReadAsync(db, user.Id, c.Id);
        Authorize(client, user);

        var res = await client.GetFromJsonAsync<JsonElement>("/api/progress/continue");
        Assert.Equal(0, res.GetArrayLength()); // no unread downloaded chapters remaining
    }

    [Fact]
    public async Task ContinueReading_SkipsNotDownloadedChapters()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        var title = await Seed.TitleAsync(db, user.Id);

        var c1 = Fake.Chapter(title.Id); c1.Number = 1; c1.Downloaded = true;
        var c2 = Fake.Chapter(title.Id); c2.Number = 2; c2.Downloaded = false; // not downloaded
        var c3 = Fake.Chapter(title.Id); c3.Number = 3; c3.Downloaded = true;
        await Seed.ChapterAsync(db, title.Id, c1);
        await Seed.ChapterAsync(db, title.Id, c2);
        await Seed.ChapterAsync(db, title.Id, c3);
        await Seed.MarkReadAsync(db, user.Id, c1.Id);
        Authorize(client, user);

        var res = await client.GetFromJsonAsync<JsonElement>("/api/progress/continue");
        Assert.Equal(1, res.GetArrayLength());
        Assert.Equal(3, res[0].GetProperty("chapter_number").GetDouble()); // skips c2 (not downloaded)
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
