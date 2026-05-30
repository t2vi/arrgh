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
public class QueueTests
{
    static AppFactory NewFactory() => new();

    // ── GET /api/queue ────────────────────────────────────────────────────────

    [Fact]
    public async Task ListQueue_ReturnsItems_OrderedByCreatedAtDesc()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        var title = await Seed.TitleAsync(db, user.Id);
        var c1 = await Seed.ChapterAsync(db, title.Id);
        var c2 = await Seed.ChapterAsync(db, title.Id);
        await Seed.QueueItemAsync(db, c1.Id, title.TitleName, c1.Number, queuedBy: user.Id);
        await Seed.QueueItemAsync(db, c2.Id, title.TitleName, c2.Number, queuedBy: user.Id);
        Authorize(client, user);

        var res = await client.GetFromJsonAsync<JsonElement>("/api/queue");
        Assert.Equal(2, res.GetArrayLength());
    }

    [Fact]
    public async Task ListQueue_Empty_WhenNoItems()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        Authorize(client, user);

        var res = await client.GetFromJsonAsync<JsonElement>("/api/queue");
        Assert.Equal(0, res.GetArrayLength());
    }

    [Fact]
    public async Task ListQueue_HidesExplicit_ForNonExplicitMember()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var member = Fake.MemberUser();
        await Seed.UserAsync(db, member);
        var explicitTitle = Fake.Title(isExplicit: true);
        await Seed.TitleAsync(db, member.Id, explicitTitle);
        var chapter = await Seed.ChapterAsync(db, explicitTitle.Id);
        await Seed.QueueItemAsync(db, chapter.Id, explicitTitle.TitleName, chapter.Number);
        Authorize(client, member, allowExplicit: false);

        var res = await client.GetFromJsonAsync<JsonElement>("/api/queue");
        Assert.Equal(0, res.GetArrayLength());
    }

    [Fact]
    public async Task ListQueue_ShowsExplicit_ForAdmin()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var admin = Fake.AdminUser();
        await Seed.UserAsync(db, admin);
        var explicitTitle = Fake.Title(isExplicit: true);
        await Seed.TitleAsync(db, admin.Id, explicitTitle);
        var chapter = await Seed.ChapterAsync(db, explicitTitle.Id);
        await Seed.QueueItemAsync(db, chapter.Id, explicitTitle.TitleName, chapter.Number);
        // Admin sees explicit regardless of allow_explicit flag
        Authorize(client, admin, allowExplicit: false);

        var res = await client.GetFromJsonAsync<JsonElement>("/api/queue");
        Assert.Equal(1, res.GetArrayLength());
    }

    [Fact]
    public async Task ListQueue_Unauthorized_NoToken()
    {
        var (client, _) = NewFactory().CreateClientWithDb();
        Assert.Equal(HttpStatusCode.Unauthorized,
            (await client.GetAsync("/api/queue")).StatusCode);
    }

    // ── GET /api/queue/title/{titleId} ────────────────────────────────────────

    [Fact]
    public async Task ListTitleQueue_ReturnsItemsForTitle_OrderedByChapterNum()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        var t1 = await Seed.TitleAsync(db, user.Id);
        var t2 = await Seed.TitleAsync(db, user.Id);
        var c1 = Fake.Chapter(t1.Id); c1.Number = 3;
        var c2 = Fake.Chapter(t1.Id); c2.Number = 1;
        var c3 = await Seed.ChapterAsync(db, t2.Id); // different title
        await Seed.ChapterAsync(db, t1.Id, c1);
        await Seed.ChapterAsync(db, t1.Id, c2);
        await Seed.QueueItemAsync(db, c1.Id, t1.TitleName, c1.Number);
        await Seed.QueueItemAsync(db, c2.Id, t1.TitleName, c2.Number);
        await Seed.QueueItemAsync(db, c3.Id, t2.TitleName, c3.Number);
        Authorize(client, user);

        var res = await client.GetFromJsonAsync<JsonElement>($"/api/queue/title/{t1.Id}");
        Assert.Equal(2, res.GetArrayLength());
        Assert.Equal(1, res[0].GetProperty("chapter_num").GetDouble()); // ordered by chapter_num ASC
        Assert.Equal(3, res[1].GetProperty("chapter_num").GetDouble());
    }

    [Fact]
    public async Task ListTitleQueue_Empty_WhenNoItemsForTitle()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        var title = await Seed.TitleAsync(db, user.Id);
        Authorize(client, user);

        var res = await client.GetFromJsonAsync<JsonElement>($"/api/queue/title/{title.Id}");
        Assert.Equal(0, res.GetArrayLength());
    }

    // ── DELETE /api/queue/completed ───────────────────────────────────────────

    [Fact]
    public async Task ClearCompleted_NoContent_DeletesDoneAndCancelledAndError()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        var title = await Seed.TitleAsync(db, user.Id);
        var c1 = await Seed.ChapterAsync(db, title.Id);
        var c2 = await Seed.ChapterAsync(db, title.Id);
        var c3 = await Seed.ChapterAsync(db, title.Id);
        var c4 = await Seed.ChapterAsync(db, title.Id);
        await Seed.QueueItemAsync(db, c1.Id, title.TitleName, 1, "done");
        await Seed.QueueItemAsync(db, c2.Id, title.TitleName, 2, "cancelled");
        await Seed.QueueItemAsync(db, c3.Id, title.TitleName, 3, "error");
        await Seed.QueueItemAsync(db, c4.Id, title.TitleName, 4, "pending");
        Authorize(client, user);

        var res = await client.DeleteAsync("/api/queue/completed");
        Assert.Equal(HttpStatusCode.NoContent, res.StatusCode);

        db.ChangeTracker.Clear();
        Assert.Equal(1, await db.DownloadQueue.CountAsync()); // only pending remains
    }

    [Fact]
    public async Task ClearCompleted_Forbidden_ForMember()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var member = Fake.MemberUser();
        await Seed.UserAsync(db, member);
        Authorize(client, member);

        Assert.Equal(HttpStatusCode.Forbidden,
            (await client.DeleteAsync("/api/queue/completed")).StatusCode);
    }

    // ── DELETE /api/queue/{id} ────────────────────────────────────────────────

    [Fact]
    public async Task RemoveFromQueue_NoContent_DeletesPendingItem()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        var title = await Seed.TitleAsync(db, user.Id);
        var chapter = await Seed.ChapterAsync(db, title.Id);
        var item = await Seed.QueueItemAsync(db, chapter.Id, title.TitleName, chapter.Number,
            "pending", user.Id);
        Authorize(client, user);

        var res = await client.DeleteAsync($"/api/queue/{item.Id}");
        Assert.Equal(HttpStatusCode.NoContent, res.StatusCode);

        db.ChangeTracker.Clear();
        Assert.Null(await db.DownloadQueue.FindAsync(item.Id));
    }

    [Fact]
    public async Task RemoveFromQueue_CancelsDownloading_InsteadOfDelete()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        var title = await Seed.TitleAsync(db, user.Id);
        var chapter = await Seed.ChapterAsync(db, title.Id);
        var item = await Seed.QueueItemAsync(db, chapter.Id, title.TitleName, chapter.Number,
            "downloading", user.Id);
        Authorize(client, user);

        var res = await client.DeleteAsync($"/api/queue/{item.Id}");
        Assert.Equal(HttpStatusCode.NoContent, res.StatusCode);

        db.ChangeTracker.Clear();
        var updated = await db.DownloadQueue.FindAsync(item.Id);
        Assert.NotNull(updated);            // still exists
        Assert.Equal("cancelled", updated!.Status);
    }

    [Fact]
    public async Task RemoveFromQueue_NotFound_NonexistentItem()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        Authorize(client, user);

        Assert.Equal(HttpStatusCode.NotFound,
            (await client.DeleteAsync("/api/queue/ghost")).StatusCode);
    }

    [Fact]
    public async Task RemoveFromQueue_Forbidden_MemberCantCancelOthersItem()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var admin = Fake.AdminUser();
        var member = Fake.MemberUser();
        await Seed.UserAsync(db, admin);
        await Seed.UserAsync(db, member);
        var title = await Seed.TitleAsync(db, admin.Id);
        var chapter = await Seed.ChapterAsync(db, title.Id);
        // item queued by admin
        var item = await Seed.QueueItemAsync(db, chapter.Id, title.TitleName, chapter.Number,
            "pending", admin.Id);
        Authorize(client, member); // member tries to cancel

        Assert.Equal(HttpStatusCode.Forbidden,
            (await client.DeleteAsync($"/api/queue/{item.Id}")).StatusCode);
    }

    [Fact]
    public async Task RemoveFromQueue_AdminCanCancelAnyItem()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var admin = Fake.AdminUser();
        var member = Fake.MemberUser();
        await Seed.UserAsync(db, admin);
        await Seed.UserAsync(db, member);
        var title = await Seed.TitleAsync(db, member.Id);
        var chapter = await Seed.ChapterAsync(db, title.Id);
        // item queued by member
        var item = await Seed.QueueItemAsync(db, chapter.Id, title.TitleName, chapter.Number,
            "pending", member.Id);
        Authorize(client, admin);

        Assert.Equal(HttpStatusCode.NoContent,
            (await client.DeleteAsync($"/api/queue/{item.Id}")).StatusCode);
    }

    [Fact]
    public async Task RemoveFromQueue_MemberCanCancelOwnItem()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var member = Fake.MemberUser();
        await Seed.UserAsync(db, member);
        var title = await Seed.TitleAsync(db, member.Id);
        var chapter = await Seed.ChapterAsync(db, title.Id);
        var item = await Seed.QueueItemAsync(db, chapter.Id, title.TitleName, chapter.Number,
            "pending", member.Id);
        Authorize(client, member);

        Assert.Equal(HttpStatusCode.NoContent,
            (await client.DeleteAsync($"/api/queue/{item.Id}")).StatusCode);
    }

    // ── helpers ───────────────────────────────────────────────────────────────

    void Authorize(HttpClient client, User user, bool? allowExplicit = null)
    {
        var config = new ConfigurationBuilder()
            .AddInMemoryCollection(new Dictionary<string, string?> { ["JwtSecret"] = AppFactory.JwtSecret })
            .Build();
        var token = Auth.CreateToken(user.Id, user.Username, user.Role,
            allowExplicit ?? user.AllowExplicit, config);
        client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue("Bearer", token);
    }
}
