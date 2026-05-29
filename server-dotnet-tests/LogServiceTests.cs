using ArrghServer.Services;
using Microsoft.Extensions.Logging;
using Xunit;

namespace ArrghServer.Tests;

/// <summary>
/// Unit tests for LogService pure helpers — no HTTP, no DB.
/// </summary>
[Trait("Category", TestCategories.Unit)]
public class LogServiceTests
{
    // ── ParseLevel ────────────────────────────────────────────────────────────

    [Theory]
    [InlineData("error", LogLevel.Error)]
    [InlineData("ERROR", LogLevel.Error)]
    [InlineData("warn",  LogLevel.Warning)]
    [InlineData("WARN",  LogLevel.Warning)]
    [InlineData("info",  LogLevel.Information)]
    [InlineData("INFO",  LogLevel.Information)]
    [InlineData("debug", LogLevel.Debug)]
    [InlineData("DEBUG", LogLevel.Debug)]
    public void ParseLevel_ValidLevel_ReturnsExpected(string input, LogLevel expected) =>
        Assert.Equal(expected, LogService.ParseLevel(input));

    [Theory]
    [InlineData("trace")]
    [InlineData("verbose")]
    [InlineData("")]
    [InlineData("critical")]
    public void ParseLevel_UnknownLevel_ReturnsNull(string input) =>
        Assert.Null(LogService.ParseLevel(input));

    // ── LevelToString ─────────────────────────────────────────────────────────

    [Theory]
    [InlineData(LogLevel.Error,       "ERROR")]
    [InlineData(LogLevel.Warning,     "WARN")]
    [InlineData(LogLevel.Information, "INFO")]
    [InlineData(LogLevel.Debug,       "DEBUG")]
    [InlineData(LogLevel.None,        "INFO")]
    public void LevelToString_ReturnsExpectedString(LogLevel level, string expected) =>
        Assert.Equal(expected, LogService.LevelToString(level));

    // ── SetLevel ──────────────────────────────────────────────────────────────

    [Fact]
    public void SetLevel_ValidLevel_ReturnsTrueAndUpdates()
    {
        var svc = new LogService();
        Assert.True(svc.SetLevel("debug"));
        Assert.Equal("DEBUG", svc.CurrentLevel);
    }

    [Fact]
    public void SetLevel_InvalidLevel_ReturnsFalse()
    {
        var svc = new LogService();
        Assert.False(svc.SetLevel("trace"));
        Assert.Equal("INFO", svc.CurrentLevel); // unchanged
    }

    [Fact]
    public void SetLevel_NormalisesToUppercase()
    {
        var svc = new LogService();
        svc.SetLevel("warn");
        Assert.Equal("WARN", svc.CurrentLevel);
    }

    // ── GetRecent / Append ────────────────────────────────────────────────────

    [Fact]
    public void GetRecent_Empty_ReturnsEmptyList()
    {
        var svc = new LogService();
        Assert.Empty(svc.GetRecent(10));
    }

    [Fact]
    public void GetRecent_ReturnsLastN()
    {
        var svc = new LogService();
        for (var i = 0; i < 10; i++)
            svc.Append(new LogEntry(DateTime.UtcNow, "INFO", "ArrghServer", $"msg {i}"));

        var recent = svc.GetRecent(3);
        Assert.Equal(3, recent.Count);
        Assert.Equal("msg 9", recent[^1].Message);
        Assert.Equal("msg 7", recent[0].Message);
    }

    [Fact]
    public void Append_EvictsOldestWhenCapacityExceeded()
    {
        var svc = new LogService();
        for (var i = 0; i < 501; i++)
            svc.Append(new LogEntry(DateTime.UtcNow, "INFO", "ArrghServer", $"msg {i}"));

        var all = svc.GetRecent(500);
        Assert.Equal(500, all.Count);
        Assert.Equal("msg 500", all[^1].Message); // newest
        Assert.Equal("msg 1",   all[0].Message);  // oldest kept (0 was evicted)
    }
}
