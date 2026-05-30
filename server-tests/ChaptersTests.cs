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
public class ChaptersTests
{
    static AppFactory NewFactory() => new();

    // ── GET /api/chapters/title/{titleId} ─────────────────────────────────────

    [Fact]
    public async Task ListChapters_ReturnsChaptersOrderedByNumber()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        var title = await Seed.TitleAsync(db, user.Id);
        var c1 = Fake.Chapter(title.Id); c1.Number = 3;
        var c2 = Fake.Chapter(title.Id); c2.Number = 1;
        var c3 = Fake.Chapter(title.Id); c3.Number = 2;
        await Seed.ChapterAsync(db, title.Id, c1);
        await Seed.ChapterAsync(db, title.Id, c2);
        await Seed.ChapterAsync(db, title.Id, c3);
        Authorize(client, user);

        var res = await client.GetFromJsonAsync<JsonElement>($"/api/chapters/title/{title.Id}");
        Assert.Equal(3, res.GetArrayLength());
        Assert.Equal(1, res[0].GetProperty("number").GetDouble());
        Assert.Equal(2, res[1].GetProperty("number").GetDouble());
        Assert.Equal(3, res[2].GetProperty("number").GetDouble());
    }

    [Fact]
    public async Task ListChapters_ReturnsEmpty_NoChapters()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        var title = await Seed.TitleAsync(db, user.Id);
        Authorize(client, user);

        var res = await client.GetFromJsonAsync<JsonElement>($"/api/chapters/title/{title.Id}");
        Assert.Equal(0, res.GetArrayLength());
    }

    [Fact]
    public async Task ListChapters_HasSources_True_WhenChapterSourceExists()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        var title = await Seed.TitleAsync(db, user.Id);
        var chapter = await Seed.ChapterAsync(db, title.Id);
        db.ChapterSources.Add(new ChapterSource
        {
            Id = Guid.NewGuid().ToString(),
            ChapterId = chapter.Id,
            Source = "mangadex",
            SourceId = Guid.NewGuid().ToString(),
        });
        await db.SaveChangesAsync();
        Authorize(client, user);

        var res = await client.GetFromJsonAsync<JsonElement>($"/api/chapters/title/{title.Id}");
        Assert.True(res[0].GetProperty("has_sources").GetBoolean());
    }

    [Fact]
    public async Task ListChapters_HasSources_False_WhenNoChapterSource()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        var title = await Seed.TitleAsync(db, user.Id);
        await Seed.ChapterAsync(db, title.Id);
        Authorize(client, user);

        var res = await client.GetFromJsonAsync<JsonElement>($"/api/chapters/title/{title.Id}");
        Assert.False(res[0].GetProperty("has_sources").GetBoolean());
    }

    [Fact]
    public async Task ListChapters_HidesExplicitTitle_WhenUserNotAllowed()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.MemberUser();
        await Seed.UserAsync(db, user);
        var title = Fake.Title(isExplicit: true);
        await Seed.TitleAsync(db, user.Id, title);
        await Seed.ChapterAsync(db, title.Id);
        Authorize(client, user, allowExplicit: false);

        var res = await client.GetFromJsonAsync<JsonElement>($"/api/chapters/title/{title.Id}");
        Assert.Equal(0, res.GetArrayLength());
    }

    [Fact]
    public async Task ListChapters_ShowsExplicitTitle_WhenUserAllowed()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        var title = Fake.Title(isExplicit: true);
        await Seed.TitleAsync(db, user.Id, title);
        await Seed.ChapterAsync(db, title.Id);
        Authorize(client, user, allowExplicit: true);

        var res = await client.GetFromJsonAsync<JsonElement>($"/api/chapters/title/{title.Id}");
        Assert.Equal(1, res.GetArrayLength());
    }

    [Fact]
    public async Task ListChapters_Unauthorized_NoToken()
    {
        var (client, _) = NewFactory().CreateClientWithDb();
        var res = await client.GetAsync("/api/chapters/title/any");
        Assert.Equal(HttpStatusCode.Unauthorized, res.StatusCode);
    }

    // ── GET /api/chapters/{id} ────────────────────────────────────────────────

    [Fact]
    public async Task GetChapter_ReturnsChapter()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        var title = await Seed.TitleAsync(db, user.Id);
        var chapter = await Seed.ChapterAsync(db, title.Id);
        Authorize(client, user);

        var res = await client.GetFromJsonAsync<JsonElement>($"/api/chapters/{chapter.Id}");
        Assert.Equal(chapter.Id, res.GetProperty("id").GetString());
        Assert.Equal(chapter.Number, res.GetProperty("number").GetDouble());
    }

    [Fact]
    public async Task GetChapter_NotFound_Nonexistent()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        Authorize(client, user);

        var res = await client.GetAsync("/api/chapters/ghost");
        Assert.Equal(HttpStatusCode.NotFound, res.StatusCode);
    }

    [Fact]
    public async Task GetChapter_NotFound_ExplicitHiddenFromUser()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.MemberUser();
        await Seed.UserAsync(db, user);
        var title = Fake.Title(isExplicit: true);
        await Seed.TitleAsync(db, user.Id, title);
        var chapter = await Seed.ChapterAsync(db, title.Id);
        Authorize(client, user, allowExplicit: false);

        var res = await client.GetAsync($"/api/chapters/{chapter.Id}");
        Assert.Equal(HttpStatusCode.NotFound, res.StatusCode);
    }

    // ── GET /api/chapters/{id}/text ───────────────────────────────────────────

    [Fact]
    public async Task GetChapterText_BadRequest_WhenNotTextFormat()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        var title = await Seed.TitleAsync(db, user.Id);
        var chapter = Fake.Chapter(title.Id); chapter.ChapterFormat = "pages";
        await Seed.ChapterAsync(db, title.Id, chapter);
        Authorize(client, user);

        var res = await client.GetAsync($"/api/chapters/{chapter.Id}/text");
        Assert.Equal(HttpStatusCode.BadRequest, res.StatusCode);
    }

    [Fact]
    public async Task GetChapterText_NotFound_WhenNotDownloaded()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        var title = await Seed.TitleAsync(db, user.Id);
        var chapter = Fake.Chapter(title.Id);
        chapter.ChapterFormat = "text";
        chapter.Downloaded = false;
        await Seed.ChapterAsync(db, title.Id, chapter);
        Authorize(client, user);

        var res = await client.GetAsync($"/api/chapters/{chapter.Id}/text");
        Assert.Equal(HttpStatusCode.NotFound, res.StatusCode);
    }

    [Fact]
    public async Task GetChapterText_NotFound_WhenFileGone()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        var title = await Seed.TitleAsync(db, user.Id);
        var chapter = Fake.Chapter(title.Id);
        chapter.ChapterFormat = "text";
        chapter.Downloaded = true;
        chapter.LocalPath = "/nonexistent/path/novel.txt";
        await Seed.ChapterAsync(db, title.Id, chapter);
        Authorize(client, user);

        var res = await client.GetAsync($"/api/chapters/{chapter.Id}/text");
        Assert.Equal(HttpStatusCode.NotFound, res.StatusCode);
    }

    [Fact]
    public async Task GetChapterText_ReturnsContent_WhenFileExists()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        var title = await Seed.TitleAsync(db, user.Id);

        var tmpFile = Path.GetTempFileName();
        await File.WriteAllTextAsync(tmpFile, "Chapter one content.");

        var chapter = Fake.Chapter(title.Id);
        chapter.ChapterFormat = "text";
        chapter.Downloaded = true;
        chapter.LocalPath = tmpFile;
        await Seed.ChapterAsync(db, title.Id, chapter);
        Authorize(client, user);

        try
        {
            var res = await client.GetFromJsonAsync<JsonElement>($"/api/chapters/{chapter.Id}/text");
            Assert.Equal("Chapter one content.", res.GetProperty("content").GetString());
        }
        finally
        {
            File.Delete(tmpFile);
        }
    }

    [Fact]
    public async Task GetChapterText_NotFound_Nonexistent()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        Authorize(client, user);

        var res = await client.GetAsync("/api/chapters/ghost/text");
        Assert.Equal(HttpStatusCode.NotFound, res.StatusCode);
    }

    // ── POST /api/chapters/{id}/download ──────────────────────────────────────

    [Fact]
    public async Task QueueDownload_Accepted_WhenEligible()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        var title = await Seed.TitleAsync(db, user.Id);
        var chapter = await Seed.ChapterAsync(db, title.Id);
        db.ChapterSources.Add(new ChapterSource
        {
            Id = Guid.NewGuid().ToString(),
            ChapterId = chapter.Id,
            Source = "mangadex",
            SourceId = Guid.NewGuid().ToString(),
        });
        await db.SaveChangesAsync();
        Authorize(client, user);

        var res = await client.PostAsync($"/api/chapters/{chapter.Id}/download", null);
        Assert.Equal(HttpStatusCode.Accepted, res.StatusCode);

        db.ChangeTracker.Clear();
        var queued = await db.DownloadQueue.FirstOrDefaultAsync(q => q.ChapterId == chapter.Id);
        Assert.NotNull(queued);
        Assert.Equal("pending", queued!.Status);
        Assert.Equal(user.Id, queued.QueuedBy);
    }

    [Fact]
    public async Task QueueDownload_NotFound_WhenAlreadyDownloaded()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        var title = await Seed.TitleAsync(db, user.Id);
        var chapter = Fake.Chapter(title.Id); chapter.Downloaded = true;
        await Seed.ChapterAsync(db, title.Id, chapter);
        db.ChapterSources.Add(new ChapterSource
        {
            Id = Guid.NewGuid().ToString(), ChapterId = chapter.Id,
            Source = "mangadex", SourceId = Guid.NewGuid().ToString(),
        });
        await db.SaveChangesAsync();
        Authorize(client, user);

        var res = await client.PostAsync($"/api/chapters/{chapter.Id}/download", null);
        Assert.Equal(HttpStatusCode.NotFound, res.StatusCode);
    }

    [Fact]
    public async Task QueueDownload_NotFound_WhenNoChapterSource()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        var title = await Seed.TitleAsync(db, user.Id);
        var chapter = await Seed.ChapterAsync(db, title.Id); // no chapter_sources
        Authorize(client, user);

        var res = await client.PostAsync($"/api/chapters/{chapter.Id}/download", null);
        Assert.Equal(HttpStatusCode.NotFound, res.StatusCode);
    }

    [Fact]
    public async Task QueueDownload_NotFound_ExplicitHiddenFromUser()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.MemberUser();
        await Seed.UserAsync(db, user);
        var title = Fake.Title(isExplicit: true);
        await Seed.TitleAsync(db, user.Id, title);
        var chapter = await Seed.ChapterAsync(db, title.Id);
        db.ChapterSources.Add(new ChapterSource
        {
            Id = Guid.NewGuid().ToString(), ChapterId = chapter.Id,
            Source = "mangadex", SourceId = Guid.NewGuid().ToString(),
        });
        await db.SaveChangesAsync();
        Authorize(client, user, allowExplicit: false);

        var res = await client.PostAsync($"/api/chapters/{chapter.Id}/download", null);
        Assert.Equal(HttpStatusCode.NotFound, res.StatusCode);
    }

    [Fact]
    public async Task QueueDownload_ReQueues_WhenPreviouslyErrored()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        var title = await Seed.TitleAsync(db, user.Id);
        var chapter = await Seed.ChapterAsync(db, title.Id);
        db.ChapterSources.Add(new ChapterSource
        {
            Id = Guid.NewGuid().ToString(), ChapterId = chapter.Id,
            Source = "mangadex", SourceId = Guid.NewGuid().ToString(),
        });
        // Seed an errored queue item
        db.DownloadQueue.Add(new DownloadQueueItem
        {
            Id = Guid.NewGuid().ToString(), ChapterId = chapter.Id,
            MangaTitle = title.TitleName, ChapterNum = chapter.Number,
            Status = "error", Error = "timeout",
            CreatedAt = DateTime.UtcNow, UpdatedAt = DateTime.UtcNow,
        });
        await db.SaveChangesAsync();
        Authorize(client, user);

        var res = await client.PostAsync($"/api/chapters/{chapter.Id}/download", null);
        Assert.Equal(HttpStatusCode.Accepted, res.StatusCode);

        db.ChangeTracker.Clear();
        var queued = await db.DownloadQueue.FirstAsync(q => q.ChapterId == chapter.Id);
        Assert.Equal("pending", queued.Status);
        Assert.Null(queued.Error);
    }

    [Fact]
    public async Task QueueDownload_Unauthorized_NoToken()
    {
        var (client, _) = NewFactory().CreateClientWithDb();
        var res = await client.PostAsync("/api/chapters/any/download", null);
        Assert.Equal(HttpStatusCode.Unauthorized, res.StatusCode);
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
