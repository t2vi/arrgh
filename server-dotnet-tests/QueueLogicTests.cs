using ArrghServer.Api;
using Xunit;

namespace ArrghServer.Tests;

/// <summary>
/// Unit tests for Queue business logic.
/// No HTTP stack, no DB.
/// </summary>
[Trait("Category", TestCategories.Unit)]
public class QueueLogicTests
{
    // ── IsAllowedExplicit ─────────────────────────────────────────────────────
    // Rule: admin ALWAYS sees explicit regardless of allow_explicit flag.
    // Member sees explicit only when allow_explicit=true.

    [Fact]
    public void IsAllowedExplicit_True_WhenAllowExplicitTrue()
    {
        Assert.True(Queue.IsAllowedExplicit("member", allowExplicit: true));
    }

    [Fact]
    public void IsAllowedExplicit_False_WhenMemberWithoutFlag()
    {
        Assert.False(Queue.IsAllowedExplicit("member", allowExplicit: false));
    }

    [Fact]
    public void IsAllowedExplicit_True_WhenAdmin_EvenIfFlagFalse()
    {
        Assert.True(Queue.IsAllowedExplicit("admin", allowExplicit: false));
    }

    [Fact]
    public void IsAllowedExplicit_True_WhenAdmin_AndFlagTrue()
    {
        Assert.True(Queue.IsAllowedExplicit("admin", allowExplicit: true));
    }

    [Fact]
    public void IsAllowedExplicit_False_WhenNullRole_AndFlagFalse()
    {
        Assert.False(Queue.IsAllowedExplicit(null, allowExplicit: false));
    }

    [Fact]
    public void IsAllowedExplicit_True_WhenNullRole_ButFlagTrue()
    {
        Assert.True(Queue.IsAllowedExplicit(null, allowExplicit: true));
    }
}
