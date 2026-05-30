using System.Net;
using System.Net.Http.Json;
using System.Text.Json;
using Xunit;

namespace ArrghServer.Tests;

[Trait("Category", TestCategories.Integration)]
public class SettingsTests
{
    static AppFactory NewFactory() => new();

    // ── GET /api/settings ─────────────────────────────────────────────────────

    [Fact]
    public async Task GetSettings_ReturnsDefaults_WhenNothingSaved()
    {
        var (client, _) = NewFactory().CreateClientWithDb();
        var res = await client.GetFromJsonAsync<JsonElement>("/api/settings");

        Assert.Equal(2, res.GetProperty("download_workers").GetInt64());
        Assert.Equal(6, res.GetProperty("index_interval_hours").GetInt64());
        Assert.False(res.GetProperty("auto_download").GetBoolean());
        Assert.Equal("paged", res.GetProperty("reader_mode").GetString());
        Assert.Equal(5, res.GetProperty("trending_per_source").GetInt64());
        Assert.False(res.GetProperty("check_for_updates").GetBoolean());
    }

    [Fact]
    public async Task GetSettings_NoAuthRequired()
    {
        var (client, _) = NewFactory().CreateClientWithDb();
        var res = await client.GetAsync("/api/settings");
        Assert.Equal(HttpStatusCode.OK, res.StatusCode);
    }

    // ── POST /api/settings ────────────────────────────────────────────────────

    [Fact]
    public async Task SaveSettings_UpdatesAndReturnsNewValues()
    {
        var (client, _) = NewFactory().CreateClientWithDb();

        var res = await client.PostAsJsonAsync("/api/settings", new
        {
            download_workers = 4,
            reader_mode = "scroll",
            auto_download = true,
            check_for_updates = true,
        });

        Assert.Equal(HttpStatusCode.OK, res.StatusCode);
        var body = await res.Content.ReadFromJsonAsync<JsonElement>();
        Assert.Equal(4, body.GetProperty("download_workers").GetInt64());
        Assert.Equal("scroll", body.GetProperty("reader_mode").GetString());
        Assert.True(body.GetProperty("auto_download").GetBoolean());
        Assert.True(body.GetProperty("check_for_updates").GetBoolean());
    }

    [Fact]
    public async Task SaveSettings_PartialUpdate_OnlyChangesSpecifiedFields()
    {
        var (client, _) = NewFactory().CreateClientWithDb();

        await client.PostAsJsonAsync("/api/settings", new { download_workers = 4 });
        var res = await client.GetFromJsonAsync<JsonElement>("/api/settings");

        Assert.Equal(4, res.GetProperty("download_workers").GetInt64());
        Assert.Equal(6, res.GetProperty("index_interval_hours").GetInt64()); // unchanged default
    }

    [Fact]
    public async Task SaveSettings_Idempotent_OverwritesSameKey()
    {
        var (client, _) = NewFactory().CreateClientWithDb();

        await client.PostAsJsonAsync("/api/settings", new { download_workers = 4 });
        await client.PostAsJsonAsync("/api/settings", new { download_workers = 8 });
        var res = await client.GetFromJsonAsync<JsonElement>("/api/settings");

        Assert.Equal(8, res.GetProperty("download_workers").GetInt64());
    }

    [Fact]
    public async Task SaveSettings_UnprocessableEntity_InvalidReaderMode()
    {
        var (client, _) = NewFactory().CreateClientWithDb();
        var res = await client.PostAsJsonAsync("/api/settings", new { reader_mode = "continuous" });
        Assert.Equal(HttpStatusCode.UnprocessableEntity, res.StatusCode);
    }

    [Fact]
    public async Task SaveSettings_ClampsTrendingPerSource()
    {
        var (client, _) = NewFactory().CreateClientWithDb();
        var res = await client.PostAsJsonAsync("/api/settings", new { trending_per_source = 999 });
        var body = await res.Content.ReadFromJsonAsync<JsonElement>();
        Assert.Equal(50, body.GetProperty("trending_per_source").GetInt64());
    }

    [Fact]
    public async Task SaveSettings_IgnoresEmptyDownloadDir()
    {
        var (client, _) = NewFactory().CreateClientWithDb();
        var before = await client.GetFromJsonAsync<JsonElement>("/api/settings");
        var defaultDir = before.GetProperty("download_dir").GetString();

        await client.PostAsJsonAsync("/api/settings", new { download_dir = "   " });
        var after = await client.GetFromJsonAsync<JsonElement>("/api/settings");

        Assert.Equal(defaultDir, after.GetProperty("download_dir").GetString());
    }

    [Fact]
    public async Task SaveSettings_TrimsDownloadDir()
    {
        var (client, _) = NewFactory().CreateClientWithDb();
        var res = await client.PostAsJsonAsync("/api/settings", new { download_dir = "  /data/downloads  " });
        var body = await res.Content.ReadFromJsonAsync<JsonElement>();
        Assert.Equal("/data/downloads", body.GetProperty("download_dir").GetString());
    }

    [Fact]
    public async Task SaveSettings_NoAuthRequired()
    {
        var (client, _) = NewFactory().CreateClientWithDb();
        var res = await client.PostAsJsonAsync("/api/settings", new { download_workers = 3 });
        Assert.Equal(HttpStatusCode.OK, res.StatusCode);
    }
}
