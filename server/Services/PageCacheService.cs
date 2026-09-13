using Microsoft.Extensions.Caching.Memory;

namespace ArrghServer.Services;

public record PageUrlEntry(string Url, string? Referer);

/// <summary>
/// Cache for plugin-host page URL responses. 300-second TTL.
/// </summary>
public class PageCacheService(IMemoryCache cache)
{
    public PageCacheService() : this(new MemoryCache(new MemoryCacheOptions())) { }

    private static readonly TimeSpan Ttl = TimeSpan.FromSeconds(300);

    public List<PageUrlEntry>? Get(string key) =>
        cache.TryGetValue(key, out List<PageUrlEntry>? pages) ? pages : null;

    public void Set(string key, List<PageUrlEntry> pages) =>
        cache.Set(key, pages, Ttl);
}
