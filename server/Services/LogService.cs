using Microsoft.Extensions.Logging;

namespace ArrghServer.Services;

public record LogEntry(DateTime Timestamp, string Level, string Target, string Message);

/// <summary>
/// In-memory ring buffer for server log entries.
/// Registered as a singleton; populated by RingBufferLoggerProvider.
/// </summary>
public class LogService
{
    private readonly Queue<LogEntry> _buffer = new();
    private const int Capacity = 500;
    private readonly object _lock = new();

    public string CurrentLevel { get; private set; } = "INFO";

    public void Append(LogEntry entry)
    {
        lock (_lock)
        {
            if (_buffer.Count >= Capacity) _buffer.Dequeue();
            _buffer.Enqueue(entry);
        }
    }

    public IReadOnlyList<LogEntry> GetRecent(int limit)
    {
        lock (_lock)
            return _buffer.TakeLast(Math.Clamp(limit, 1, 500)).ToList();
    }

    public bool SetLevel(string level)
    {
        if (ParseLevel(level) is null) return false;
        CurrentLevel = level.ToUpperInvariant();
        return true;
    }

    // ── Pure helpers — unit-testable ─────────────────────────────────────────

    internal static LogLevel? ParseLevel(string level) => level.ToUpperInvariant() switch
    {
        "ERROR" => LogLevel.Error,
        "WARN"  => LogLevel.Warning,
        "INFO"  => LogLevel.Information,
        "DEBUG" => LogLevel.Debug,
        _       => null,
    };

    internal static string LevelToString(LogLevel level) => level switch
    {
        LogLevel.Error   => "ERROR",
        LogLevel.Warning => "WARN",
        LogLevel.Debug   => "DEBUG",
        _                => "INFO",
    };
}

/// <summary>
/// Custom ILoggerProvider that captures log events into the LogService ring buffer.
/// Only captures entries from the "ArrghServer" namespace.
/// </summary>
public class RingBufferLoggerProvider(LogService logService) : ILoggerProvider
{
    public ILogger CreateLogger(string categoryName) =>
        new RingBufferLogger(categoryName, logService);

    public void Dispose() { }
}

file class RingBufferLogger(string category, LogService logService) : ILogger
{
    public IDisposable? BeginScope<TState>(TState state) where TState : notnull => null;

    public bool IsEnabled(LogLevel logLevel)
    {
        var gate = LogService.ParseLevel(logService.CurrentLevel) ?? LogLevel.Information;
        return logLevel >= gate && IsTrackedCategory(category);
    }

    static bool IsTrackedCategory(string cat) =>
        cat.StartsWith("ArrghServer") ||
        cat == "Microsoft.AspNetCore.Hosting.Diagnostics";

    public void Log<TState>(LogLevel logLevel, EventId eventId, TState state,
        Exception? exception, Func<TState, Exception?, string> formatter)
    {
        if (!IsEnabled(logLevel)) return;

        logService.Append(new LogEntry(
            Timestamp: DateTime.UtcNow,
            Level: LogService.LevelToString(logLevel),
            Target: category,
            Message: formatter(state, exception)));
    }
}
