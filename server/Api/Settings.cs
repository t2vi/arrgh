using System.Text.Json.Serialization;
using ArrghServer.Data;
using ArrghServer.Data.Models;
using Microsoft.EntityFrameworkCore;

namespace ArrghServer.Api;

public static class Settings
{
    public static RouteGroupBuilder MapSettingsRoutes(this RouteGroupBuilder group)
    {
        group.MapGet("/", GetSettings).WithSummary("Get all server settings");
        group.MapPost("/", SaveSettings).WithSummary("Save server settings");
        return group;
    }

    // -------------------------------------------------------------------------

    static async Task<IResult> GetSettings(AppDbContext db, IConfiguration config)
    {
        var settings = await ReadSettings(db, config);
        return Results.Ok(settings);
    }

    static async Task<IResult> SaveSettings(SaveSettingsBody body, AppDbContext db, IConfiguration config)
    {
        if (body.ReaderMode is not null && !ValidReaderMode(body.ReaderMode))
            return Results.UnprocessableEntity();

        // Load tracked entities once — mutate in-memory, single SaveChangesAsync
        var tracked = await db.ServerSettings.ToListAsync();
        var byKey = tracked.ToDictionary(s => s.Key);

        void Stage(string key, string value)
        {
            if (byKey.TryGetValue(key, out var existing))
                existing.Value = value;
            else
                db.ServerSettings.Add(new ServerSetting { Key = key, Value = value });
        }

        if (body.DownloadWorkers is not null)
            Stage("download_workers", body.DownloadWorkers.Value.ToString());

        if (body.IndexIntervalHours is not null)
            Stage("index_interval_hours", body.IndexIntervalHours.Value.ToString());

        if (body.AutoDownload is not null)
            Stage("auto_download", body.AutoDownload.Value ? "true" : "false");

        if (body.ReaderMode is not null)
            Stage("reader_mode", body.ReaderMode);

        if (body.DownloadDir is not null)
        {
            var trimmed = body.DownloadDir.Trim();
            if (!string.IsNullOrEmpty(trimmed))
                Stage("download_dir", trimmed);
        }

        if (body.TrendingPerSource is not null)
            Stage("trending_per_source", ClampTrending(body.TrendingPerSource.Value).ToString());

        if (body.CheckForUpdates is not null)
            Stage("check_for_updates", body.CheckForUpdates.Value ? "true" : "false");

        await db.SaveChangesAsync();

        return Results.Ok(await ReadSettings(db, config));
    }

    // -------------------------------------------------------------------------

    internal static async Task<AppSettingsDto> ReadSettings(AppDbContext db, IConfiguration config)
    {
        var kv = await db.ServerSettings.ToDictionaryAsync(s => s.Key, s => s.Value);
        string? Get(string key) => kv.TryGetValue(key, out var v) ? v : null;

        return new AppSettingsDto
        {
            DownloadWorkers     = ParseLong(Get("download_workers"),   defaultVal: 2),
            IndexIntervalHours  = ParseLong(Get("index_interval_hours"), defaultVal: 6),
            AutoDownload        = ParseBool(Get("auto_download"),      defaultVal: false),
            ReaderMode          = Get("reader_mode") ?? "paged",
            DownloadDir         = Get("download_dir") ?? config["DownloadDir"] ?? "./downloads",
            TrendingPerSource   = ParseLong(Get("trending_per_source"), defaultVal: 5),
            CheckForUpdates     = ParseBool(Get("check_for_updates"),  defaultVal: false),
        };
    }

    // Pure helpers — extracted for unit testability
    internal static long ParseLong(string? value, long defaultVal) =>
        long.TryParse(value, out var v) ? v : defaultVal;

    internal static bool ParseBool(string? value, bool defaultVal) =>
        value is null ? defaultVal : value == "true";

    internal static long ClampTrending(long value) =>
        Math.Clamp(value, 1, 50);

    internal static bool ValidReaderMode(string value) =>
        value is "paged" or "scroll";
}

// ── DTOs ──────────────────────────────────────────────────────────────────────

public class AppSettingsDto
{
    public long DownloadWorkers { get; set; }
    public long IndexIntervalHours { get; set; }
    public bool AutoDownload { get; set; }
    public string ReaderMode { get; set; } = null!;
    public string DownloadDir { get; set; } = null!;
    public long TrendingPerSource { get; set; }
    public bool CheckForUpdates { get; set; }
}

public class SaveSettingsBody
{
    [JsonPropertyName("download_workers")]      public long?   DownloadWorkers { get; set; }
    [JsonPropertyName("index_interval_hours")]  public long?   IndexIntervalHours { get; set; }
    [JsonPropertyName("auto_download")]         public bool?   AutoDownload { get; set; }
    [JsonPropertyName("reader_mode")]           public string? ReaderMode { get; set; }
    [JsonPropertyName("download_dir")]          public string? DownloadDir { get; set; }
    [JsonPropertyName("trending_per_source")]   public long?   TrendingPerSource { get; set; }
    [JsonPropertyName("check_for_updates")]     public bool?   CheckForUpdates { get; set; }
}
