using ArrghServer.Api;
using Microsoft.Extensions.Caching.Memory;

namespace ArrghServer.Services;

public class TrendingCacheService(IMemoryCache cache)
{
    private static readonly TimeSpan Ttl = TimeSpan.FromHours(1);

    public List<DiscoverResult>? GetFresh(string lane) =>
        cache.TryGetValue(lane, out (DateTime FetchedAt, List<DiscoverResult> Results) c) && DateTime.UtcNow - c.FetchedAt < Ttl
            ? c.Results
            : null;

    public List<DiscoverResult>? GetStale(string lane) =>
        cache.TryGetValue(lane, out (DateTime FetchedAt, List<DiscoverResult> Results) c) ? c.Results : null;

    public void Set(string lane, List<DiscoverResult> results) =>
        cache.Set(lane, (FetchedAt: DateTime.UtcNow, Results: results));

    internal void ExpireForTest(string lane)
    {
        if (cache.TryGetValue(lane, out (DateTime FetchedAt, List<DiscoverResult> Results) c))
            cache.Set(lane, (FetchedAt: DateTime.UtcNow - Ttl - TimeSpan.FromSeconds(1), Results: c.Results));
    }
}
