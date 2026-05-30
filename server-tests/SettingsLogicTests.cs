using ArrghServer.Api;
using Xunit;

namespace ArrghServer.Tests;

/// <summary>
/// Unit tests for Settings pure helpers.
/// No HTTP stack, no DB.
/// </summary>
[Trait("Category", TestCategories.Unit)]
public class SettingsLogicTests
{
    // ── ParseLong ─────────────────────────────────────────────────────────────

    [Fact] public void ParseLong_ValidString_ReturnsValue() =>
        Assert.Equal(4, Settings.ParseLong("4", 2));

    [Fact] public void ParseLong_Null_ReturnsDefault() =>
        Assert.Equal(2, Settings.ParseLong(null, 2));

    [Fact] public void ParseLong_InvalidString_ReturnsDefault() =>
        Assert.Equal(6, Settings.ParseLong("not-a-number", 6));

    // ── ParseBool ─────────────────────────────────────────────────────────────

    [Fact] public void ParseBool_TrueString_ReturnsTrue() =>
        Assert.True(Settings.ParseBool("true", false));

    [Fact] public void ParseBool_FalseString_ReturnsFalse() =>
        Assert.False(Settings.ParseBool("false", true));

    [Fact] public void ParseBool_Null_ReturnsDefault() =>
        Assert.True(Settings.ParseBool(null, true));

    [Fact] public void ParseBool_OtherString_ReturnsFalse() =>
        Assert.False(Settings.ParseBool("yes", false));

    // ── ClampTrending ─────────────────────────────────────────────────────────

    [Fact] public void ClampTrending_Below1_ClampsTo1() =>
        Assert.Equal(1, Settings.ClampTrending(0));

    [Fact] public void ClampTrending_Above50_ClampsTo50() =>
        Assert.Equal(50, Settings.ClampTrending(999));

    [Fact] public void ClampTrending_Within_PassesThrough() =>
        Assert.Equal(25, Settings.ClampTrending(25));

    [Fact] public void ClampTrending_Boundary1_Valid() =>
        Assert.Equal(1, Settings.ClampTrending(1));

    [Fact] public void ClampTrending_Boundary50_Valid() =>
        Assert.Equal(50, Settings.ClampTrending(50));

    // ── ValidReaderMode ───────────────────────────────────────────────────────

    [Fact] public void ValidReaderMode_Paged_Valid() =>
        Assert.True(Settings.ValidReaderMode("paged"));

    [Fact] public void ValidReaderMode_Scroll_Valid() =>
        Assert.True(Settings.ValidReaderMode("scroll"));

    [Fact] public void ValidReaderMode_Other_Invalid() =>
        Assert.False(Settings.ValidReaderMode("continuous"));

    [Fact] public void ValidReaderMode_Empty_Invalid() =>
        Assert.False(Settings.ValidReaderMode(""));
}
