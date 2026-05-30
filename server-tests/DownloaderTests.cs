using System.Net;
using System.Net.Http.Headers;
using ArrghServer.Api;
using ArrghServer.Data;
using ArrghServer.Data.Models;
using Microsoft.EntityFrameworkCore;
using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Microsoft.Extensions.Hosting;
using Xunit;

namespace ArrghServer.Tests;

// Integration tests for DownloaderService.
// Each test uses DownloadFactory which fakes plugin-host pages/text responses and
// wires the real DownloaderService hosted service.

[Trait("Category", TestCategories.Integration)]
public class DownloaderTests
{
    // ── Factory helpers ───────────────────────────────────────────────────────

    static DownloadFactory NewFactory(
        string[]? pageUrls = null,
        bool pagesThrows = false,
        string? chapterText = null) =>
        new(pageUrls ?? ["http://plugin-host/image.jpg"], pagesThrows, chapterText ?? "Chapter text content.");

    void Authorize(HttpClient client, User user)
    {
        var config = new ConfigurationBuilder()
            .AddInMemoryCollection(new Dictionary<string, string?> { ["JwtSecret"] = AppFactory.JwtSecret })
            .Build();
        var token = Auth.CreateToken(user.Id, user.Username, user.Role, user.AllowExplicit, config);
        client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue("Bearer", token);
    }

    // Poll until queue item reaches a terminal status
    static async Task WaitForQueueDoneAsync(AppDbContext db, string queueId, int maxMs = 10_000)
    {
        var sw = System.Diagnostics.Stopwatch.StartNew();
        while (sw.ElapsedMilliseconds < maxMs)
        {
            db.ChangeTracker.Clear();
            var status = await db.DownloadQueue.Where(q => q.Id == queueId).Select(q => q.Status).FirstOrDefaultAsync();
            if (status is "done" or "error") return;
            await Task.Delay(50);
        }
    }

    static async Task<string> QueueChapterAsync(AppDbContext db, string chapterId, string mangaTitle, double chapterNum)
    {
        var item = await Seed.QueueItemAsync(db, chapterId, mangaTitle, chapterNum, status: "pending");
        return item.Id;
    }

    // ── Manga (pages) download ────────────────────────────────────────────────

    [Fact]
    public async Task Download_PendingMangaChapter_SetsDone_AndChapterDownloaded()
    {
        var (_, db) = NewFactory().CreateClientWithDb();

        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        var title = await Seed.TitleAsync(db, user.Id, Fake.Title());
        var chapter = Fake.Chapter(title.Id, number: 1.0);
        chapter.ChapterFormat = "pages";
        await Seed.ChapterAsync(db, title.Id, chapter);

        // Seed a chapter source so the downloader knows which plugin to call
        db.ChapterSources.Add(new ChapterSource
        {
            Id = Guid.NewGuid().ToString(),
            ChapterId = chapter.Id,
            Source = "mangadex",
            SourceId = "ext-ch-1",
        });
        await db.SaveChangesAsync();

        var queueId = await QueueChapterAsync(db, chapter.Id, title.Id, 1.0);

        await WaitForQueueDoneAsync(db, queueId);
        db.ChangeTracker.Clear();

        var status = await db.DownloadQueue.Where(q => q.Id == queueId).Select(q => q.Status).FirstAsync();
        Assert.Equal("done", status);

        var downloaded = await db.Chapters.Where(c => c.Id == chapter.Id).Select(c => c.Downloaded).FirstAsync();
        Assert.True(downloaded);
    }

    [Fact]
    public async Task Download_PageChapter_SetsLocalPath()
    {
        var (_, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        var title = await Seed.TitleAsync(db, user.Id, Fake.Title());
        var chapter = Fake.Chapter(title.Id, number: 2.0);
        chapter.ChapterFormat = "pages";
        await Seed.ChapterAsync(db, title.Id, chapter);
        db.ChapterSources.Add(new ChapterSource
        {
            Id = Guid.NewGuid().ToString(), ChapterId = chapter.Id, Source = "mangadex", SourceId = "ext-ch-2",
        });
        await db.SaveChangesAsync();

        var queueId = await QueueChapterAsync(db, chapter.Id, title.Id, 2.0);
        await WaitForQueueDoneAsync(db, queueId);
        db.ChangeTracker.Clear();

        var localPath = await db.Chapters.Where(c => c.Id == chapter.Id).Select(c => c.LocalPath).FirstAsync();
        Assert.NotNull(localPath);
        Assert.EndsWith(".cbz", localPath);
    }

    [Fact]
    public async Task Download_PageChapter_SetsPageCount()
    {
        // Factory returns 3 page URLs — PageCount should be 3
        var (_, db) = NewFactory(pageUrls: ["http://plugin-host/p1.jpg", "http://plugin-host/p2.jpg", "http://plugin-host/p3.jpg"]).CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        var title = await Seed.TitleAsync(db, user.Id, Fake.Title());
        var chapter = Fake.Chapter(title.Id, number: 3.0);
        chapter.ChapterFormat = "pages";
        await Seed.ChapterAsync(db, title.Id, chapter);
        db.ChapterSources.Add(new ChapterSource
        {
            Id = Guid.NewGuid().ToString(), ChapterId = chapter.Id, Source = "mangadex", SourceId = "ext-ch-3",
        });
        await db.SaveChangesAsync();

        var queueId = await QueueChapterAsync(db, chapter.Id, title.Id, 3.0);
        await WaitForQueueDoneAsync(db, queueId);
        db.ChangeTracker.Clear();

        var pageCount = await db.Chapters.Where(c => c.Id == chapter.Id).Select(c => c.PageCount).FirstAsync();
        Assert.Equal(3, pageCount);
    }

    // ── Novel (text) download ─────────────────────────────────────────────────

    [Fact]
    public async Task Download_TextChapter_SetsDoneAndDownloaded()
    {
        var (_, db) = NewFactory(chapterText: "# Chapter 1\nOnce upon a time...").CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        var title = await Seed.TitleAsync(db, user.Id, Fake.Title());
        var chapter = Fake.Chapter(title.Id, number: 1.0);
        chapter.ChapterFormat = "text";
        await Seed.ChapterAsync(db, title.Id, chapter);
        db.ChapterSources.Add(new ChapterSource
        {
            Id = Guid.NewGuid().ToString(), ChapterId = chapter.Id, Source = "boxnovel", SourceId = "ext-text-1",
        });
        await db.SaveChangesAsync();

        var queueId = await QueueChapterAsync(db, chapter.Id, title.Id, 1.0);
        await WaitForQueueDoneAsync(db, queueId);
        db.ChangeTracker.Clear();

        var q = await db.DownloadQueue.Where(q => q.Id == queueId).Select(q => q.Status).FirstAsync();
        Assert.Equal("done", q);

        var ch = await db.Chapters.Where(c => c.Id == chapter.Id).FirstAsync();
        Assert.True(ch.Downloaded);
        Assert.NotNull(ch.LocalPath);
        Assert.EndsWith(".md", ch.LocalPath);
    }

    // ── Error cases ───────────────────────────────────────────────────────────

    [Fact]
    public async Task Download_NoChapterSources_SetsError()
    {
        var (_, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        var title = await Seed.TitleAsync(db, user.Id, Fake.Title());
        var chapter = Fake.Chapter(title.Id, number: 1.0);
        chapter.ChapterFormat = "pages";
        await Seed.ChapterAsync(db, title.Id, chapter);
        // No chapter_sources seeded

        var queueId = await QueueChapterAsync(db, chapter.Id, title.Id, 1.0);
        await WaitForQueueDoneAsync(db, queueId);
        db.ChangeTracker.Clear();

        var status = await db.DownloadQueue.Where(q => q.Id == queueId).Select(q => q.Status).FirstAsync();
        Assert.Equal("error", status);

        var error = await db.DownloadQueue.Where(q => q.Id == queueId).Select(q => q.Error).FirstAsync();
        Assert.NotNull(error);
    }

    [Fact]
    public async Task Download_AllSourcesFail_SetsError()
    {
        var (_, db) = NewFactory(pagesThrows: true).CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        var title = await Seed.TitleAsync(db, user.Id, Fake.Title());
        var chapter = Fake.Chapter(title.Id, number: 1.0);
        chapter.ChapterFormat = "pages";
        await Seed.ChapterAsync(db, title.Id, chapter);
        db.ChapterSources.Add(new ChapterSource
        {
            Id = Guid.NewGuid().ToString(), ChapterId = chapter.Id, Source = "mangadex", SourceId = "ext-fail",
        });
        await db.SaveChangesAsync();

        var queueId = await QueueChapterAsync(db, chapter.Id, title.Id, 1.0);
        await WaitForQueueDoneAsync(db, queueId);
        db.ChangeTracker.Clear();

        var status = await db.DownloadQueue.Where(q => q.Id == queueId).Select(q => q.Status).FirstAsync();
        Assert.Equal("error", status);
    }

    [Fact]
    public async Task Download_SecondSourceSucceeds_WhenFirstFails()
    {
        // Two chapter_sources: first has wrong source key (no match) → second works
        var (_, db) = NewFactory().CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);

        // Seed external_source priority: bad-source priority 10, mangadex priority 20
        db.ExternalSources.Add(new ExternalSource
        {
            Id = Guid.NewGuid().ToString(), SourceKey = "bad-source", Name = "Bad", BaseUrl = "http://plugin-host",
            ContentTypes = "manga", Enabled = true, Priority = 10, CreatedAt = DateTime.UtcNow,
        });
        db.ExternalSources.Add(new ExternalSource
        {
            Id = Guid.NewGuid().ToString(), SourceKey = "mangadex", Name = "MangaDex", BaseUrl = "http://plugin-host",
            ContentTypes = "manga", Enabled = true, Priority = 20, CreatedAt = DateTime.UtcNow,
        });
        await db.SaveChangesAsync();

        var title = await Seed.TitleAsync(db, user.Id, Fake.Title());
        var chapter = Fake.Chapter(title.Id, number: 1.0);
        chapter.ChapterFormat = "pages";
        await Seed.ChapterAsync(db, title.Id, chapter);

        // bad-source has priority 10 (tried first) but fake handler returns 502 for unknown sources
        db.ChapterSources.Add(new ChapterSource { Id = Guid.NewGuid().ToString(), ChapterId = chapter.Id, Source = "bad-source", SourceId = "ext-bad" });
        // mangadex has priority 20 — fake handler recognises it and returns pages
        db.ChapterSources.Add(new ChapterSource { Id = Guid.NewGuid().ToString(), ChapterId = chapter.Id, Source = "mangadex", SourceId = "ext-good" });
        await db.SaveChangesAsync();

        var queueId = await QueueChapterAsync(db, chapter.Id, title.Id, 1.0);
        await WaitForQueueDoneAsync(db, queueId);
        db.ChangeTracker.Clear();

        var status = await db.DownloadQueue.Where(q => q.Id == queueId).Select(q => q.Status).FirstAsync();
        Assert.Equal("done", status);
    }

    // ── Error context (Bug: 400 with no URL in the error message) ─────────────

    [Fact]
    public async Task Download_PagesEndpointError_ErrorMessageContainsUrl()
    {
        // When plugin-host pages endpoint returns non-200 (e.g. 400), the error stored
        // in the queue must include the request URL — not just the bare status code.
        var (_, db) = new DownloadFactory(["http://plugin-host/image.jpg"], pagesThrows: false, "text",
            pagesReturn400: true).CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        var title = await Seed.TitleAsync(db, user.Id, Fake.Title());
        var chapter = Fake.Chapter(title.Id, number: 1.0);
        chapter.ChapterFormat = "pages";
        await Seed.ChapterAsync(db, title.Id, chapter);
        db.ChapterSources.Add(new ChapterSource
        {
            Id = Guid.NewGuid().ToString(), ChapterId = chapter.Id, Source = "mangadex", SourceId = "ext-bad-1",
        });
        await db.SaveChangesAsync();

        var queueId = await QueueChapterAsync(db, chapter.Id, title.Id, 1.0);
        await WaitForQueueDoneAsync(db, queueId);
        db.ChangeTracker.Clear();

        var error = await db.DownloadQueue.Where(q => q.Id == queueId).Select(q => q.Error).FirstAsync();
        Assert.NotNull(error);
        // Error must include the plugin-host URL so the user can diagnose which endpoint failed
        Assert.Contains("mangadex", error);
        Assert.Contains("/chapter/", error);
    }

    [Fact]
    public async Task Download_ImageError_ErrorMessageContainsUrl()
    {
        // When an image download returns non-200, the error must contain the image URL.
        var badImageUrl = "http://plugin-host/bad-image.jpg";
        var (_, db) = new DownloadFactory([badImageUrl], pagesThrows: false, "text").CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        var title = await Seed.TitleAsync(db, user.Id, Fake.Title());
        var chapter = Fake.Chapter(title.Id, number: 1.0);
        chapter.ChapterFormat = "pages";
        await Seed.ChapterAsync(db, title.Id, chapter);
        db.ChapterSources.Add(new ChapterSource
        {
            Id = Guid.NewGuid().ToString(), ChapterId = chapter.Id, Source = "mangadex", SourceId = "ext-img-fail",
        });
        await db.SaveChangesAsync();

        var queueId = await QueueChapterAsync(db, chapter.Id, title.Id, 1.0);
        await WaitForQueueDoneAsync(db, queueId);
        db.ChangeTracker.Clear();

        var error = await db.DownloadQueue.Where(q => q.Id == queueId).Select(q => q.Error).FirstAsync();
        Assert.NotNull(error);
        Assert.Contains("bad-image.jpg", error);
    }

    // ── Status contract ───────────────────────────────────────────────────────

    [Fact]
    public async Task Download_StatusIsDownloading_NotInProgress_WhileProcessing()
    {
        // The frontend QueueRow/ChapterRow components check for status == "downloading".
        // If the service sets "in_progress" instead, polling stops and the UI freezes.
        var imageStarted = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var allowContinue = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);

        var factory = new BlockingDownloadFactory(imageStarted, allowContinue);
        var (_, db) = factory.CreateClientWithDb();

        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        var title = await Seed.TitleAsync(db, user.Id, Fake.Title());
        var chapter = Fake.Chapter(title.Id, number: 1.0);
        chapter.ChapterFormat = "pages";
        await Seed.ChapterAsync(db, title.Id, chapter);
        db.ChapterSources.Add(new ChapterSource
        {
            Id = Guid.NewGuid().ToString(), ChapterId = chapter.Id, Source = "mangadex", SourceId = "blocking-1",
        });
        await db.SaveChangesAsync();

        var queueId = await QueueChapterAsync(db, chapter.Id, title.Id, 1.0);

        // Wait until the handler has started downloading the image (tick has claimed the item)
        await imageStarted.Task.WaitAsync(TimeSpan.FromSeconds(10));

        // The status must be "downloading" — NOT "in_progress"
        db.ChangeTracker.Clear();
        var status = await db.DownloadQueue.Where(q => q.Id == queueId).Select(q => q.Status).FirstAsync();
        Assert.Equal("downloading", status);

        // Unblock so the test can clean up
        allowContinue.SetResult();
        await WaitForQueueDoneAsync(db, queueId);
    }

    [Fact]
    public async Task Download_RequestHasUserAgent()
    {
        // Downloader must send a User-Agent header; some CDNs return 400/403 without one.
        string? capturedUserAgent = null;
        var (_, db) = new UserAgentCapturingFactory(ua => capturedUserAgent = ua).CreateClientWithDb();
        var user = Fake.AdminUser();
        await Seed.UserAsync(db, user);
        var title = await Seed.TitleAsync(db, user.Id, Fake.Title());
        var chapter = Fake.Chapter(title.Id, number: 1.0);
        chapter.ChapterFormat = "pages";
        await Seed.ChapterAsync(db, title.Id, chapter);
        db.ChapterSources.Add(new ChapterSource
        {
            Id = Guid.NewGuid().ToString(), ChapterId = chapter.Id, Source = "mangadex", SourceId = "ua-test",
        });
        await db.SaveChangesAsync();

        var queueId = await QueueChapterAsync(db, chapter.Id, title.Id, 1.0);
        await WaitForQueueDoneAsync(db, queueId);

        Assert.NotNull(capturedUserAgent);
        Assert.NotEmpty(capturedUserAgent);
    }
}

// ── DownloadFactory ───────────────────────────────────────────────────────────

public class DownloadFactory(
    string[] pageUrls, bool pagesThrows, string chapterText,
    bool pagesReturn400 = false) : AppFactory
{
    protected override IHost CreateHost(IHostBuilder builder)
    {
        var urls = pageUrls;
        var throws = pagesThrows;
        var text = chapterText;
        var return400 = pagesReturn400;
        var downloadDir = Path.Combine(Path.GetTempPath(), $"arrgh-dl-{Guid.NewGuid():N}");

        builder.ConfigureServices(services =>
        {
            services.RemoveAll<IHttpClientFactory>();
            services.AddSingleton<IHttpClientFactory>(
                new DownloadFakeHttpClientFactory(urls, throws, text, return400));
        });

        builder.ConfigureAppConfiguration(cfg =>
            cfg.AddInMemoryCollection(new Dictionary<string, string?> { ["DownloadDir"] = downloadDir }));

        return base.CreateHost(builder);
    }
}

// Factory that captures the User-Agent header from the first outbound request.
public class UserAgentCapturingFactory(Action<string?> onCapture) : AppFactory
{
    protected override IHost CreateHost(IHostBuilder builder)
    {
        var downloadDir = Path.Combine(Path.GetTempPath(), $"arrgh-dl-ua-{Guid.NewGuid():N}");
        builder.ConfigureServices(services =>
        {
            services.RemoveAll<IHttpClientFactory>();
            services.AddSingleton<IHttpClientFactory>(new UaCapturingHttpClientFactory(onCapture));
        });
        builder.ConfigureAppConfiguration(cfg =>
            cfg.AddInMemoryCollection(new Dictionary<string, string?> { ["DownloadDir"] = downloadDir }));
        return base.CreateHost(builder);
    }
}

file class UaCapturingHttpClientFactory(Action<string?> onCapture) : IHttpClientFactory
{
    public HttpClient CreateClient(string name) => new(new UaCapturingHandler(onCapture));
}

file class UaCapturingHandler(Action<string?> onCapture) : HttpMessageHandler
{
    static readonly byte[] _jpeg = DownloadFakeHandler.TinyJpeg;

    protected override Task<HttpResponseMessage> SendAsync(HttpRequestMessage req, CancellationToken ct)
    {
        var path = req.RequestUri?.PathAndQuery ?? "";
        if (path.Contains("/chapter/") && path.EndsWith("/pages"))
        {
            // Capture User-Agent from the image-download request context
            onCapture(req.Headers.UserAgent.ToString());
            var json = System.Text.Json.JsonSerializer.Serialize(new[] { "http://plugin-host/image.jpg" });
            return Task.FromResult(new HttpResponseMessage(HttpStatusCode.OK)
            {
                Content = new StringContent(json, System.Text.Encoding.UTF8, "application/json"),
            });
        }
        if (path.EndsWith(".jpg"))
        {
            // Capture User-Agent on the actual image download
            onCapture(req.Headers.UserAgent.ToString());
            return Task.FromResult(new HttpResponseMessage(HttpStatusCode.OK)
            {
                Content = new ByteArrayContent(_jpeg),
            });
        }
        return Task.FromResult(new HttpResponseMessage(HttpStatusCode.OK)
        {
            Content = new ByteArrayContent([]),
        });
    }
}

file class DownloadFakeHttpClientFactory(
    string[] pageUrls, bool pagesThrows, string chapterText, bool pagesReturn400)
    : IHttpClientFactory
{
    public HttpClient CreateClient(string name) =>
        new(new DownloadFakeHandler(pageUrls, pagesThrows, chapterText, pagesReturn400));
}

// ── BlockingDownloadFactory ───────────────────────────────────────────────────
// Returns pages immediately, then pauses on the first image download so the test
// can observe the intermediate queue status before the download completes.

public class BlockingDownloadFactory(
    TaskCompletionSource imageStarted,
    TaskCompletionSource allowContinue) : AppFactory
{
    protected override IHost CreateHost(IHostBuilder builder)
    {
        var downloadDir = Path.Combine(Path.GetTempPath(), $"arrgh-dl-block-{Guid.NewGuid():N}");
        builder.ConfigureServices(services =>
        {
            services.RemoveAll<IHttpClientFactory>();
            services.AddSingleton<IHttpClientFactory>(
                new BlockingHttpClientFactory(imageStarted, allowContinue));
        });
        builder.ConfigureAppConfiguration(cfg =>
            cfg.AddInMemoryCollection(new Dictionary<string, string?> { ["DownloadDir"] = downloadDir }));
        return base.CreateHost(builder);
    }
}

file class BlockingHttpClientFactory(
    TaskCompletionSource imageStarted,
    TaskCompletionSource allowContinue) : IHttpClientFactory
{
    public HttpClient CreateClient(string name) =>
        new(new BlockingHandler(imageStarted, allowContinue));
}

file class BlockingHandler(
    TaskCompletionSource imageStarted,
    TaskCompletionSource allowContinue) : HttpMessageHandler
{
    static readonly byte[] _jpeg = DownloadFakeHandler.TinyJpeg;

    protected override async Task<HttpResponseMessage> SendAsync(HttpRequestMessage req, CancellationToken ct)
    {
        var path = req.RequestUri?.PathAndQuery ?? "";

        if (path.Contains("/chapter/") && path.EndsWith("/pages"))
        {
            var json = System.Text.Json.JsonSerializer.Serialize(new[] { "http://plugin-host/image.jpg" });
            return new HttpResponseMessage(System.Net.HttpStatusCode.OK)
            {
                Content = new StringContent(json, System.Text.Encoding.UTF8, "application/json"),
            };
        }

        if (path.EndsWith(".jpg") || path.EndsWith(".jpeg") || path.EndsWith(".png"))
        {
            // Signal that image download started, then block until the test allows continuation
            imageStarted.TrySetResult();
            await allowContinue.Task.WaitAsync(ct);
            return new HttpResponseMessage(System.Net.HttpStatusCode.OK)
            {
                Content = new ByteArrayContent(_jpeg),
            };
        }

        return new HttpResponseMessage(System.Net.HttpStatusCode.OK)
        {
            Content = new ByteArrayContent([]),
        };
    }
}

file class DownloadFakeHandler(
    string[] pageUrls, bool pagesThrows, string chapterText, bool pagesReturn400 = false)
    : HttpMessageHandler
{
    internal static readonly byte[] TinyJpeg = Convert.FromBase64String(
        "/9j/4AAQSkZJRgABAQEASABIAAD/2wBDAAgGBgcGBQgHBwcJCQgKDBQNDAsLDBkSEw8U" +
        "HRofHh0aHBwgJC4nICIsIxwcKDcpLDAxNDQ0Hyc5PTgyPC4zNDL/wAARCAABAAEDASIA" +
        "AhEBAxEB/8QAFAABAAAAAAAAAAAAAAAAAAAACf/EABQQAQAAAAAAAAAAAAAAAAAAAAD/xAAU" +
        "AQEAAAAAAAAAAAAAAAAAAAAA/8QAFBEBAAAAAAAAAAAAAAAAAAAAAP/aAAwDAQACEQMRAD8A" +
        "JQAB/9k=");

    protected override Task<HttpResponseMessage> SendAsync(HttpRequestMessage req, CancellationToken ct)
    {
        var path = req.RequestUri?.PathAndQuery ?? "";

        // Plugin-host pages endpoint: /{source}/chapter/{id}/pages
        if (path.Contains("/chapter/") && path.EndsWith("/pages"))
        {
            if (pagesThrows) throw new HttpRequestException("plugin-host unavailable");
            if (pagesReturn400) return Task.FromResult(new HttpResponseMessage(HttpStatusCode.BadRequest));
            if (path.StartsWith("/bad-source/") || path.StartsWith("/unknown/"))
                return Task.FromResult(new HttpResponseMessage(HttpStatusCode.BadGateway));
            var json = System.Text.Json.JsonSerializer.Serialize(pageUrls);
            return Task.FromResult(Json(json));
        }

        // Plugin-host text endpoint: /{source}/chapter/{id}/text
        if (path.Contains("/chapter/") && path.EndsWith("/text"))
        {
            return Task.FromResult(new HttpResponseMessage(HttpStatusCode.OK)
            {
                Content = new StringContent(chapterText, System.Text.Encoding.UTF8, "text/plain"),
            });
        }

        // Image download — bad-image.jpg → 400, everything else → 200 JPEG
        if (path.EndsWith(".jpg") || path.EndsWith(".jpeg") || path.EndsWith(".png"))
        {
            if (path.Contains("bad-image"))
                return Task.FromResult(new HttpResponseMessage(HttpStatusCode.BadRequest));
            return Task.FromResult(new HttpResponseMessage(HttpStatusCode.OK)
            {
                Content = new ByteArrayContent(TinyJpeg),
            });
        }

        // Everything else → 200 empty
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
