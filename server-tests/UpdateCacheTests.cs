using ArrghServer.Api;
using Xunit;

namespace ArrghServer.Tests;

[Trait("Category", TestCategories.Unit)]
public class UpdateCacheTests
{
    [Fact]
    public void GetIfNewer_Empty_ReturnsNulls()
    {
        var cache = new UpdateCache();
        var (latest, url) = cache.GetIfNewer("1.0.0");
        Assert.Null(latest);
        Assert.Null(url);
    }

    [Fact]
    public void GetIfNewer_SameVersion_ReturnsNulls()
    {
        // Prevents "update available" banner when already on latest
        var cache = new UpdateCache();
        cache.Set("1.2.3", "https://github.com/t2vi/arrgh/releases/tag/v1.2.3");

        var (latest, url) = cache.GetIfNewer("1.2.3");
        Assert.Null(latest);
        Assert.Null(url);
    }

    [Fact]
    public void GetIfNewer_NewerVersion_ReturnsVersionAndUrl()
    {
        var cache = new UpdateCache();
        cache.Set("2.0.0", "https://github.com/t2vi/arrgh/releases/tag/v2.0.0");

        var (latest, url) = cache.GetIfNewer("1.0.0");
        Assert.Equal("2.0.0", latest);
        Assert.Equal("https://github.com/t2vi/arrgh/releases/tag/v2.0.0", url);
    }

    [Fact]
    public void Clear_AfterSet_ReturnsNulls()
    {
        var cache = new UpdateCache();
        cache.Set("2.0.0", "https://example.com");
        cache.Clear();

        var (latest, _) = cache.GetIfNewer("1.0.0");
        Assert.Null(latest);
    }
}
