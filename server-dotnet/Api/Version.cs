using System.Reflection;

namespace ArrghServer.Api;

public static class Version
{
    // Read from assembly version — set via <Version> in .csproj
    public static readonly string Current =
        Assembly.GetExecutingAssembly()
            .GetCustomAttribute<AssemblyInformationalVersionAttribute>()
            ?.InformationalVersion
            ?? Assembly.GetExecutingAssembly().GetName().Version?.ToString(3)
            ?? "0.0.0";

    public static RouteGroupBuilder MapVersionRoutes(this RouteGroupBuilder group)
    {
        group.MapGet("/", GetVersion).WithSummary("Get server version and update availability");
        return group;
    }

    static IResult GetVersion(UpdateCache cache)
    {
        var (latest, releaseUrl) = cache.GetIfNewer(Current);
        return Results.Ok(new
        {
            current = Current,
            latest,
            release_url = releaseUrl,
        });
    }
}

/// <summary>
/// Singleton holding the latest GitHub release info.
/// Populated by background update checker when check_for_updates=true.
/// </summary>
public class UpdateCache
{
    private record CachedRelease(string Version, string HtmlUrl);
    private CachedRelease? _cached;
    private readonly object _lock = new();

    public void Set(string version, string htmlUrl)
    {
        lock (_lock) _cached = new(version, htmlUrl);
    }

    public void Clear()
    {
        lock (_lock) _cached = null;
    }

    /// Returns (latest, releaseUrl) when a newer version is cached, (null, null) otherwise.
    public (string? Latest, string? ReleaseUrl) GetIfNewer(string current)
    {
        lock (_lock)
        {
            if (_cached is not null && _cached.Version != current)
                return (_cached.Version, _cached.HtmlUrl);
            return (null, null);
        }
    }
}
