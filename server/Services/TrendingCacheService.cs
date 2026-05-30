namespace ArrghServer.Services;

/// <summary>
/// Singleton 1-hour TTL cache for MangaUpdates trending results.
/// Stale data is retained so it can be served on MU API failure.
/// </summary>
public class TrendingCacheService
{
    private (DateTime FetchedAt, List<MuSeries> Series)? _cached;
    private readonly object _lock = new();
    private static readonly TimeSpan Ttl = TimeSpan.FromHours(1);

    public List<MuSeries>? GetFresh()
    {
        lock (_lock)
        {
            if (_cached is { } c && DateTime.UtcNow - c.FetchedAt < Ttl)
                return c.Series;
            return null;
        }
    }

    public List<MuSeries>? GetStale()
    {
        lock (_lock) { return _cached?.Series; }
    }

    public void Set(List<MuSeries> series)
    {
        lock (_lock) { _cached = (DateTime.UtcNow, series); }
    }
}
