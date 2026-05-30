using System.Net;
using System.Net.Http.Headers;
using System.Net.Http.Json;
using System.Text.Json;
using ArrghServer.Api;
using ArrghServer.Data.Models;
using Microsoft.EntityFrameworkCore;
using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Microsoft.Extensions.Hosting;
using Xunit;

namespace ArrghServer.Tests;

// Integration tests for chapter sync (ChapterSync.SyncFromSourceAsync + SyncTitleAsync).
// Uses SyncFactory which fakes plugin-host chapter + search endpoints.

[Trait("Category", TestCategories.Integration)]
public class ChapterSyncTests
{
    // ── Factory helpers ───────────────────────────────────────────────────────

    static SyncFactory NewFactory(
        string chaptersJson = DefaultChaptersJson,
        bool chaptersThrows = false) =>
        new(chaptersJson, chaptersThrows);

    void Authorize(HttpClient client, User user)
    {
        var config = new ConfigurationBuilder()
            .AddInMemoryCollection(new Dictionary<string, string?> { ["JwtSecret"] = AppFactory.JwtSecret })
            .Build();
        var token = Auth.CreateToken(user.Id, user.Username, user.Role, user.AllowExplicit, config);
        client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue("Bearer", token);
    }

    // Default: 2 chapters (number 1 and 2) from plugin-host
    const string DefaultChaptersJson = """
        [
          {"source_id":"src-ch-1","id":"src-ch-1","number":1.0,"title":"Chapter 1"},
          {"source_id":"src-ch-2","id":"src-ch-2","number":2.0,"title":"Chapter 2"}
        ]
        """;

    // Waits for the title to leave "syncing" state (→ "ready" or "error").
    // First polls until we see "syncing" so we don't exit immediately on the
    // pre-sync "ready" value that may linger in a stale SQLite read snapshot.
    static async Task WaitForSyncReadyAsync(ArrghServer.Data.AppDbContext db, string titleId, int maxMs = 5000)
    {
        var sw = System.Diagnostics.Stopwatch.StartNew();

        // Phase 1 — wait until "syncing" appears (server has accepted the job)
        while (sw.ElapsedMilliseconds < maxMs)
        {
            db.ChangeTracker.Clear();
            var s = await db.Titles.Where(t => t.Id == titleId).Select(t => t.SyncStatus).FirstOrDefaultAsync();
            if (s == "syncing") break;
            await Task.Delay(30);
        }

        // Phase 2 — wait until background task finishes
        while (sw.ElapsedMilliseconds < maxMs)
        {
            db.ChangeTracker.Clear();
            var s = await db.Titles.Where(t => t.Id == titleId).Select(t => t.SyncStatus).FirstOrDefaultAsync();
            if (s is "ready" or "error") return;
            await Task.Delay(50);
        }
    }

    // ── POST /api/titles/:id/sync — chapter creation ──────────────────────────

    [Fact]
    public async Task Sync_CreatesChapterRows()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        var title = await Seed.TitleAsync(db, user.Id, Fake.Title());
        await Seed.AddTitleSourceAsync(db, title.Id, "mangadex");
        Authorize(client, user);

        var res = await client.PostAsync($"/api/titles/{title.Id}/sync", null);
        Assert.Equal(HttpStatusCode.Accepted, res.StatusCode);

        await WaitForSyncReadyAsync(db, title.Id);
        db.ChangeTracker.Clear();

        var chapters = await db.Chapters.Where(c => c.TitleId == title.Id).ToListAsync();
        Assert.Equal(2, chapters.Count);
        Assert.Contains(chapters, c => c.Number == 1.0);
        Assert.Contains(chapters, c => c.Number == 2.0);
    }

    [Fact]
    public async Task Sync_CreatesChapterSources()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        var title = await Seed.TitleAsync(db, user.Id, Fake.Title());
        await Seed.AddTitleSourceAsync(db, title.Id, "mangadex");
        Authorize(client, user);

        await client.PostAsync($"/api/titles/{title.Id}/sync", null);
        await WaitForSyncReadyAsync(db, title.Id);
        db.ChangeTracker.Clear();

        var chapterIds = await db.Chapters.Where(c => c.TitleId == title.Id).Select(c => c.Id).ToListAsync();
        var sources = await db.ChapterSources.Where(cs => chapterIds.Contains(cs.ChapterId)).ToListAsync();
        Assert.Equal(2, sources.Count);
        Assert.All(sources, cs => Assert.Equal("mangadex", cs.Source));
        Assert.Contains(sources, cs => cs.SourceId == "src-ch-1");
        Assert.Contains(sources, cs => cs.SourceId == "src-ch-2");
    }

    [Fact]
    public async Task Sync_SetsStatusReady()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        var title = await Seed.TitleAsync(db, user.Id, Fake.Title());
        await Seed.AddTitleSourceAsync(db, title.Id, "mangadex");
        Authorize(client, user);

        await client.PostAsync($"/api/titles/{title.Id}/sync", null);
        await WaitForSyncReadyAsync(db, title.Id);
        db.ChangeTracker.Clear();

        var status = await db.Titles.Where(t => t.Id == title.Id).Select(t => t.SyncStatus).FirstAsync();
        Assert.Equal("ready", status);
    }

    [Fact]
    public async Task Sync_HasSources_True_InGetChapters()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        var title = await Seed.TitleAsync(db, user.Id, Fake.Title());
        await Seed.AddTitleSourceAsync(db, title.Id, "mangadex");
        Authorize(client, user);

        await client.PostAsync($"/api/titles/{title.Id}/sync", null);
        await WaitForSyncReadyAsync(db, title.Id);

        var chapRes = await client.GetFromJsonAsync<JsonElement[]>($"/api/chapters/title/{title.Id}");
        Assert.NotNull(chapRes);
        Assert.NotEmpty(chapRes);
        Assert.All(chapRes, ch => Assert.True(ch.GetProperty("has_sources").GetBoolean()));
    }

    [Fact]
    public async Task Sync_Idempotent_NoDuplicateChapters()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        var title = await Seed.TitleAsync(db, user.Id, Fake.Title());
        await Seed.AddTitleSourceAsync(db, title.Id, "mangadex");
        Authorize(client, user);

        // Sync twice
        await client.PostAsync($"/api/titles/{title.Id}/sync", null);
        await WaitForSyncReadyAsync(db, title.Id);
        await client.PostAsync($"/api/titles/{title.Id}/sync", null);
        await WaitForSyncReadyAsync(db, title.Id);

        db.ChangeTracker.Clear();
        var chapterCount = await db.Chapters.CountAsync(c => c.TitleId == title.Id);
        Assert.Equal(2, chapterCount); // still exactly 2, no duplicates

        var sourceCount = await db.ChapterSources
            .Where(cs => db.Chapters.Where(c => c.TitleId == title.Id).Select(c => c.Id).Contains(cs.ChapterId))
            .CountAsync();
        Assert.Equal(2, sourceCount); // still 2 chapter_sources, no duplicates
    }

    [Fact]
    public async Task Sync_TwoSources_SameChapterNumbers_OneChapterRow_TwoSourceLinks()
    {
        // Both mangadex and mangapill return chapters 1+2 → dedup by number → 1 row per chapter
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        var title = await Seed.TitleAsync(db, user.Id, Fake.Title());
        await Seed.AddTitleSourceAsync(db, title.Id, "mangadex");
        await Seed.AddTitleSourceAsync(db, title.Id, "mangapill");
        Authorize(client, user);

        await client.PostAsync($"/api/titles/{title.Id}/sync", null);
        await WaitForSyncReadyAsync(db, title.Id);
        db.ChangeTracker.Clear();

        // Only 2 chapter rows (not 4)
        var chapters = await db.Chapters.Where(c => c.TitleId == title.Id).ToListAsync();
        Assert.Equal(2, chapters.Count);

        // But 4 chapter_sources (2 per chapter from 2 sources)
        var chapterIds = chapters.Select(c => c.Id).ToList();
        var sourcePairs = await db.ChapterSources
            .Where(cs => chapterIds.Contains(cs.ChapterId))
            .Select(cs => new { cs.ChapterId, cs.Source })
            .ToListAsync();
        Assert.Equal(4, sourcePairs.Count);
        Assert.Contains(sourcePairs, x => x.Source == "mangadex");
        Assert.Contains(sourcePairs, x => x.Source == "mangapill");
    }

    [Fact]
    public async Task Sync_Novel_ChapterFormat_IsText()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);

        var novel = Fake.Title();
        novel.ContentType = "novel";
        await Seed.TitleAsync(db, user.Id, novel);
        await Seed.AddTitleSourceAsync(db, novel.Id, "boxnovel");
        Authorize(client, user);

        await client.PostAsync($"/api/titles/{novel.Id}/sync", null);
        await WaitForSyncReadyAsync(db, novel.Id);
        db.ChangeTracker.Clear();

        var chapters = await db.Chapters.Where(c => c.TitleId == novel.Id).ToListAsync();
        Assert.NotEmpty(chapters);
        Assert.All(chapters, c => Assert.Equal("text", c.ChapterFormat));
    }

    [Fact]
    public async Task Sync_Manga_ChapterFormat_IsPages()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);

        var manga = Fake.Title();
        manga.ContentType = "manga";
        await Seed.TitleAsync(db, user.Id, manga);
        await Seed.AddTitleSourceAsync(db, manga.Id, "mangadex");
        Authorize(client, user);

        await client.PostAsync($"/api/titles/{manga.Id}/sync", null);
        await WaitForSyncReadyAsync(db, manga.Id);
        db.ChangeTracker.Clear();

        var chapters = await db.Chapters.Where(c => c.TitleId == manga.Id).ToListAsync();
        Assert.All(chapters, c => Assert.Equal("pages", c.ChapterFormat));
    }

    [Fact]
    public async Task Sync_PluginHostError_SetsStatusError()
    {
        // If plugin-host fails for all sources, sync_status should be "error"
        var (client, db) = NewFactory(chaptersThrows: true).CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        var title = await Seed.TitleAsync(db, user.Id, Fake.Title());
        await Seed.AddTitleSourceAsync(db, title.Id, "mangadex");
        Authorize(client, user);

        await client.PostAsync($"/api/titles/{title.Id}/sync", null);
        await WaitForSyncReadyAsync(db, title.Id);
        db.ChangeTracker.Clear();

        var status = await db.Titles.Where(t => t.Id == title.Id).Select(t => t.SyncStatus).FirstAsync();
        Assert.Equal("error", status);
    }

    // ── IsNew flag ──────────────────────────────────────────────────────────

    [Fact]
    public async Task Sync_ChapterIsNew_IsFalse_AfterManualSync()
    {
        // Chapters created during explicit sync must NOT be marked is_new so they don't
        // flood New Releases. Only the indexer (background periodic scan) should mark is_new.
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        var title = await Seed.TitleAsync(db, user.Id, Fake.Title());
        await Seed.AddTitleSourceAsync(db, title.Id, "mangadex");
        Authorize(client, user);

        await client.PostAsync($"/api/titles/{title.Id}/sync", null);
        await WaitForSyncReadyAsync(db, title.Id);
        db.ChangeTracker.Clear();

        var chapters = await db.Chapters.Where(c => c.TitleId == title.Id).ToListAsync();
        Assert.NotEmpty(chapters);
        Assert.All(chapters, c => Assert.False(c.IsNew));
    }

    [Fact]
    public async Task NewReleases_ReturnsEmpty_AfterSync()
    {
        // GET /api/titles/new-releases should not include chapters from manual sync
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        var title = await Seed.TitleAsync(db, user.Id, Fake.Title());
        await Seed.AddTitleSourceAsync(db, title.Id, "mangadex");
        Authorize(client, user);

        await client.PostAsync($"/api/titles/{title.Id}/sync", null);
        await WaitForSyncReadyAsync(db, title.Id);

        var res = await client.GetFromJsonAsync<System.Text.Json.JsonElement[]>("/api/titles/new-releases");
        Assert.NotNull(res);
        Assert.Empty(res);
    }

    // ── Auth / ownership ─────────────────────────────────────────────────────

    [Fact]
    public async Task Sync_Returns401_WhenUnauthenticated()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        var title = await Seed.TitleAsync(db, user.Id, Fake.Title());
        await Seed.AddTitleSourceAsync(db, title.Id, "mangadex");
        // No Authorize call

        var res = await client.PostAsync($"/api/titles/{title.Id}/sync", null);
        Assert.Equal(HttpStatusCode.Unauthorized, res.StatusCode);
    }

    [Fact]
    public async Task Sync_Returns404_WhenCallerDoesNotOwnTitle()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var owner = Fake.AdminUser();
        var caller = Fake.MemberUser();
        await Seed.UserAsync(db, owner);
        await Seed.UserAsync(db, caller);
        var title = await Seed.TitleAsync(db, owner.Id, Fake.Title()); // owned by owner
        await Seed.AddTitleSourceAsync(db, title.Id, "mangadex");
        Authorize(client, caller); // authenticated as caller, not owner

        var res = await client.PostAsync($"/api/titles/{title.Id}/sync", null);
        Assert.Equal(HttpStatusCode.NotFound, res.StatusCode);
    }

    // ── 404 edge cases ───────────────────────────────────────────────────────

    [Fact]
    public async Task Sync_Returns404_WhenTitleNotFound()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        Authorize(client, user);

        var res = await client.PostAsync($"/api/titles/nonexistent-id/sync", null);
        Assert.Equal(HttpStatusCode.NotFound, res.StatusCode);
    }

    [Fact]
    public async Task Sync_Returns404_WhenTitleHasNoSources()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        var title = await Seed.TitleAsync(db, user.Id, Fake.Title());
        // No AddTitleSourceAsync — local-only title
        Authorize(client, user);

        var res = await client.PostAsync($"/api/titles/{title.Id}/sync", null);
        Assert.Equal(HttpStatusCode.NotFound, res.StatusCode);
    }

    // ── Sync log ─────────────────────────────────────────────────────────────

    [Fact]
    public async Task Sync_ClearsSyncLog_BeforeNewSync()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        var title = await Seed.TitleAsync(db, user.Id, Fake.Title());
        await Seed.AddTitleSourceAsync(db, title.Id, "mangadex");
        await Seed.AddSyncLogAsync(db, title.Id, "old log entry");
        Authorize(client, user);

        await client.PostAsync($"/api/titles/{title.Id}/sync", null);
        await WaitForSyncReadyAsync(db, title.Id);
        db.ChangeTracker.Clear();

        var logs = await db.SyncLogs.Where(l => l.TitleId == title.Id).ToListAsync();
        Assert.DoesNotContain(logs, l => l.Message == "old log entry");
    }

    // ── Status transition ────────────────────────────────────────────────────

    [Fact]
    public async Task Sync_Returns202_Immediately()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        var title = await Seed.TitleAsync(db, user.Id, Fake.Title());
        await Seed.AddTitleSourceAsync(db, title.Id, "mangadex");
        Authorize(client, user);

        var res = await client.PostAsync($"/api/titles/{title.Id}/sync", null);
        Assert.Equal(HttpStatusCode.Accepted, res.StatusCode);
    }

    // ── Chapter format by content type ──────────────────────────────────────

    [Fact]
    public async Task Sync_Manhwa_ChapterFormat_IsPages()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);

        var title = Fake.Title();
        title.ContentType = "manhwa";
        await Seed.TitleAsync(db, user.Id, title);
        await Seed.AddTitleSourceAsync(db, title.Id, "asurascans");
        Authorize(client, user);

        await client.PostAsync($"/api/titles/{title.Id}/sync", null);
        await WaitForSyncReadyAsync(db, title.Id);
        db.ChangeTracker.Clear();

        var chapters = await db.Chapters.Where(c => c.TitleId == title.Id).ToListAsync();
        Assert.NotEmpty(chapters);
        Assert.All(chapters, c => Assert.Equal("pages", c.ChapterFormat));
    }

    // ── Chapter count logging (Bug: "Syncing from X… Sync complete" with no count) ──

    [Fact]
    public async Task Sync_LogsChapterCount_PerSource()
    {
        // SyncTitleAsync must log "Synced N chapter(s) from {source}" so the user can see
        // whether chapters were found — not just silence between "Syncing from X…" and "Sync complete".
        var (client, db) = NewFactory().CreateClientWithDb(); // returns 2 chapters by default
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        var title = await Seed.TitleAsync(db, user.Id, Fake.Title());
        await Seed.AddTitleSourceAsync(db, title.Id, "mangadex");
        Authorize(client, user);

        await client.PostAsync($"/api/titles/{title.Id}/sync", null);
        await WaitForSyncReadyAsync(db, title.Id);
        db.ChangeTracker.Clear();

        var logs = await db.SyncLogs.Where(l => l.TitleId == title.Id).Select(l => l.Message).ToListAsync();
        Assert.Contains(logs, m =>
            m.Contains("Synced") && m.Contains("chapter") && m.Contains("mangadex"));
    }

    [Fact]
    public async Task Sync_LogsZeroChapterCount_WhenPluginReturnsEmpty()
    {
        // When the plugin returns an empty chapter list, the log must still say
        // "Synced 0 chapter(s) from X" — not silence — so the user knows chapters were sought.
        var (client, db) = new SyncFactory("[]", chaptersThrows: false).CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        var title = await Seed.TitleAsync(db, user.Id, Fake.Title());
        await Seed.AddTitleSourceAsync(db, title.Id, "mangadex");
        Authorize(client, user);

        await client.PostAsync($"/api/titles/{title.Id}/sync", null);
        await WaitForSyncReadyAsync(db, title.Id);
        db.ChangeTracker.Clear();

        var logs = await db.SyncLogs.Where(l => l.TitleId == title.Id).Select(l => l.Message).ToListAsync();
        Assert.Contains(logs, m =>
            m.Contains("0") && m.Contains("chapter") && m.Contains("mangadex"));
    }

    [Fact]
    public async Task Sync_Manhua_ChapterFormat_IsPages()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);

        var title = Fake.Title();
        title.ContentType = "manhua";
        await Seed.TitleAsync(db, user.Id, title);
        await Seed.AddTitleSourceAsync(db, title.Id, "manhuafast");
        Authorize(client, user);

        await client.PostAsync($"/api/titles/{title.Id}/sync", null);
        await WaitForSyncReadyAsync(db, title.Id);
        db.ChangeTracker.Clear();

        var chapters = await db.Chapters.Where(c => c.TitleId == title.Id).ToListAsync();
        Assert.NotEmpty(chapters);
        Assert.All(chapters, c => Assert.Equal("pages", c.ChapterFormat));
    }
}

// ── SyncFactory ───────────────────────────────────────────────────────────────

public class SyncFactory(string chaptersJson, bool chaptersThrows) : AppFactory
{
    protected override IHost CreateHost(IHostBuilder builder)
    {
        var json = chaptersJson;
        var throws = chaptersThrows;

        builder.ConfigureServices(services =>
        {
            services.RemoveAll<IHttpClientFactory>();
            services.AddSingleton<IHttpClientFactory>(new SyncFakeHttpClientFactory(json, throws));
        });
        return base.CreateHost(builder);
    }
}

file class SyncFakeHttpClientFactory(string chaptersJson, bool chaptersThrows) : IHttpClientFactory
{
    public HttpClient CreateClient(string name) =>
        new(new SyncFakeHandler(chaptersJson, chaptersThrows));
}

file class SyncFakeHandler(string chaptersJson, bool chaptersThrows) : HttpMessageHandler
{
    protected override Task<HttpResponseMessage> SendAsync(HttpRequestMessage req, CancellationToken ct)
    {
        var path = req.RequestUri?.PathAndQuery ?? "";

        // plugin-host chapters endpoint
        if (path.Contains("/manga/") && path.Contains("/chapters"))
        {
            if (chaptersThrows) throw new HttpRequestException("plugin-host unavailable");
            return Task.FromResult(Json(chaptersJson));
        }

        // plugin-host search endpoint — return a match with a predictable id
        if (path.Contains("/search"))
            return Task.FromResult(Json("""[{"id":"source-id","title":"Match"}]"""));

        // Anything else — 200 empty
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
