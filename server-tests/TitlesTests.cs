using System.Net;
using System.Net.Http.Headers;
using System.Net.Http.Json;
using System.Text.Json;
using ArrghServer.Api;
using Microsoft.EntityFrameworkCore;
using Microsoft.Extensions.Configuration;
using Xunit;

namespace ArrghServer.Tests;

[Trait("Category", TestCategories.Integration)]
public class TitlesTests
{
    static AppFactory NewFactory() => new();

    // ── GET /api/titles ──────────────────────────────────────────────────────

    [Fact]
    public async Task ListTitles_EmptyLibrary_ReturnsEmptyPage()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        Authorize(client, user);

        var res = await client.GetFromJsonAsync<JsonElement>("/api/titles");
        Assert.Equal(0, res.GetProperty("total").GetInt32());
        Assert.Equal(0, res.GetProperty("items").GetArrayLength());
    }

    [Fact]
    public async Task ListTitles_ReturnsOwnedTitles()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        await Seed.TitleAsync(db, user.Id);
        await Seed.TitleAsync(db, user.Id);
        Authorize(client, user);

        var res = await client.GetFromJsonAsync<JsonElement>("/api/titles");
        Assert.Equal(2, res.GetProperty("total").GetInt32());
        Assert.Equal(2, res.GetProperty("items").GetArrayLength());
    }

    [Fact]
    public async Task ListTitles_ExcludesOtherUsersLibrary()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user1 = Fake.AdminUser();
        var user2 = Fake.MemberUser();
        await Seed.UserAsync(db, user1);
        await Seed.UserAsync(db, user2);
        await Seed.TitleAsync(db, user2.Id); // owned by user2 only
        Authorize(client, user1);

        var res = await client.GetFromJsonAsync<JsonElement>("/api/titles");
        Assert.Equal(0, res.GetProperty("total").GetInt32());
    }

    [Fact]
    public async Task ListTitles_HidesExplicitFromNonExplicitUser()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.MemberUser(); // allow_explicit = false
        await Seed.UserAsync(db, user);
        var explicit_title = Fake.Title(isExplicit: true);
        await Seed.TitleAsync(db, user.Id, explicit_title);
        Authorize(client, user, allowExplicit: false);

        var res = await client.GetFromJsonAsync<JsonElement>("/api/titles");
        Assert.Equal(0, res.GetProperty("total").GetInt32());
    }

    [Fact]
    public async Task ListTitles_ShowsExplicitToExplicitUser()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser(); // allow_explicit = true
        await Seed.UserAsync(db, user);
        var explicit_title = Fake.Title(isExplicit: true);
        await Seed.TitleAsync(db, user.Id, explicit_title);
        Authorize(client, user, allowExplicit: true);

        var res = await client.GetFromJsonAsync<JsonElement>("/api/titles");
        Assert.Equal(1, res.GetProperty("total").GetInt32());
    }

    [Fact]
    public async Task ListTitles_SearchFiltersByTitle()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);

        var t1 = Fake.Title(); t1.TitleName = "Naruto";
        var t2 = Fake.Title(); t2.TitleName = "Bleach";
        await Seed.TitleAsync(db, user.Id, t1);
        await Seed.TitleAsync(db, user.Id, t2);
        Authorize(client, user);

        var res = await client.GetFromJsonAsync<JsonElement>("/api/titles?search=Naruto");
        Assert.Equal(1, res.GetProperty("total").GetInt32());
        Assert.Equal("Naruto", res.GetProperty("items")[0].GetProperty("title").GetString());
    }

    [Fact]
    public async Task ListTitles_Unauthorized_NoToken()
    {
        var (client, _) = NewFactory().CreateClientWithDb();
        var res = await client.GetAsync("/api/titles");
        Assert.Equal(HttpStatusCode.Unauthorized, res.StatusCode);
    }

    // ── GET /api/titles/{id} ─────────────────────────────────────────────────

    [Fact]
    public async Task GetTitle_ReturnsTitle_WhenOwned()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        var title = await Seed.TitleAsync(db, user.Id);
        Authorize(client, user);

        var res = await client.GetFromJsonAsync<JsonElement>($"/api/titles/{title.Id}");
        Assert.Equal(title.Id, res.GetProperty("id").GetString());
        Assert.Equal(title.TitleName, res.GetProperty("title").GetString());
    }

    [Fact]
    public async Task GetTitle_NotFound_WhenNotOwned()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        var title = Fake.Title();
        db.Titles.Add(title); // no user_titles row
        await db.SaveChangesAsync();
        Authorize(client, user);

        var res = await client.GetAsync($"/api/titles/{title.Id}");
        Assert.Equal(HttpStatusCode.NotFound, res.StatusCode);
    }

    [Fact]
    public async Task GetTitle_NotFound_Nonexistent()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        Authorize(client, user);

        var res = await client.GetAsync("/api/titles/does-not-exist");
        Assert.Equal(HttpStatusCode.NotFound, res.StatusCode);
    }

    // ── GET /api/titles/new-releases ─────────────────────────────────────────

    [Fact]
    public async Task NewReleases_ReturnsNewChapters()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        var title = await Seed.TitleAsync(db, user.Id);
        var chapter = Fake.Chapter(title.Id); chapter.IsNew = true;
        await Seed.ChapterAsync(db, title.Id, chapter);
        Authorize(client, user);

        var res = await client.GetFromJsonAsync<JsonElement>("/api/titles/new-releases");
        Assert.Equal(1, res.GetArrayLength());
        Assert.Equal(chapter.Id, res[0].GetProperty("chapter_id").GetString());
    }

    [Fact]
    public async Task NewReleases_ExcludesExplicitFromNonExplicitUser()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.MemberUser();
        await Seed.UserAsync(db, user);
        var explicit_title = Fake.Title(isExplicit: true);
        await Seed.TitleAsync(db, user.Id, explicit_title);
        var chapter = Fake.Chapter(explicit_title.Id); chapter.IsNew = true;
        await Seed.ChapterAsync(db, explicit_title.Id, chapter);
        Authorize(client, user, allowExplicit: false);

        var res = await client.GetFromJsonAsync<JsonElement>("/api/titles/new-releases");
        Assert.Equal(0, res.GetArrayLength());
    }

    // ── DELETE /api/titles/{id} ───────────────────────────────────────────────

    [Fact]
    public async Task RemoveTitle_NoContent_WhenOwned()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        var title = await Seed.TitleAsync(db, user.Id);
        Authorize(client, user);

        var res = await client.DeleteAsync($"/api/titles/{title.Id}");
        Assert.Equal(HttpStatusCode.NoContent, res.StatusCode);
    }

    [Fact]
    public async Task RemoveTitle_NotFound_WhenNotOwned()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        Authorize(client, user);

        var res = await client.DeleteAsync("/api/titles/ghost");
        Assert.Equal(HttpStatusCode.NotFound, res.StatusCode);
    }

    // ── PATCH /api/titles/{id} ────────────────────────────────────────────────

    [Fact]
    public async Task PatchTitle_UpdatesAutoDownload()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        var title = await Seed.TitleAsync(db, user.Id);
        Authorize(client, user);

        var res = await client.PatchAsJsonAsync($"/api/titles/{title.Id}",
            new { auto_download = true });
        Assert.Equal(HttpStatusCode.NoContent, res.StatusCode);

        db.ChangeTracker.Clear();
        var updated = await db.Titles.AsNoTracking().FirstAsync(t => t.Id == title.Id);
        Assert.True(updated.AutoDownload);
    }

    [Fact]
    public async Task PatchTitle_Forbidden_IsExplicit_ForMember()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var member = Fake.MemberUser();
        await Seed.UserAsync(db, member);
        var title = await Seed.TitleAsync(db, member.Id);
        Authorize(client, member);

        var res = await client.PatchAsJsonAsync($"/api/titles/{title.Id}",
            new { is_explicit = true });
        Assert.Equal(HttpStatusCode.Forbidden, res.StatusCode);
    }

    [Fact]
    public async Task PatchTitle_UnprocessableEntity_InvalidReaderMode()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        var title = await Seed.TitleAsync(db, user.Id);
        Authorize(client, user);

        var res = await client.PatchAsJsonAsync($"/api/titles/{title.Id}",
            new { reader_mode = "invalid" });
        Assert.Equal(HttpStatusCode.UnprocessableEntity, res.StatusCode);
    }

    // ── PATCH /api/titles/{id} — more cases ──────────────────────────────────

    [Fact]
    public async Task PatchTitle_SetsReaderMode()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        var title = await Seed.TitleAsync(db, user.Id);
        Authorize(client, user);

        var res = await client.PatchAsJsonAsync($"/api/titles/{title.Id}", new { reader_mode = "scroll" });
        Assert.Equal(HttpStatusCode.NoContent, res.StatusCode);

        var get = await client.GetFromJsonAsync<JsonElement>($"/api/titles/{title.Id}");
        Assert.Equal("scroll", get.GetProperty("reader_mode").GetString());
    }

    [Fact]
    public async Task PatchTitle_SetsReaderMode_Paged()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        var title = await Seed.TitleAsync(db, user.Id);
        Authorize(client, user);

        await client.PatchAsJsonAsync($"/api/titles/{title.Id}", new { reader_mode = "paged" });
        var get = await client.GetFromJsonAsync<JsonElement>($"/api/titles/{title.Id}");
        Assert.Equal("paged", get.GetProperty("reader_mode").GetString());
    }

    [Fact]
    public async Task PatchTitle_AdminCanSetExplicit()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        var title = await Seed.TitleAsync(db, user.Id);
        Authorize(client, user);

        var res = await client.PatchAsJsonAsync($"/api/titles/{title.Id}", new { is_explicit = true });
        Assert.Equal(HttpStatusCode.NoContent, res.StatusCode);

        db.ChangeTracker.Clear();
        var updated = await db.Titles.AsNoTracking().FirstAsync(t => t.Id == title.Id);
        Assert.True(updated.IsExplicit);
    }

    [Fact]
    public async Task PatchTitle_UnprocessableEntity_InvalidContentType()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        var title = await Seed.TitleAsync(db, user.Id);
        Authorize(client, user);

        var res = await client.PatchAsJsonAsync($"/api/titles/{title.Id}", new { content_type = "comic" });
        Assert.Equal(HttpStatusCode.UnprocessableEntity, res.StatusCode);
    }

    [Fact]
    public async Task PatchTitle_NotFound_WhenNotOwned()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        Authorize(client, user);

        var res = await client.PatchAsJsonAsync("/api/titles/ghost", new { auto_download = true });
        Assert.Equal(HttpStatusCode.NotFound, res.StatusCode);
    }

    // ── title stats (tests complex SQL subqueries) ────────────────────────────

    [Fact]
    public async Task GetTitle_ReportsChapterStats()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        var title = await Seed.TitleAsync(db, user.Id);

        var c1 = await Seed.ChapterAsync(db, title.Id);
        var c2 = await Seed.ChapterAsync(db, title.Id);
        var c3 = await Seed.ChapterAsync(db, title.Id);
        await Seed.MarkDownloadedAsync(db, c1.Id);
        await Seed.MarkDownloadedAsync(db, c2.Id);
        await Seed.MarkReadAsync(db, user.Id, c1.Id);
        Authorize(client, user);

        var res = await client.GetFromJsonAsync<JsonElement>($"/api/titles/{title.Id}");
        Assert.Equal(3, res.GetProperty("total_chapters").GetInt64());
        Assert.Equal(2, res.GetProperty("downloaded_chapters").GetInt64());
        Assert.Equal(1, res.GetProperty("chapters_read").GetInt64());
    }

    [Fact]
    public async Task GetTitle_ChaptersRead_IsolatedPerUser()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user1 = Fake.AdminUser();
        var user2 = Fake.MemberUser();
        await Seed.UserAsync(db, user1);
        await Seed.UserAsync(db, user2);

        var title = await Seed.TitleAsync(db, user1.Id);
        await Seed.TitleAsync(db, user2.Id, title); // user2 subscribes to same title

        var c = await Seed.ChapterAsync(db, title.Id);
        await Seed.MarkReadAsync(db, user2.Id, c.Id); // only user2 read it
        Authorize(client, user1);

        var res = await client.GetFromJsonAsync<JsonElement>($"/api/titles/{title.Id}");
        Assert.Equal(0, res.GetProperty("chapters_read").GetInt64()); // user1 hasn't read
    }

    [Fact]
    public async Task GetTitle_IsLocal_TrueWhenNoSources()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        var title = await Seed.TitleAsync(db, user.Id);
        Authorize(client, user);

        var res = await client.GetFromJsonAsync<JsonElement>($"/api/titles/{title.Id}");
        Assert.True(res.GetProperty("is_local").GetBoolean());
    }

    [Fact]
    public async Task GetTitle_IsLocal_FalseWhenHasSources()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        var title = await Seed.TitleAsync(db, user.Id);
        await Seed.AddTitleSourceAsync(db, title.Id);
        Authorize(client, user);

        var res = await client.GetFromJsonAsync<JsonElement>($"/api/titles/{title.Id}");
        Assert.False(res.GetProperty("is_local").GetBoolean());
    }

    [Fact]
    public async Task GetTitle_HasSyncWarnings_WhenWarningExists()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        var title = await Seed.TitleAsync(db, user.Id);
        await Seed.AddSyncWarningAsync(db, title.Id);
        Authorize(client, user);

        var res = await client.GetFromJsonAsync<JsonElement>($"/api/titles/{title.Id}");
        Assert.True(res.GetProperty("has_sync_warnings").GetBoolean());
    }

    // ── GET /api/titles — pagination ─────────────────────────────────────────

    [Fact]
    public async Task ListTitles_Pagination_LimitApplied()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        for (var i = 0; i < 5; i++) await Seed.TitleAsync(db, user.Id);
        Authorize(client, user);

        var res = await client.GetFromJsonAsync<JsonElement>("/api/titles?limit=2");
        Assert.Equal(5, res.GetProperty("total").GetInt32());
        Assert.Equal(2, res.GetProperty("items").GetArrayLength());
        Assert.Equal(2, res.GetProperty("limit").GetInt32());
    }

    [Fact]
    public async Task ListTitles_Pagination_PageOffset()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        // Prefix all with "pg_" so search path used (orders alphabetically — deterministic)
        var t1 = Fake.Title(); t1.TitleName = "pg_AAA";
        var t2 = Fake.Title(); t2.TitleName = "pg_BBB";
        var t3 = Fake.Title(); t3.TitleName = "pg_CCC";
        await Seed.TitleAsync(db, user.Id, t1);
        await Seed.TitleAsync(db, user.Id, t2);
        await Seed.TitleAsync(db, user.Id, t3);
        Authorize(client, user);

        var page2 = await client.GetFromJsonAsync<JsonElement>("/api/titles?search=pg_&page=2&limit=2");
        Assert.Equal(1, page2.GetProperty("items").GetArrayLength());
        Assert.Equal("pg_CCC", page2.GetProperty("items")[0].GetProperty("title").GetString());
    }

    // ── GET /api/titles/{id}/sync-log ─────────────────────────────────────────

    [Fact]
    public async Task GetSyncLog_ReturnsEntries()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        var title = await Seed.TitleAsync(db, user.Id);
        await Seed.AddSyncLogAsync(db, title.Id, "Syncing chapters from mangadex…");
        await Seed.AddSyncLogAsync(db, title.Id, "Synced 12 chapters from mangadex");
        Authorize(client, user);

        var res = await client.GetFromJsonAsync<JsonElement>($"/api/titles/{title.Id}/sync-log");
        Assert.Equal(2, res.GetArrayLength());
        Assert.Equal("Syncing chapters from mangadex…", res[0].GetProperty("message").GetString());
    }

    [Fact]
    public async Task GetSyncLog_NotFound_WhenNotOwned()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        Authorize(client, user);

        var res = await client.GetAsync("/api/titles/ghost/sync-log");
        Assert.Equal(HttpStatusCode.NotFound, res.StatusCode);
    }

    // ── POST /api/titles/{id}/sync ────────────────────────────────────────────

    [Fact]
    public async Task SyncTitle_Accepted_WhenSourceLinksExist()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        var title = await Seed.TitleAsync(db, user.Id);
        await Seed.AddTitleSourceAsync(db, title.Id);
        Authorize(client, user);

        var res = await client.PostAsync($"/api/titles/{title.Id}/sync", null);
        Assert.Equal(HttpStatusCode.Accepted, res.StatusCode);
    }

    [Fact]
    public async Task SyncTitle_NotFound_WhenNoSourceLinks()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        var title = await Seed.TitleAsync(db, user.Id);
        Authorize(client, user);

        var res = await client.PostAsync($"/api/titles/{title.Id}/sync", null);
        Assert.Equal(HttpStatusCode.NotFound, res.StatusCode);
    }

    [Fact]
    public async Task SyncTitle_NotFound_WhenNotOwned()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        Authorize(client, user);

        var res = await client.PostAsync("/api/titles/ghost/sync", null);
        Assert.Equal(HttpStatusCode.NotFound, res.StatusCode);
    }

    // ── multi-user isolation ──────────────────────────────────────────────────

    [Fact]
    public async Task ListTitles_MultiUser_EachSeesOnlyOwnLibrary()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user1 = Fake.AdminUser();
        var user2 = Fake.MemberUser();
        await Seed.UserAsync(db, user1);
        await Seed.UserAsync(db, user2);

        await Seed.TitleAsync(db, user1.Id); // only user1
        await Seed.TitleAsync(db, user2.Id); // only user2
        var shared = Fake.Title();
        await Seed.TitleAsync(db, user1.Id, shared);
        await Seed.TitleAsync(db, user2.Id, shared); // shared

        Authorize(client, user1);
        var res = await client.GetFromJsonAsync<JsonElement>("/api/titles");
        Assert.Equal(2, res.GetProperty("total").GetInt32()); // user1 sees own + shared
    }

    [Fact]
    public async Task RemoveTitle_DoesNotDeleteTitle_WhenOtherUserStillHasIt()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user1 = Fake.AdminUser();
        var user2 = Fake.MemberUser();
        await Seed.UserAsync(db, user1);
        await Seed.UserAsync(db, user2);

        var title = Fake.Title();
        await Seed.TitleAsync(db, user1.Id, title);
        await Seed.TitleAsync(db, user2.Id, title);
        Authorize(client, user1);

        await client.DeleteAsync($"/api/titles/{title.Id}");

        // Title still exists in DB (user2 still has it)
        db.ChangeTracker.Clear();
        Assert.NotNull(await db.Titles.FindAsync(title.Id));
    }

    [Fact]
    public async Task RemoveTitle_DeletesTitle_WhenLastUser()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        var title = await Seed.TitleAsync(db, user.Id);
        Authorize(client, user);

        await client.DeleteAsync($"/api/titles/{title.Id}");

        db.ChangeTracker.Clear();
        Assert.Null(await db.Titles.FindAsync(title.Id));
    }

    // ── Manual sync → sync log entries ───────────────────────────────────────

    [Fact]
    public async Task SyncTitle_WritesSyncLogEntries()
    {
        // After POST /sync, sync log must have at least one entry.
        // Bug: SyncTitleAsync wrote zero log entries, leaving "Last Sync" section blank.
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        var title = await Seed.TitleAsync(db, user.Id, Fake.Title());
        await Seed.AddTitleSourceAsync(db, title.Id, "mangadex");
        Authorize(client, user);

        var res = await client.PostAsync($"/api/titles/{title.Id}/sync", null);
        Assert.Equal(HttpStatusCode.Accepted, res.StatusCode);

        // Wait for background task to finish
        var deadline = DateTime.UtcNow.AddSeconds(5);
        while (DateTime.UtcNow < deadline)
        {
            db.ChangeTracker.Clear();
            var status = await db.Titles.Where(t => t.Id == title.Id).Select(t => t.SyncStatus).FirstAsync();
            if (status is "ready" or "error") break;
            await Task.Delay(50);
        }

        db.ChangeTracker.Clear();
        var logs = await db.SyncLogs.Where(l => l.TitleId == title.Id).Select(l => l.Message).ToListAsync();
        Assert.NotEmpty(logs);
    }

    [Fact]
    public async Task SyncTitle_SyncLogContainsSyncComplete()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        var title = await Seed.TitleAsync(db, user.Id, Fake.Title());
        await Seed.AddTitleSourceAsync(db, title.Id, "mangadex");
        Authorize(client, user);

        await client.PostAsync($"/api/titles/{title.Id}/sync", null);

        var deadline = DateTime.UtcNow.AddSeconds(5);
        while (DateTime.UtcNow < deadline)
        {
            db.ChangeTracker.Clear();
            var status = await db.Titles.Where(t => t.Id == title.Id).Select(t => t.SyncStatus).FirstAsync();
            if (status is "ready" or "error") break;
            await Task.Delay(50);
        }

        db.ChangeTracker.Clear();
        var logs = await db.SyncLogs.Where(l => l.TitleId == title.Id).Select(l => l.Message).ToListAsync();
        Assert.Contains(logs, m => m.Contains("Sync complete"));
    }

    [Fact]
    public async Task SyncTitle_SyncLogContainsSourceName()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        var title = await Seed.TitleAsync(db, user.Id, Fake.Title());
        await Seed.AddTitleSourceAsync(db, title.Id, "mangadex");
        Authorize(client, user);

        await client.PostAsync($"/api/titles/{title.Id}/sync", null);

        var deadline = DateTime.UtcNow.AddSeconds(5);
        while (DateTime.UtcNow < deadline)
        {
            db.ChangeTracker.Clear();
            var status = await db.Titles.Where(t => t.Id == title.Id).Select(t => t.SyncStatus).FirstAsync();
            if (status is "ready" or "error") break;
            await Task.Delay(50);
        }

        db.ChangeTracker.Clear();
        var logs = await db.SyncLogs.Where(l => l.TitleId == title.Id).Select(l => l.Message).ToListAsync();
        Assert.Contains(logs, m => m.Contains("mangadex"));
    }

    // ── new-releases cross-cutting ────────────────────────────────────────────

    [Fact]
    public async Task NewReleases_OnlyOwnedTitles()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user1 = Fake.AdminUser();
        var user2 = Fake.MemberUser();
        await Seed.UserAsync(db, user1);
        await Seed.UserAsync(db, user2);

        var title2 = await Seed.TitleAsync(db, user2.Id); // user2 only
        var c = Fake.Chapter(title2.Id); c.IsNew = true;
        await Seed.ChapterAsync(db, title2.Id, c);
        Authorize(client, user1);

        var res = await client.GetFromJsonAsync<JsonElement>("/api/titles/new-releases");
        Assert.Equal(0, res.GetArrayLength()); // user1 doesn't own title2
    }

    // ── helpers ───────────────────────────────────────────────────────────────

    void Authorize(HttpClient client, ArrghServer.Data.Models.User user, bool? allowExplicit = null)
    {
        var config = new ConfigurationBuilder()
            .AddInMemoryCollection(new Dictionary<string, string?> { ["JwtSecret"] = AppFactory.JwtSecret })
            .Build();
        var token = Auth.CreateToken(user.Id, user.Username, user.Role,
            allowExplicit ?? user.AllowExplicit, config);
        client.DefaultRequestHeaders.Authorization =
            new AuthenticationHeaderValue("Bearer", token);
    }
}
