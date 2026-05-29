using ArrghServer.Api;
using ArrghServer.Services;
using Xunit;

namespace ArrghServer.Tests;

// TDD: Unit tests for fan-out orchestration helpers (ADR 0031).
// Tests will fail until implementation is complete.
// Requires:
//   - Discover.DesignatedAuthority(contentType) static method
//   - Discover.Deduplicate(IEnumerable<DiscoverResult>) static method
//   - Authority ordering constant: MU → AniList → MangaDex → NovelUpdates → EHentai

[Trait("Category", TestCategories.Unit)]
public class DiscoverFanOutLogicTests
{
    // ── DesignatedAuthority ───────────────────────────────────────────────────

    [Theory]
    [InlineData("manga",   "mangaupdates")]
    [InlineData("manhwa",  "anilist")]
    [InlineData("manhua",  "mangadex")]
    [InlineData("novel",   "novelupdates")]
    [InlineData("hentai",  "ehentai")]
    [InlineData("unknown", "mangaupdates")] // fallback to MU
    public void DesignatedAuthority_ReturnsCorrectSource(string contentType, string expected) =>
        Assert.Equal(expected, Discover.DesignatedAuthority(contentType));

    // ── Deduplicate ───────────────────────────────────────────────────────────

    [Fact]
    public void Deduplicate_NoConflict_ReturnsAll()
    {
        var results = new List<DiscoverResult>
        {
            Result("Naruto",       "manga",  "mangaupdates",  "1"),
            Result("Solo Leveling","manhwa", "anilist",       "101517"),
        };

        var deduped = Discover.Deduplicate(results);

        Assert.Equal(2, deduped.Count);
    }

    [Fact]
    public void Deduplicate_AuthorityWins_ForManhwa()
    {
        // MU returns "Solo Leveling" as manhwa, AniList also returns it
        // AniList is designated authority for manhwa → AniList result must survive
        var results = new List<DiscoverResult>
        {
            Result("Solo Leveling", "manhwa", "mangaupdates", "1"),
            Result("Solo Leveling", "manhwa", "anilist",      "101517"),
        };

        var deduped = Discover.Deduplicate(results);

        Assert.Single(deduped);
        Assert.Equal("anilist", deduped[0].Source);
        Assert.Equal("101517", deduped[0].MangaupdatesId); // source_id in MangaupdatesId field (or SourceId)
    }

    [Fact]
    public void Deduplicate_AuthorityWins_ForManhua()
    {
        // AniList and MangaDex both return the same manhua
        // MangaDex is designated authority for manhua
        var results = new List<DiscoverResult>
        {
            Result("Battle Through the Heavens", "manhua", "anilist",   "al-999"),
            Result("Battle Through the Heavens", "manhua", "mangadex",  "md-abc"),
        };

        var deduped = Discover.Deduplicate(results);

        Assert.Single(deduped);
        Assert.Equal("mangadex", deduped[0].Source);
    }

    [Fact]
    public void Deduplicate_SameTitle_DifferentContentType_NotDeduped()
    {
        // "I Shall Seal the Heavens" exists as both novel AND manhwa/manhua
        // These are different works — must NOT be deduplicated
        var results = new List<DiscoverResult>
        {
            Result("I Shall Seal the Heavens", "novel",  "novelupdates", "nu-1"),
            Result("I Shall Seal the Heavens", "manhwa", "anilist",      "al-1"),
        };

        var deduped = Discover.Deduplicate(results);

        Assert.Equal(2, deduped.Count);
    }

    [Fact]
    public void Deduplicate_NormalizedTitleComparison()
    {
        // "Solo Leveling" and "Solo  Leveling" (extra space) normalize to same key
        var results = new List<DiscoverResult>
        {
            Result("Solo Leveling",  "manhwa", "mangaupdates", "mu-1"),
            Result("Solo  Leveling", "manhwa", "anilist",      "al-1"), // extra space
        };

        var deduped = Discover.Deduplicate(results);

        Assert.Single(deduped);
        Assert.Equal("anilist", deduped[0].Source);
    }

    // ── Authority ordering ────────────────────────────────────────────────────

    [Fact]
    public void AuthorityOrder_MuBeforeAniList()
    {
        var order = Discover.AuthorityOrder;
        Assert.True(order.IndexOf("mangaupdates") < order.IndexOf("anilist"));
    }

    [Fact]
    public void AuthorityOrder_AniListBeforeMangaDex()
    {
        var order = Discover.AuthorityOrder;
        Assert.True(order.IndexOf("anilist") < order.IndexOf("mangadex"));
    }

    [Fact]
    public void AuthorityOrder_MangaDexBeforeNovelUpdates()
    {
        var order = Discover.AuthorityOrder;
        Assert.True(order.IndexOf("mangadex") < order.IndexOf("novelupdates"));
    }

    [Fact]
    public void AuthorityOrder_NovelUpdatesBeforeEHentai()
    {
        var order = Discover.AuthorityOrder;
        Assert.True(order.IndexOf("novelupdates") < order.IndexOf("ehentai"));
    }

    // ── MergeFanOut ───────────────────────────────────────────────────────────

    [Fact]
    public void MergeFanOut_OrderedByAuthority()
    {
        // AniList result before MU result in raw input — merged output must reorder
        var raw = new List<DiscoverResult>
        {
            Result("Solo Leveling", "manhwa", "anilist",      "101517"),
            Result("Naruto",        "manga",  "mangaupdates", "1"),
        };

        var merged = Discover.MergeFanOut(raw);

        Assert.Equal("mangaupdates", merged[0].Source); // MU first
        Assert.Equal("anilist",      merged[1].Source); // AniList second
    }

    [Fact]
    public void MergeFanOut_DeduplicatesBeforeSorting()
    {
        var raw = new List<DiscoverResult>
        {
            Result("Solo Leveling", "manhwa", "mangaupdates", "mu-1"),
            Result("Solo Leveling", "manhwa", "anilist",      "al-1"),
        };

        var merged = Discover.MergeFanOut(raw);

        Assert.Single(merged);
        Assert.Equal("anilist", merged[0].Source);
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    static DiscoverResult Result(string title, string contentType, string source, string sourceId) =>
        new()
        {
            MangaupdatesId = sourceId,
            Title          = title,
            ContentType    = contentType,
            Source         = source,
            Status         = "ongoing",
            InLibrary      = false,
        };
}
