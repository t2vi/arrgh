using ArrghServer.Api;

namespace ArrghServer.Services;

public class TrendingCacheService
{
    private readonly Dictionary<string, (DateTime FetchedAt, List<DiscoverResult> Results)> _lanes = new();
    private readonly object _lock = new();
    private static readonly TimeSpan Ttl = TimeSpan.FromHours(1);

    public List<DiscoverResult>? GetFresh(string lane)
    {
        lock (_lock)
        {
            if (_lanes.TryGetValue(lane, out var c) && DateTime.UtcNow - c.FetchedAt < Ttl)
                return c.Results;
            return null;
        }
    }

    public List<DiscoverResult>? GetStale(string lane)
    {
        lock (_lock)
        {
            return _lanes.TryGetValue(lane, out var c) ? c.Results : null;
        }
    }

    public void Set(string lane, List<DiscoverResult> results)
    {
        lock (_lock) { _lanes[lane] = (DateTime.UtcNow, results); }
    }

    internal void ExpireForTest(string lane)
    {
        lock (_lock)
        {
            if (_lanes.TryGetValue(lane, out var c))
                _lanes[lane] = (DateTime.UtcNow - Ttl - TimeSpan.FromSeconds(1), c.Results);
        }
    }
}
