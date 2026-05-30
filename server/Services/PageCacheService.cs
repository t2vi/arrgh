namespace ArrghServer.Services;

public record PageUrlEntry(string Url, string? Referer);

/// <summary>
/// In-memory LRU-ish cache for plugin-host page URL responses.
/// 300-second TTL, max 200 entries (stale eviction on overflow).
/// </summary>
public class PageCacheService
{
    private readonly Dictionary<string, (DateTime FetchedAt, List<PageUrlEntry> Pages)> _cache = new();
    private readonly object _lock = new();
    private static readonly TimeSpan Ttl = TimeSpan.FromSeconds(300);

    public List<PageUrlEntry>? Get(string key)
    {
        lock (_lock)
        {
            if (_cache.TryGetValue(key, out var entry) && DateTime.UtcNow - entry.FetchedAt < Ttl)
                return entry.Pages;
            return null;
        }
    }

    public void Set(string key, List<PageUrlEntry> pages)
    {
        lock (_lock)
        {
            if (_cache.Count > 200)
            {
                var stale = _cache
                    .Where(kvp => DateTime.UtcNow - kvp.Value.FetchedAt >= Ttl)
                    .Select(kvp => kvp.Key).ToList();
                foreach (var k in stale) _cache.Remove(k);
            }
            _cache[key] = (DateTime.UtcNow, pages);
        }
    }
}
