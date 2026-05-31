using System.Text.Json;
using ArrghServer.Api;
using ArrghServer.Services;
using Xunit;

namespace ArrghServer.Tests;

[Trait("Category", TestCategories.Unit)]
public class DiscoverLogicTests
{
    // ── TitleMatches / Levenshtein ────────────────────────────────────────────

    [Fact] public void TitleMatches_Exact() =>
        Assert.True(Discover.TitleMatches("solo leveling", "solo leveling"));

    [Fact] public void TitleMatches_BothEmpty() =>
        Assert.True(Discover.TitleMatches("", ""));

    [Fact] public void TitleMatches_SmallTypo() =>
        Assert.True(Discover.TitleMatches(
            Media.NormalizeTitle("Overlord"),
            Media.NormalizeTitle("0verlord")));

    [Fact] public void TitleMatches_NovelSuffix_SiteResult()
    {
        var a = Media.NormalizeTitle("I Shall Seal the Heavens (Novel)");
        var b = Media.NormalizeTitle("I Shall Seal the Heavens");
        Assert.True(Discover.TitleMatches(a, b), "novel-suffix should match bare site result");
    }

    [Fact] public void TitleMatches_Unrelated_ReturnsFalse()
    {
        Assert.False(Discover.TitleMatches(
            Media.NormalizeTitle("Solo Leveling"),
            Media.NormalizeTitle("Tower of God")));
    }

    [Fact] public void TitleMatches_HyphenatedVsCompact_Matches() =>
        Assert.True(Discover.TitleMatches(
            Media.NormalizeTitle("So-Eun"),
            Media.NormalizeTitle("Soeun")));

    [Fact] public void TitleMatches_AniListSynonymCloseEnough_Matches() =>
        Assert.True(Discover.TitleMatches(
            Media.NormalizeTitle("Everything Is Agreed Upon"),
            Media.NormalizeTitle("Everything Is Agreed")));

    [Fact] public void Levenshtein_SameString_IsZero() =>
        Assert.Equal(0, Discover.Levenshtein("abc", "abc"));

    [Fact] public void Levenshtein_EmptyVsNonEmpty_IsLength() =>
        Assert.Equal(3, Discover.Levenshtein("", "abc"));

    [Fact] public void Levenshtein_OneSub() =>
        Assert.Equal(1, Discover.Levenshtein("cat", "bat"));

    // ── StripSearchQualifier ──────────────────────────────────────────────────

    [Fact] public void StripQualifier_Novel() =>
        Assert.Equal("I Shall Seal the Heavens", Discover.StripSearchQualifier("I Shall Seal the Heavens (Novel)"));

    [Fact] public void StripQualifier_Manga() =>
        Assert.Equal("Berserk", Discover.StripSearchQualifier("Berserk (Manga)"));

    [Fact] public void StripQualifier_NoSuffix_ReturnsNull() =>
        Assert.Null(Discover.StripSearchQualifier("Solo Leveling"));

    [Fact] public void StripQualifier_MidParen_ReturnsNull() =>
        Assert.Null(Discover.StripSearchQualifier("The World (Is Mine) Forever"));

    [Fact] public void StripQualifier_OnlyParen_ReturnsNull() =>
        Assert.Null(Discover.StripSearchQualifier("(Novel)"));

    [Fact] public void StripQualifier_LongSuffix_ReturnsNull() =>
        Assert.Null(Discover.StripSearchQualifier("Title (This Is Way Too Long To Strip)"));

    // ── SearchCandidates ──────────────────────────────────────────────────────

    [Fact]
    public void SearchCandidates_StrippedFirst()
    {
        var cands = Discover.SearchCandidates(["I Shall Seal the Heavens (Novel)"]);
        Assert.Equal(["I Shall Seal the Heavens", "I Shall Seal the Heavens (Novel)"], cands);
    }

    [Fact]
    public void SearchCandidates_NoDuplicates()
    {
        var cands = Discover.SearchCandidates(["Solo Leveling", "Solo Leveling"]);
        Assert.Equal(["Solo Leveling"], cands);
    }

    [Fact]
    public void SearchCandidates_AliasAlsoStripped()
    {
        var cands = Discover.SearchCandidates(["A Will Eternal (Novel)", "Yi Nian Yong Heng"]);
        Assert.Equal(["A Will Eternal", "A Will Eternal (Novel)", "Yi Nian Yong Heng"], cands);
    }

    // ── KnownNorms ────────────────────────────────────────────────────────────

    [Fact]
    public void KnownNorms_IncludesStrippedVariant()
    {
        var norms = Discover.KnownNorms(["A Will Eternal (Novel)"]);
        Assert.Contains("a will eternal novel", norms);
        Assert.Contains("a will eternal", norms);
    }

    [Fact]
    public void KnownNorms_NoDuplicates()
    {
        var norms = Discover.KnownNorms(["A Will Eternal (Novel)", "A Will Eternal"]);
        Assert.Equal(1, norms.Count(n => n == "a will eternal"));
    }

    [Fact]
    public void KnownNorms_EnablesExactMatchForSiteResult()
    {
        var norms = Discover.KnownNorms(["A Will Eternal (Novel)"]);
        var siteResult = Media.NormalizeTitle("A Will Eternal");
        Assert.True(norms.Any(kn => Discover.TitleMatches(kn, siteResult)));
    }

    // ── IsHentaiTag ───────────────────────────────────────────────────────────

    [Fact] public void IsHentaiTag_Hentai_True() =>
        Assert.True(Discover.IsHentaiTag("Action,hentai,Romance"));

    [Fact] public void IsHentaiTag_AdultOnly_False() =>
        Assert.False(Discover.IsHentaiTag("adult,Drama"));

    [Fact] public void IsHentaiTag_CaseInsensitive() =>
        Assert.True(Discover.IsHentaiTag("HENTAI,Action"));

    [Fact] public void IsHentaiTag_Null_False() =>
        Assert.False(Discover.IsHentaiTag(null));

    [Fact] public void IsHentaiTag_Empty_False() =>
        Assert.False(Discover.IsHentaiTag(""));

    // ── MangaUpdatesService helpers ───────────────────────────────────────────

    [Theory]
    [InlineData("Manhwa", "manhwa")]
    [InlineData("manhua", "manhua")]
    [InlineData("Novel", "novel")]
    [InlineData("Light Novel", "novel")]
    [InlineData("Web Novel", "novel")]
    [InlineData("Manga", "manga")]
    [InlineData(null, "manga")]
    public void MapContentType(string? input, string expected) =>
        Assert.Equal(expected, MangaUpdatesService.MapContentType(input));

    [Fact]
    public void StripHtml_RemovesTags() =>
        Assert.Equal("Bold text", MangaUpdatesService.StripHtml("<b>Bold</b> text"));

    [Fact]
    public void StripHtml_PlainTextUnchanged() =>
        Assert.Equal("plain text", MangaUpdatesService.StripHtml("plain text"));

    [Fact]
    public void StripHtml_Trims() =>
        Assert.Equal("hi", MangaUpdatesService.StripHtml("  <p>hi</p>  "));

    [Fact]
    public void ParseFlexULong_Number() =>
        Assert.Equal(123UL, MangaUpdatesService.ParseFlexULong(JsonDocument.Parse("123").RootElement));

    [Fact]
    public void ParseFlexULong_String() =>
        Assert.Equal(42UL, MangaUpdatesService.ParseFlexULong(JsonDocument.Parse("\"42\"").RootElement));

    [Fact]
    public void ParseFlexULong_Null_ReturnsNull() =>
        Assert.Null(MangaUpdatesService.ParseFlexULong(JsonDocument.Parse("null").RootElement));

    [Fact]
    public void MapSeries_BasicRecord()
    {
        var json = """
            {
              "series_id": 1,
              "title": "Test Manga",
              "description": "<b>Bold</b> text",
              "image": {"url": {"original": "https://cdn.mu/cover.jpg"}},
              "type": "Manhwa",
              "year": "2021",
              "status": "Ongoing",
              "authors": [{"name": "Artist","type":"Artist"},{"name":"Writer","type":"Author"}],
              "genres": [{"genre":"Action"},{"genre":"Hentai"},{"genre":"Smut"}]
            }
            """;
        var el = JsonDocument.Parse(json).RootElement;
        var s = MangaUpdatesService.MapSeries(el);

        Assert.Equal(1UL, s.SeriesId);
        Assert.Equal("Test Manga", s.Title);
        Assert.Equal("Bold text", s.Description);
        Assert.Equal("https://cdn.mu/cover.jpg", s.CoverUrl);
        Assert.Equal("manhwa", s.ContentType);
        Assert.Equal("ongoing", s.Status);
        Assert.Equal(2021, s.Year);
        Assert.Equal("Writer", s.Author);
        Assert.Contains("hentai", s.Tags);
        Assert.Contains("adult", s.Tags); // Smut → adult
        Assert.Contains("Action", s.Tags);
    }

    [Fact]
    public void MapSeries_StringSeriesId()
    {
        var json = """{"series_id":"99","title":"T","description":null,"image":null,"type":null,"year":null,"status":null,"authors":null,"genres":null}""";
        var s = MangaUpdatesService.MapSeries(JsonDocument.Parse(json).RootElement);
        Assert.Equal(99UL, s.SeriesId);
    }

    // ── NovelUpdatesService.ParseHtml ─────────────────────────────────────────

    const string NuSingleResult = """
        <div class="search_main_box_nu">
          <div class="search_body_nu">
            <div class="search_title"><a href="/series/i-shall-seal-the-heavens/">I Shall Seal the Heavens</a></div>
            <div class="search_img_nu"><img src="https://cdn.novelupdates.com/issh.jpg" alt="cover"></div>
            <div class="series_latest_status">Completed</div>
          </div>
        </div>
        """;

    [Fact]
    public void ParseHtml_SingleResult_ExtractsFields()
    {
        var results = NovelUpdatesService.ParseHtml(NuSingleResult);
        Assert.Single(results);
        Assert.Equal("I Shall Seal the Heavens", results[0].Title);
        Assert.Equal("i-shall-seal-the-heavens", results[0].SourceId);
        Assert.Equal("complete", results[0].Status);
        Assert.Contains("novelupdates.com", results[0].CoverUrl ?? "");
    }

    [Fact]
    public void ParseHtml_EmptyHtml_ReturnsEmpty() =>
        Assert.Empty(NovelUpdatesService.ParseHtml("<html><body></body></html>"));

    [Fact]
    public void ParseHtml_MultipleResults_ParsesAll()
    {
        var html = NuSingleResult + """
            <div class="search_main_box_nu">
              <div class="search_body_nu">
                <div class="search_title"><a href="/series/a-will-eternal/">A Will Eternal</a></div>
                <div class="search_img_nu"><img src="https://cdn.novelupdates.com/awe.jpg"></div>
                <div class="series_latest_status">Completed</div>
              </div>
            </div>
            """;
        var results = NovelUpdatesService.ParseHtml(html);
        Assert.Equal(2, results.Count);
        Assert.Contains(results, r => r.Title == "A Will Eternal");
        Assert.Contains(results, r => r.Title == "I Shall Seal the Heavens");
    }

    [Fact]
    public void ParseHtml_OngoingStatus_MapsCorrectly()
    {
        var html = NuSingleResult.Replace("Completed", "Ongoing");
        var results = NovelUpdatesService.ParseHtml(html);
        Assert.Equal("ongoing", results[0].Status);
    }

    // ── AniListService.MapEntry ───────────────────────────────────────────────

    static JsonElement AniListItem(string json) => JsonDocument.Parse(json).RootElement;

    [Fact]
    public void MapEntry_ReturnsNullForNonMangaFormat()
    {
        var json = """{"id":1,"title":{"romaji":"Test","english":null},"isAdult":false,"format":"ANIME","countryOfOrigin":"JP","status":"FINISHED","description":null,"coverImage":{"large":null},"startDate":{"year":2020},"staff":{"nodes":[]}}""";
        Assert.Null(AniListService.MapEntry(AniListItem(json)));
    }

    [Fact]
    public void MapEntry_IsAdult_True_WhenFlagSet()
    {
        var json = """{"id":2,"title":{"romaji":"So-Eun","english":"So-Eun"},"isAdult":true,"format":"MANHWA","countryOfOrigin":"KR","status":"RELEASING","description":null,"coverImage":{"large":null},"startDate":{"year":2021},"staff":{"nodes":[]}}""";
        var result = AniListService.MapEntry(AniListItem(json));
        Assert.NotNull(result);
        Assert.True(result!.IsAdult);
    }

    [Fact]
    public void MapEntry_IsAdult_False_WhenFlagAbsent()
    {
        var json = """{"id":3,"title":{"romaji":"Berserk","english":"Berserk"},"isAdult":false,"format":"MANGA","countryOfOrigin":"JP","status":"RELEASING","description":null,"coverImage":{"large":null},"startDate":{"year":1989},"staff":{"nodes":[]}}""";
        var result = AniListService.MapEntry(AniListItem(json));
        Assert.NotNull(result);
        Assert.False(result!.IsAdult);
    }

    [Fact]
    public void MapEntry_ContentType_KoreanManga_IsManhwa()
    {
        var json = """{"id":4,"title":{"romaji":"Test","english":"Test"},"isAdult":false,"format":"MANGA","countryOfOrigin":"KR","status":"RELEASING","description":null,"coverImage":{"large":null},"startDate":{"year":2020},"staff":{"nodes":[]}}""";
        var result = AniListService.MapEntry(AniListItem(json));
        Assert.Equal("manhwa", result!.ContentType);
    }

    [Fact]
    public void MapEntry_ContentType_ChineseManga_IsManhua()
    {
        var json = """{"id":5,"title":{"romaji":"Test","english":"Test"},"isAdult":false,"format":"MANGA","countryOfOrigin":"CN","status":"RELEASING","description":null,"coverImage":{"large":null},"startDate":{"year":2020},"staff":{"nodes":[]}}""";
        var result = AniListService.MapEntry(AniListItem(json));
        Assert.Equal("manhua", result!.ContentType);
    }

    [Fact]
    public void MapContentType_MapsManhwa()
    {
        Assert.Equal("manhwa", AniListService.MapContentType("MANHWA", null));
        Assert.Equal("manhwa", AniListService.MapContentType("MANGA", "KR"));
    }

    [Fact]
    public void MapContentType_MapsNovel()
    {
        Assert.Equal("novel", AniListService.MapContentType("NOVEL", null));
        Assert.Equal("novel", AniListService.MapContentType("LIGHT_NOVEL", null));
    }

    [Fact]
    public void MapContentType_UnknownFormat_IsOther() =>
        Assert.Equal("other", AniListService.MapContentType("ANIME", "JP"));

    // ── TrendingCacheService (keyed by lane) ──────────────────────────────────

    [Fact]
    public void TrendingCache_GetFresh_ReturnsNull_WhenEmpty()
    {
        var cache = new TrendingCacheService();
        Assert.Null(cache.GetFresh("manga"));
    }

    [Fact]
    public void TrendingCache_GetFresh_ReturnsCachedResults_ForCorrectLane()
    {
        var cache = new TrendingCacheService();
        var result = new ArrghServer.Api.DiscoverResult
            { MangaupdatesId = "1", Title = "Berserk", Status = "ongoing", ContentType = "manga", Source = "mu" };
        cache.Set("manga", [result]);
        var fresh = cache.GetFresh("manga");
        Assert.NotNull(fresh);
        Assert.Single(fresh);
        Assert.Equal("Berserk", fresh[0].Title);
    }

    [Fact]
    public void TrendingCache_GetFresh_ReturnsNull_ForDifferentLane()
    {
        var cache = new TrendingCacheService();
        var result = new ArrghServer.Api.DiscoverResult
            { MangaupdatesId = "1", Title = "Solo Leveling", Status = "ongoing", ContentType = "manhwa", Source = "anilist" };
        cache.Set("manhwa", [result]);
        Assert.Null(cache.GetFresh("manga"));
    }

    [Fact]
    public void TrendingCache_GetStale_ReturnsData_EvenAfterTtlExpiry()
    {
        var cache = new TrendingCacheService();
        var result = new ArrghServer.Api.DiscoverResult
            { MangaupdatesId = "1", Title = "Test", Status = "ongoing", ContentType = "manga", Source = "mu" };
        cache.Set("manga", [result]);
        cache.ExpireForTest("manga");
        Assert.Null(cache.GetFresh("manga"));
        Assert.NotNull(cache.GetStale("manga"));
    }

    [Fact]
    public void TrendingCache_IndependentLanes_DoNotCrossContaminate()
    {
        var cache = new TrendingCacheService();
        cache.Set("manga", [new ArrghServer.Api.DiscoverResult
            { MangaupdatesId = "1", Title = "Manga", Status = "ongoing", ContentType = "manga", Source = "mu" }]);
        cache.Set("manhwa", [new ArrghServer.Api.DiscoverResult
            { MangaupdatesId = "2", Title = "Manhwa", Status = "ongoing", ContentType = "manhwa", Source = "anilist" }]);
        Assert.Equal("Manga", cache.GetFresh("manga")![0].Title);
        Assert.Equal("Manhwa", cache.GetFresh("manhwa")![0].Title);
    }
}
