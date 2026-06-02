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
    [InlineData("hentai",  "nhentai")]
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
    public void AuthorityOrder_NovelUpdatesBeforeWuxiaWorld()
    {
        var order = Discover.AuthorityOrder;
        Assert.True(order.IndexOf("novelupdates") < order.IndexOf("wuxiaworld"));
    }

    [Fact]
    public void AuthorityOrder_WuxiaWorldBeforeNhentai()
    {
        var order = Discover.AuthorityOrder;
        Assert.True(order.IndexOf("wuxiaworld") < order.IndexOf("nhentai"));
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

    // ── FilterMuScope ─────────────────────────────────────────────────────────
    // ADR 0031: MangaUpdates is manga-authority only.
    // Non-manga MU results must be filtered BEFORE dedup — dedup alone can't remove them
    // when no designated-authority result exists for the same title (e.g. NU didn't find it).

    [Theory]
    [InlineData("novel")]
    [InlineData("manhwa")]
    [InlineData("manhua")]
    [InlineData("hentai")]
    public void FilterMuScope_Excludes_NonMangaTypes(string contentType)
    {
        var input = new[] { Result("Some Title", contentType, "mangaupdates", "1") };
        Assert.Empty(Discover.FilterMuScope(input));
    }

    [Theory]
    [InlineData("manga")]
    [InlineData("one-shot")]
    public void FilterMuScope_Keeps_MangaTypes(string contentType)
    {
        var input = new[] { Result("Naruto", contentType, "mangaupdates", "1") };
        Assert.Single(Discover.FilterMuScope(input));
    }

    [Fact]
    public void FilterMuScope_EmptyInput_ReturnsEmpty()
    {
        Assert.Empty(Discover.FilterMuScope([]));
    }

    [Fact]
    public void FilterMuScope_MixedTypes_ReturnsOnlyMangaAndOneShot()
    {
        var input = new[]
        {
            Result("Naruto",                        "manga",   "mangaupdates", "1"),
            Result("I Shall Seal the Heavens",      "novel",   "mangaupdates", "2"),
            Result("Solo Leveling",                 "manhwa",  "mangaupdates", "3"),
            Result("Battle Through the Heavens",    "manhua",  "mangaupdates", "4"),
            Result("Test Doujin",                   "hentai",  "mangaupdates", "5"),
            Result("Some One-shot",                 "one-shot","mangaupdates", "6"),
        };

        var filtered = Discover.FilterMuScope(input).ToList();

        Assert.Equal(2, filtered.Count);
        Assert.All(filtered, r => Assert.True(r.ContentType is "manga" or "one-shot"));
        Assert.Contains(filtered, r => r.Title == "Naruto");
        Assert.Contains(filtered, r => r.Title == "Some One-shot");
    }

    // ── NhentaiUpgrade (MergeFanOut pre-dedup pass) ───────────────────────────

    [Fact]
    public void MergeFanOut_NhentaiHit_ExactMatch_UpgradesMangaResultToHentai()
    {
        // MU manga (explicit) + nhentai same title → one result, content_type=hentai, MU source kept
        var muResult = Result("KayaNetori", "manga", "mangaupdates", "mu-1");
        muResult.IsExplicit = true;
        var raw = new List<DiscoverResult>
        {
            muResult,
            Result("KayaNetori", "hentai", "nhentai", "nh-1"),
        };

        var merged = Discover.MergeFanOut(raw);

        Assert.Single(merged);
        Assert.Equal("hentai",       merged[0].ContentType);
        Assert.True(merged[0].IsExplicit);
        Assert.Equal("mangaupdates", merged[0].Source);
    }

    [Fact]
    public void MergeFanOut_NhentaiHit_LongerMuTitle_NotUpgraded()
    {
        // nhentai returns query "KayaNetori"; MU returns full title "KayaNetori Kaya-Nee Series Aizou Ban"
        // These normalize differently → no exact match → no merge (two separate results kept)
        var muResult = Result("KayaNetori Kaya-Nee Series Aizou Ban", "manga", "mangaupdates", "mu-1");
        muResult.IsExplicit = true;
        var raw = new List<DiscoverResult>
        {
            muResult,
            Result("KayaNetori", "hentai", "nhentai", "nh-1"),
        };

        var merged = Discover.MergeFanOut(raw);

        Assert.Equal(2, merged.Count);
        var muRes = merged.Single(r => r.Source == "mangaupdates");
        Assert.Equal("manga", muRes.ContentType); // NOT upgraded — different normalized titles
    }

    [Fact]
    public void MergeFanOut_NhentaiHit_NonExplicit_NotUpgraded()
    {
        // Berserk is NOT explicit — nhentai parody result must NOT upgrade MU manga result
        var raw = new List<DiscoverResult>
        {
            Result("Berserk", "manga",  "mangaupdates", "mu-1"), // is_explicit=false
            Result("Berserk", "hentai", "nhentai",      "nh-1"),
        };

        var merged = Discover.MergeFanOut(raw);

        Assert.Equal(2, merged.Count);
        var muResult = merged.Single(r => r.Source == "mangaupdates");
        Assert.Equal("manga", muResult.ContentType); // NOT upgraded
    }

    [Fact]
    public void MergeFanOut_NhentaiHit_MangaResultPreservesMetadata()
    {
        var muResult = new DiscoverResult
        {
            MangaupdatesId = "mu-1",
            Title          = "KayaNetori",
            Description    = "Some desc",
            Author         = "Test Author",
            ContentType    = "manga",
            Source         = "mangaupdates",
            Status         = "complete",
            IsExplicit     = true,
            InLibrary      = false,
        };
        var nh = Result("KayaNetori", "hentai", "nhentai", "nh-1");

        var merged = Discover.MergeFanOut(new List<DiscoverResult> { muResult, nh });

        Assert.Single(merged);
        Assert.Equal("hentai",       merged[0].ContentType);
        Assert.Equal("Some desc",    merged[0].Description);
        Assert.Equal("Test Author",  merged[0].Author);
        Assert.Equal("mangaupdates", merged[0].Source);
    }

    [Fact]
    public void MergeFanOut_NhentaiHit_NoOverlap_NhentaiKept()
    {
        // nhentai result for a title not in any other source → keep it standalone
        var raw = new List<DiscoverResult>
        {
            Result("Naruto",          "manga",  "mangaupdates", "mu-1"),
            Result("SomeHentaiTitle", "hentai", "nhentai",      "nh-1"),
        };

        var merged = Discover.MergeFanOut(raw);

        Assert.Equal(2, merged.Count);
        Assert.Contains(merged, r => r.Source == "nhentai");
    }

    [Fact]
    public void MergeFanOut_NhentaiHit_NormalizationApplied()
    {
        // NormalizeTitle strips punctuation → "KayaNetori!!" and "KayaNetori" both become "kayanetori"
        var muResult = Result("KayaNetori!!", "manga", "mangaupdates", "mu-1");
        muResult.IsExplicit = true;
        var raw = new List<DiscoverResult>
        {
            muResult,
            Result("KayaNetori", "hentai", "nhentai", "nh-1"),
        };

        var merged = Discover.MergeFanOut(raw);

        Assert.Single(merged);
        Assert.Equal("hentai", merged[0].ContentType);
    }

    [Fact]
    public void MergeFanOut_NhentaiHit_OrderStillByAuthority()
    {
        // After upgrade, result re-ordered by authority (MU first)
        var naruto = Result("Naruto",     "manga",  "mangaupdates", "mu-2");
        var kaya   = Result("KayaNetori", "manga",  "mangaupdates", "mu-1");
        kaya.IsExplicit = true;
        var raw = new List<DiscoverResult>
        {
            naruto,
            Result("KayaNetori", "hentai", "nhentai", "nh-1"),
            kaya,
        };

        var merged = Discover.MergeFanOut(raw);

        Assert.Equal(2, merged.Count);
        Assert.Equal("mangaupdates", merged[0].Source);
        Assert.All(merged, r => Assert.NotEqual("nhentai", r.Source));
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
