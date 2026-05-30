using ArrghServer.Data;
using ArrghServer.Data.Models;
using Bogus;
using Microsoft.AspNetCore.Hosting;
using Microsoft.AspNetCore.Mvc.Testing;
using Microsoft.EntityFrameworkCore;
using Microsoft.Extensions.Configuration;
using System.Text;
using Microsoft.AspNetCore.Authentication.JwtBearer;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Microsoft.Extensions.Hosting;
using Microsoft.IdentityModel.Tokens;

namespace ArrghServer.Tests;

/// <summary>
/// Spins up the full ASP.NET pipeline against an isolated in-memory SQLite DB.
/// Mirrors the Rust integration test build_state() pattern.
/// </summary>
public class AppFactory : WebApplicationFactory<Program>
{
    public const string JwtSecret = "integration-test-secret-32chars!";

    // Each factory instance gets its own isolated DB file
    readonly string _dbPath = Path.Combine(Path.GetTempPath(), $"arrgh-test-{Guid.NewGuid():N}.db");

    protected override void ConfigureWebHost(IWebHostBuilder builder)
    {
        // Override config before app reads it — only DatabasePath matters here since
        // JwtSecret is injected via PostConfigure below (avoids build-time ordering issues)
        builder.ConfigureAppConfiguration(config =>
            config.AddInMemoryCollection(new Dictionary<string, string?>
            {
                ["JwtSecret"]          = JwtSecret,   // needed so Program.cs doesn't throw on startup
                ["DatabasePath"]       = _dbPath,
                ["SeedDefaultSources"] = "false",     // tests start with empty external_sources
                ["PluginHostUrl"]      = "http://plugin-host:4000",  // override dev appsettings
            }));
    }

    protected override IHost CreateHost(IHostBuilder builder)
    {
        builder.ConfigureServices(services =>
        {
            // AddDbContext uses TryAdd internally — RemoveAll + direct Options registration
            // is required to actually override the connection string
            services.RemoveAll<DbContextOptions<AppDbContext>>();
            services.AddSingleton(new DbContextOptionsBuilder<AppDbContext>()
                .UseSqlite($"Data Source={_dbPath}")
                .Options);

            // Override JWT signing key — runs after app's own AddJwtBearer, so it wins
            services.PostConfigure<JwtBearerOptions>(JwtBearerDefaults.AuthenticationScheme, opt =>
            {
                opt.TokenValidationParameters.IssuerSigningKey =
                    new SymmetricSecurityKey(Encoding.UTF8.GetBytes(JwtSecret));
            });
        });
        return base.CreateHost(builder);
    }

    /// <summary>Creates an HttpClient with a migrated DB scope ready for seeding.</summary>
    public (HttpClient Client, AppDbContext Db) CreateClientWithDb()
    {
        var client = CreateClient();
        var scope = Services.CreateScope();
        var db = scope.ServiceProvider.GetRequiredService<AppDbContext>();
        db.Database.Migrate();
        return (client, db);
    }

    protected override void Dispose(bool disposing)
    {
        base.Dispose(disposing);
        if (disposing && File.Exists(_dbPath))
            try { File.Delete(_dbPath); } catch { }
    }
}

/// <summary>
/// Like AppFactory but re-enables default source seeding so tests can verify
/// the Program.cs ExternalSource seed values (e.g. content_types per source).
/// </summary>
public class SeededFactory : AppFactory
{
    protected override void ConfigureWebHost(IWebHostBuilder builder)
    {
        base.ConfigureWebHost(builder);
        // Override the SeedDefaultSources=false that AppFactory injects
        builder.ConfigureAppConfiguration(config =>
            config.AddInMemoryCollection(new Dictionary<string, string?> { ["SeedDefaultSources"] = "true" }));
    }
}

// ── Bogus fakers ─────────────────────────────────────────────────────────────

public static class Fake
{
    static readonly Faker F = new();

    public static User AdminUser(string? id = null) => new()
    {
        Id = id ?? Guid.NewGuid().ToString(),
        Username = F.Internet.UserName(),
        PasswordHash = BCrypt.Net.BCrypt.HashPassword("password123"),
        Role = "admin",
        AllowExplicit = true,
        CreatedAt = DateTime.UtcNow,
    };

    public static User MemberUser(string? id = null) => new()
    {
        Id = id ?? Guid.NewGuid().ToString(),
        Username = F.Internet.UserName(),
        PasswordHash = BCrypt.Net.BCrypt.HashPassword("password123"),
        Role = "member",
        AllowExplicit = false,
        CreatedAt = DateTime.UtcNow,
    };

    public static Title Title(string? id = null, bool isExplicit = false) => new()
    {
        Id = id ?? Guid.NewGuid().ToString(),
        TitleName = F.Lorem.Words(3).Aggregate((a, b) => $"{a} {b}"),
        Status = "unknown",
        SyncStatus = "ready",
        ContentType = F.PickRandom("manga", "manhwa", "manhua"),
        IsExplicit = isExplicit,
        CreatedAt = DateTime.UtcNow,
        UpdatedAt = DateTime.UtcNow,
        Author = F.Name.FullName(),
        Description = F.Lorem.Sentence(),
        CoverUrl = F.Internet.Url(),
    };

    public static Chapter Chapter(string titleId, string? id = null, double? number = null) => new()
    {
        Id = id ?? Guid.NewGuid().ToString(),
        TitleId = titleId,
        Number = number ?? F.Random.Double(1, 500),
        PageCount = F.Random.Int(5, 30),
        Downloaded = false,
        ChapterFormat = "pages",
        IsNew = false,
        CreatedAt = DateTime.UtcNow,
    };

    public static TitleSource TitleSource(string titleId, string? source = null) => new()
    {
        Id = Guid.NewGuid().ToString(),
        TitleId = titleId,
        Source = source ?? F.PickRandom("mangadex", "mangapill"),
        SourceId = Guid.NewGuid().ToString(),
        DiscoveredAt = DateTime.UtcNow,
    };
}

// ── Seed helpers ─────────────────────────────────────────────────────────────

public static class Seed
{
    public static async Task<User> UserAsync(AppDbContext db, User? user = null)
    {
        user ??= Fake.AdminUser();
        db.Users.Add(user);
        await db.SaveChangesAsync();
        return user;
    }

    public static async Task<Title> TitleAsync(AppDbContext db, string userId, Title? title = null)
    {
        title ??= Fake.Title();
        if (!db.Titles.Local.Any(t => t.Id == title.Id) &&
            !await db.Titles.AnyAsync(t => t.Id == title.Id))
            db.Titles.Add(title);
        db.UserTitles.Add(new UserTitle { UserId = userId, TitleId = title.Id, AddedAt = DateTime.UtcNow });
        await db.SaveChangesAsync();
        return title;
    }

    public static async Task<Chapter> ChapterAsync(AppDbContext db, string titleId, Chapter? chapter = null)
    {
        chapter ??= Fake.Chapter(titleId);
        db.Chapters.Add(chapter);
        await db.SaveChangesAsync();
        return chapter;
    }

    public static async Task MarkDownloadedAsync(AppDbContext db, string chapterId)
    {
        var c = await db.Chapters.FindAsync(chapterId);
        c!.Downloaded = true;
        await db.SaveChangesAsync();
    }

    public static async Task MarkReadAsync(AppDbContext db, string userId, string chapterId)
    {
        db.ReadProgresses.Add(new ReadProgress
        {
            Id = Guid.NewGuid().ToString(),
            UserId = userId,
            ChapterId = chapterId,
            Completed = true,
            CurrentPage = 0,
            UpdatedAt = DateTime.UtcNow,
        });
        await db.SaveChangesAsync();
    }

    public static async Task AddSyncWarningAsync(AppDbContext db, string titleId, string pluginId = "mangadex")
    {
        db.SyncWarnings.Add(new SyncWarning
        {
            Id = Guid.NewGuid().ToString(),
            TitleId = titleId,
            PluginId = pluginId,
            Message = "no match",
            CreatedAt = DateTime.UtcNow,
        });
        await db.SaveChangesAsync();
    }

    public static async Task AddSyncLogAsync(AppDbContext db, string titleId, string message = "synced")
    {
        db.SyncLogs.Add(new SyncLog
        {
            Id = Guid.NewGuid().ToString(),
            TitleId = titleId,
            Message = message,
            CreatedAt = DateTime.UtcNow,
        });
        await db.SaveChangesAsync();
    }

    public static async Task AddTitleSourceAsync(AppDbContext db, string titleId, string source = "mangadex")
    {
        db.TitleSources.Add(Fake.TitleSource(titleId, source));
        await db.SaveChangesAsync();
    }

    public static async Task<DownloadQueueItem> QueueItemAsync(
        AppDbContext db, string chapterId, string mangaTitle, double chapterNum,
        string status = "pending", string? queuedBy = null)
    {
        var item = new DownloadQueueItem
        {
            Id = Guid.NewGuid().ToString(),
            ChapterId = chapterId,
            MangaTitle = mangaTitle,
            ChapterNum = chapterNum,
            Status = status,
            PagesDownloaded = 0,
            PagesTotal = 0,
            CreatedAt = DateTime.UtcNow,
            UpdatedAt = DateTime.UtcNow,
            QueuedBy = queuedBy,
        };
        db.DownloadQueue.Add(item);
        await db.SaveChangesAsync();
        return item;
    }
}
