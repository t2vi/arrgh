using System.Net;
using System.Net.Http.Headers;
using System.Net.Http.Json;
using System.Text.Json;
using ArrghServer.Api;
using ArrghServer.Services;
using Microsoft.AspNetCore.Hosting;
using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Microsoft.Extensions.Hosting;
using Microsoft.EntityFrameworkCore;
using Xunit;

namespace ArrghServer.Tests;

// TDD: These tests define the fan-out contract (ADR 0031).
// All tests in this file FAIL until the fan-out implementation is complete.
// Implementation checklist:
//   - Title.MetadataSource + Title.MetadataSourceId columns (EF migration)
//   - AddMangaBody.Source + AddMangaBody.SourceId properties
//   - Discover.DesignatedAuthority(contentType) static helper
//   - Discover.Deduplicate(results) static helper
//   - AniListService (manhwa/manhua from AniList GraphQL)
//   - MangaDexMetaService (manhua from MangaDex REST)
//   - NovelUpdatesService (novel from NovelUpdates HTML)
//   - EHentaiService (.NET port of Rust ehentai.rs)
//   - Search handler: fan-out, dedup, ordered merge, E-Hentai gate
//   - CheckInLibraryAsync: check by normalized (title, content_type), not only mangaupdates_id

[Trait("Category", TestCategories.Integration)]
public class DiscoverFanOutTests
{
    // ── Setup ─────────────────────────────────────────────────────────────────

    const string EmptyWuxiaWorldJson = """{"items":[]}""";

    static FanOutDiscoverFactory NewFactory(
        string muSearchJson                   = """{"results":[]}""",
        string anilistJson                    = EmptyAniListJson,
        string mangadexJson                   = EmptyMangaDexJson,
        string novelupdatesHtml               = EmptyNovelUpdatesHtml,
        string wuxiaworldJson                 = EmptyWuxiaWorldJson,
        string ehentaiSearchHtml              = EmptyEhHtml,
        string ehentaiGdataJson               = EmptyEhGdataJson,
        bool muSearchThrows                   = false,
        bool anilistThrows                    = false,
        bool mangadexThrows                   = false,
        bool novelupdatesThrows               = false,
        bool wuxiaworldThrows                 = false,
        string? anilistSynonymsJson           = null,
        Dictionary<string, string>? pluginSearchTitleMap = null) =>
        new(muSearchJson, anilistJson, mangadexJson, novelupdatesHtml, wuxiaworldJson,
            ehentaiSearchHtml, ehentaiGdataJson,
            muSearchThrows, anilistThrows, mangadexThrows, novelupdatesThrows, wuxiaworldThrows,
            anilistSynonymsJson, pluginSearchTitleMap);

    void Authorize(HttpClient client, ArrghServer.Data.Models.User user, bool allowExplicit = false)
    {
        var config = new ConfigurationBuilder()
            .AddInMemoryCollection(new Dictionary<string, string?> { ["JwtSecret"] = AppFactory.JwtSecret })
            .Build();
        var token = Auth.CreateToken(user.Id, user.Username, user.Role, allowExplicit, config);
        client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue("Bearer", token);
    }

    // ── MU fixture helpers ────────────────────────────────────────────────────

    static string MuSearchResponse(params object[] records)
    {
        var hits = records.Select(r => new { record = r });
        return JsonSerializer.Serialize(new { results = hits });
    }

    static object MuRecord(ulong id, string title, string type = "Manga") => new
    {
        series_id = id, title, description = (string?)null, image = (object?)null,
        type, year = (string?)null, status = "ongoing",
        authors = (object?)null, genres = (object?)null, associated = (object?)null,
    };

    // ── AniList fixture helpers ───────────────────────────────────────────────

    const string EmptyAniListJson = """{"data":{"Page":{"media":[]}}}""";

    static string AniListResponse(params AniListMedia[] items) =>
        JsonSerializer.Serialize(new
        {
            data = new { Page = new { media = items } }
        });

    record AniListMedia(
        int id,
        AniListTitle title,
        string format,
        string countryOfOrigin,
        string status = "FINISHED",
        string? description = null,
        AniListCover? coverImage = null,
        AniListStartDate? startDate = null,
        AniListStaff? staff = null,
        string[]? synonyms = null);

    record AniListTitle(string romaji, string? english = null);
    record AniListCover(string? large = null);
    record AniListStartDate(int? year = null);
    record AniListStaff(AniListNode[]? nodes = null);
    record AniListNode(AniListName name);
    record AniListName(string full);

    // ── MangaDex fixture helpers ──────────────────────────────────────────────

    const string EmptyMangaDexJson = """{"data":[]}""";

    static string MangaDexResponse(params MangaDexEntry[] entries) =>
        JsonSerializer.Serialize(new { data = entries });

    record MangaDexEntry(
        string id,
        MangaDexAttributes attributes,
        MangaDexRelationship[]? relationships = null);

    record MangaDexAttributes(
        Dictionary<string, string> title,
        string originalLanguage = "zh",
        string status = "completed",
        int? year = null,
        Dictionary<string, string>? description = null);

    record MangaDexRelationship(string type, MangaDexRelAttr? attributes = null);
    record MangaDexRelAttr(string? name = null);

    // ── WuxiaWorld fixture helpers ────────────────────────────────────────────

    static string WuxiaWorldResponse(params (string slug, string name, string status)[] items) =>
        System.Text.Json.JsonSerializer.Serialize(new
        {
            items = items.Select(i => new
            {
                slug = i.slug,
                name = i.name,
                status = (object)(i.status == "completed" ? 0 : 1),
                tags = i.status == "completed" ? new[] { "Completed" } : new[] { "Ongoing" },
                coverUrl = (string?)null,
            })
        });

    // ── NovelUpdates fixture helpers ──────────────────────────────────────────

    const string EmptyNovelUpdatesHtml = """<html><body><div class="w-blog-content"></div></body></html>""";

    static string NovelUpdatesHtml(string seriesSlug, string title, string status = "Completed") =>
        $"""
        <html><body>
        <div class="search_main_box_nu">
          <div class="search_body_nu">
            <div class="search_title"><a href="/series/{seriesSlug}/">{title}</a></div>
            <div class="search_img_nu"><img src="https://cdn.novelupdates.com/{seriesSlug}.jpg"></div>
            <div class="seriestypelist">Web Novel</div>
            <div class="series_latest_status">{status}</div>
          </div>
        </div>
        </body></html>
        """;

    // ── E-Hentai fixture helpers ──────────────────────────────────────────────

    const string EmptyEhHtml = """<html><body><div class="itg gltm"></div></body></html>""";
    const string EmptyEhGdataJson = """{"gmetadata":[]}""";

    static string EhSearchHtml(ulong gid, string token) =>
        $"""
        <html><body>
        <div class="itg gltm">
          <td class="gl3c gltm"><a href="https://e-hentai.org/g/{gid}/{token}/"></a></td>
        </div>
        </body></html>
        """;

    static string EhGdataResponse(ulong gid, string title, string parodyTag) =>
        JsonSerializer.Serialize(new
        {
            gmetadata = new[]
            {
                new { gid, title, thumb = (string?)null, tags = new[] { $"parody:{parodyTag}", "hentai", "english" } }
            }
        });

    // ── GET /api/discover — fan-out ───────────────────────────────────────────

    [Fact]
    public async Task Search_MuMangaResult_HasSourceMangaupdates()
    {
        var searchJson = MuSearchResponse(MuRecord(1, "Naruto", "Manga"));
        var (client, db) = NewFactory(muSearchJson: searchJson).CreateClientWithDb();
        var user = await Seed.UserAsync(db, Fake.AdminUser());
        Authorize(client, user);

        var res = await client.GetFromJsonAsync<JsonElement[]>("/api/discover?q=naruto");

        Assert.NotNull(res);
        var naruto = res.FirstOrDefault(r => r.GetProperty("title").GetString() == "Naruto");
        Assert.NotNull(naruto);
        Assert.Equal("mangaupdates", naruto.GetProperty("source").GetString());
        Assert.Equal("manga", naruto.GetProperty("content_type").GetString());
    }

    [Fact]
    public async Task Search_AniListManhwa_HasSourceAnilist()
    {
        var anilistJson = AniListResponse(new AniListMedia(
            id: 101517,
            title: new AniListTitle("Solo Leveling"),
            format: "MANHWA",
            countryOfOrigin: "KR",
            status: "FINISHED"));

        var (client, db) = NewFactory(anilistJson: anilistJson).CreateClientWithDb();
        var user = await Seed.UserAsync(db, Fake.AdminUser());
        Authorize(client, user);

        var res = await client.GetFromJsonAsync<JsonElement[]>("/api/discover?q=solo+leveling");

        Assert.NotNull(res);
        var solo = res.FirstOrDefault(r => r.GetProperty("title").GetString() == "Solo Leveling");
        Assert.NotNull(solo);
        Assert.Equal("anilist", solo.GetProperty("source").GetString());
        Assert.Equal("manhwa", solo.GetProperty("content_type").GetString());
    }

    [Fact]
    public async Task Search_MangaDexManhua_HasSourceMangadex()
    {
        var mdJson = MangaDexResponse(new MangaDexEntry(
            id: "a1c7c817-0000-0000-0000-000000000000",
            attributes: new MangaDexAttributes(
                title: new Dictionary<string, string> { ["en"] = "Battle Through the Heavens" },
                originalLanguage: "zh",
                status: "completed",
                year: 2013)));

        var (client, db) = NewFactory(mangadexJson: mdJson).CreateClientWithDb();
        var user = await Seed.UserAsync(db, Fake.AdminUser());
        Authorize(client, user);

        var res = await client.GetFromJsonAsync<JsonElement[]>("/api/discover?q=battle+through");

        Assert.NotNull(res);
        var btth = res.FirstOrDefault(r => r.GetProperty("title").GetString() == "Battle Through the Heavens");
        Assert.NotNull(btth);
        Assert.Equal("mangadex", btth.GetProperty("source").GetString());
        Assert.Equal("manhua", btth.GetProperty("content_type").GetString());
    }

    [Fact]
    public async Task Search_NovelUpdatesNovel_HasSourceNovelupdates()
    {
        var nuHtml = NovelUpdatesHtml("a-will-eternal", "A Will Eternal");

        var (client, db) = NewFactory(novelupdatesHtml: nuHtml).CreateClientWithDb();
        var user = await Seed.UserAsync(db, Fake.AdminUser());
        Authorize(client, user);

        var res = await client.GetFromJsonAsync<JsonElement[]>("/api/discover?q=a+will+eternal");

        Assert.NotNull(res);
        var awe = res.FirstOrDefault(r => r.GetProperty("title").GetString() == "A Will Eternal");
        Assert.NotNull(awe);
        Assert.Equal("novelupdates", awe.GetProperty("source").GetString());
        Assert.Equal("novel", awe.GetProperty("content_type").GetString());
    }

    [Fact]
    public async Task Search_EHentai_NotIncluded_ForNonExplicitUser()
    {
        // E-Hentai would return results, but user doesn't have allow_explicit
        var ehHtml = EhSearchHtml(12345, "abc123");
        var ehGdata = EhGdataResponse(12345, "Test Doujin", "naruto");

        var (client, db) = NewFactory(ehentaiSearchHtml: ehHtml, ehentaiGdataJson: ehGdata).CreateClientWithDb();
        var user = await Seed.UserAsync(db, Fake.AdminUser());
        Authorize(client, user, allowExplicit: false); // explicit = false

        var res = await client.GetFromJsonAsync<JsonElement[]>("/api/discover?q=naruto");

        Assert.NotNull(res);
        Assert.DoesNotContain(res, r => r.GetProperty("source").GetString() == "ehentai");
    }

    [Fact]
    public async Task Search_EHentai_Included_ForExplicitUser()
    {
        var ehHtml = EhSearchHtml(12345, "abc123");
        var ehGdata = EhGdataResponse(12345, "Naruto Doujin", "naruto");

        var (client, db) = NewFactory(ehentaiSearchHtml: ehHtml, ehentaiGdataJson: ehGdata).CreateClientWithDb();
        var user = await Seed.UserAsync(db, Fake.AdminUser());
        Authorize(client, user, allowExplicit: true);

        var res = await client.GetFromJsonAsync<JsonElement[]>("/api/discover?q=naruto");

        Assert.NotNull(res);
        var ehResult = res.FirstOrDefault(r => r.GetProperty("source").GetString() == "ehentai");
        Assert.NotNull(ehResult);
    }

    [Fact]
    public async Task Search_Dedup_AniListWins_ForManhwa()
    {
        // MU returns "Solo Leveling" as manhwa, AniList also returns "Solo Leveling" as manhwa
        // AniList is designated authority for manhwa → AniList result should be kept, MU deduplicated
        var muJson = MuSearchResponse(MuRecord(1, "Solo Leveling", "Manhwa"));
        var alJson = AniListResponse(new AniListMedia(
            id: 101517,
            title: new AniListTitle("Solo Leveling"),
            format: "MANHWA",
            countryOfOrigin: "KR"));

        var (client, db) = NewFactory(muSearchJson: muJson, anilistJson: alJson).CreateClientWithDb();
        var user = await Seed.UserAsync(db, Fake.AdminUser());
        Authorize(client, user);

        var res = await client.GetFromJsonAsync<JsonElement[]>("/api/discover?q=solo+leveling");

        Assert.NotNull(res);
        var soloResults = res.Where(r => r.GetProperty("title").GetString() == "Solo Leveling").ToArray();
        Assert.Single(soloResults); // deduplicated to exactly 1
        Assert.Equal("anilist", soloResults[0].GetProperty("source").GetString()); // AniList wins
    }

    [Fact]
    public async Task Search_ResultOrder_MuBeforeAniList()
    {
        // MU returns manga, AniList returns manhwa — manga (MU) should appear first
        var muJson = MuSearchResponse(MuRecord(1, "Naruto", "Manga"));
        var alJson = AniListResponse(new AniListMedia(
            id: 101517,
            title: new AniListTitle("Solo Leveling"),
            format: "MANHWA",
            countryOfOrigin: "KR"));

        var (client, db) = NewFactory(muSearchJson: muJson, anilistJson: alJson).CreateClientWithDb();
        var user = await Seed.UserAsync(db, Fake.AdminUser());
        Authorize(client, user);

        var res = await client.GetFromJsonAsync<JsonElement[]>("/api/discover?q=test");

        Assert.NotNull(res);
        Assert.True(res.Length >= 2);
        // First result must be the MU manga result
        Assert.Equal("mangaupdates", res[0].GetProperty("source").GetString());
        // AniList result comes after
        var alIndex = Array.FindIndex(res, r => r.GetProperty("source").GetString() == "anilist");
        Assert.True(alIndex > 0, "AniList result must come after MU result");
    }

    [Fact]
    public async Task Search_PartialFailure_ReturnsOtherResults()
    {
        // AniList fails but MU returns results — should get 200 with MU results, not 502
        var muJson = MuSearchResponse(MuRecord(1, "Naruto", "Manga"));

        var (client, db) = NewFactory(muSearchJson: muJson, anilistThrows: true).CreateClientWithDb();
        var user = await Seed.UserAsync(db, Fake.AdminUser());
        Authorize(client, user);

        var response = await client.GetAsync("/api/discover?q=naruto");
        Assert.Equal(HttpStatusCode.OK, response.StatusCode);

        var res = await response.Content.ReadFromJsonAsync<JsonElement[]>();
        Assert.NotNull(res);
        Assert.Contains(res, r => r.GetProperty("source").GetString() == "mangaupdates");
    }

    [Fact]
    public async Task Search_AllAuthoritiesFail_Returns502()
    {
        var (client, db) = NewFactory(muSearchThrows: true, anilistThrows: true,
            mangadexThrows: true, novelupdatesThrows: true, wuxiaworldThrows: true).CreateClientWithDb();
        var user = await Seed.UserAsync(db, Fake.AdminUser());
        Authorize(client, user);

        var response = await client.GetAsync("/api/discover?q=test");
        Assert.Equal(HttpStatusCode.BadGateway, response.StatusCode);
    }

    [Fact]
    public async Task Search_InLibrary_ByNormalizedTitleAndContentType()
    {
        // Title was added from AniList (no mangaupdates_id), but search now returns it via AniList
        // in_library must be true even though the result has no mangaupdates_id match
        var alJson = AniListResponse(new AniListMedia(
            id: 101517,
            title: new AniListTitle("Solo Leveling"),
            format: "MANHWA",
            countryOfOrigin: "KR"));

        var (client, db) = NewFactory(anilistJson: alJson).CreateClientWithDb();
        var user = await Seed.UserAsync(db, Fake.AdminUser());
        Authorize(client, user);

        // Seed a title with no mangaupdates_id — sourced from AniList
        var title = Fake.Title();
        title.TitleName = "Solo Leveling";
        title.ContentType = "manhwa";
        title.MangaupdatesId = null;
        // title.MetadataSource = "anilist";  // uncomment once property exists
        // title.MetadataSourceId = "101517"; // uncomment once property exists
        await Seed.TitleAsync(db, user.Id, title);
        db.ChangeTracker.Clear();

        var res = await client.GetFromJsonAsync<JsonElement[]>("/api/discover?q=solo+leveling");

        Assert.NotNull(res);
        var solo = res.FirstOrDefault(r => r.GetProperty("title").GetString() == "Solo Leveling");
        Assert.NotNull(solo);
        Assert.True(solo.GetProperty("in_library").GetBoolean(), "in_library by normalized title+content_type");
        Assert.Equal(title.Id, solo.GetProperty("library_id").GetString());
    }

    // ── POST /api/discover/add — metadata_source ──────────────────────────────

    [Fact]
    public async Task AddManga_WithSourceAnilist_StoresMetadataSource()
    {
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = await Seed.UserAsync(db, Fake.AdminUser());
        Authorize(client, user);

        var res = await client.PostAsJsonAsync("/api/discover/add", new
        {
            source = "anilist",
            source_id = "101517",
            title = "Solo Leveling",
            content_type = "manhwa",
            status = "complete",
        });

        Assert.Equal(HttpStatusCode.OK, res.StatusCode);
        var body = await res.Content.ReadFromJsonAsync<JsonElement>();
        var titleId = body.GetProperty("id").GetString()!;

        db.ChangeTracker.Clear();
        var stored = await db.Titles.FindAsync(titleId);
        Assert.NotNull(stored);
        Assert.Equal("anilist", stored.MetadataSource);
        Assert.Equal("101517", stored.MetadataSourceId);
        Assert.Null(stored.MangaupdatesId); // not a MU title
    }

    [Fact]
    public async Task AddManga_BackwardCompat_MuIdOnly_StoresMetadataSource()
    {
        // Old clients pass only mangaupdates_id — must still work and auto-set metadata_source
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = await Seed.UserAsync(db, Fake.AdminUser());
        Authorize(client, user);

        var res = await client.PostAsJsonAsync("/api/discover/add", new
        {
            mangaupdates_id = "42",
            title = "Naruto",
            content_type = "manga",
            status = "ongoing",
        });

        Assert.Equal(HttpStatusCode.OK, res.StatusCode);
        var body = await res.Content.ReadFromJsonAsync<JsonElement>();
        var titleId = body.GetProperty("id").GetString()!;

        db.ChangeTracker.Clear();
        var stored = await db.Titles.FindAsync(titleId);
        Assert.NotNull(stored);
        Assert.Equal("42", stored.MangaupdatesId);
        Assert.Equal("mangaupdates", stored.MetadataSource);
        Assert.Equal("42", stored.MetadataSourceId);
    }

    [Fact]
    public async Task AddManga_Dedup_BySameSourceAndSourceId()
    {
        // Two adds with same source+source_id produce one title row
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = await Seed.UserAsync(db, Fake.AdminUser());
        Authorize(client, user);

        var payload = new
        {
            source = "anilist",
            source_id = "101517",
            title = "Solo Leveling",
            content_type = "manhwa",
            status = "complete",
        };

        var res1 = await client.PostAsJsonAsync("/api/discover/add", payload);
        var res2 = await client.PostAsJsonAsync("/api/discover/add", payload);

        Assert.Equal(HttpStatusCode.OK, res1.StatusCode);
        Assert.Equal(HttpStatusCode.OK, res2.StatusCode);

        var body1 = await res1.Content.ReadFromJsonAsync<JsonElement>();
        var body2 = await res2.Content.ReadFromJsonAsync<JsonElement>();
        Assert.Equal(body1.GetProperty("id").GetString(), body2.GetProperty("id").GetString());

        db.ChangeTracker.Clear();
        Assert.Equal(1, await db.Titles.CountAsync(t => t.MetadataSource == "anilist" && t.MetadataSourceId == "101517"));
    }

    // ── WuxiaWorld novel fallback (no CF, works when NU is down) ─────────────

    [Fact]
    public async Task Search_WuxiaWorld_Novel_HasSourceWuxiaworld()
    {
        var wwJson = WuxiaWorldResponse(("i-shall-seal-the-heavens", "I Shall Seal the Heavens", "completed"));

        var (client, db) = NewFactory(wuxiaworldJson: wwJson).CreateClientWithDb();
        var user = await Seed.UserAsync(db, Fake.AdminUser());
        Authorize(client, user);

        var res = await client.GetFromJsonAsync<JsonElement[]>("/api/discover?q=i+shall+seal");

        Assert.NotNull(res);
        var issh = res.FirstOrDefault(r => r.GetProperty("title").GetString() == "I Shall Seal the Heavens");
        Assert.NotNull(issh);
        Assert.Equal("wuxiaworld", issh.GetProperty("source").GetString());
        Assert.Equal("novel", issh.GetProperty("content_type").GetString());
    }

    [Fact]
    public async Task Search_WuxiaWorld_Deduped_ByNovelupdates_WhenBothReturn()
    {
        // Both NU and WW return the same novel — NU is designated authority → NU result survives
        var nuHtml = NovelUpdatesHtml("i-shall-seal-the-heavens", "I Shall Seal the Heavens");
        var wwJson = WuxiaWorldResponse(("i-shall-seal-the-heavens", "I Shall Seal the Heavens", "completed"));

        var (client, db) = NewFactory(novelupdatesHtml: nuHtml, wuxiaworldJson: wwJson).CreateClientWithDb();
        var user = await Seed.UserAsync(db, Fake.AdminUser());
        Authorize(client, user);

        var res = await client.GetFromJsonAsync<JsonElement[]>("/api/discover?q=i+shall+seal");

        Assert.NotNull(res);
        var isshResults = res.Where(r => r.GetProperty("title").GetString() == "I Shall Seal the Heavens").ToArray();
        Assert.Single(isshResults); // deduped to exactly 1
        Assert.Equal("novelupdates", isshResults[0].GetProperty("source").GetString()); // NU wins
    }

    [Fact]
    public async Task Search_WuxiaWorld_NovelsAppear_EvenWhenNovelupdatesFails()
    {
        // NU throws (CF blocked / CloakBrowser down) → WW results still appear
        var wwJson = WuxiaWorldResponse(("a-will-eternal", "A Will Eternal", "completed"));

        var (client, db) = NewFactory(wuxiaworldJson: wwJson, novelupdatesThrows: true).CreateClientWithDb();
        var user = await Seed.UserAsync(db, Fake.AdminUser());
        Authorize(client, user);

        var res = await client.GetFromJsonAsync<JsonElement[]>("/api/discover?q=a+will+eternal");

        Assert.NotNull(res);
        Assert.Contains(res, r => r.GetProperty("source").GetString() == "wuxiaworld");
    }

    // ── MU scope filter (ADR 0031: MU is manga-authority only) ───────────────

    [Fact]
    public async Task Search_MuNovel_ExcludedFromResults()
    {
        // MU returns "I Shall Seal the Heavens" as Novel.
        // NovelUpdates returns nothing.
        // The novel result must NOT appear — MU is not a novel authority (ADR 0031).
        var muJson = MuSearchResponse(MuRecord(1, "I Shall Seal the Heavens", "Novel"));

        var (client, db) = NewFactory(muSearchJson: muJson).CreateClientWithDb();
        var user = await Seed.UserAsync(db, Fake.AdminUser());
        Authorize(client, user);

        var res = await client.GetFromJsonAsync<JsonElement[]>("/api/discover?q=i+shall+seal");

        Assert.NotNull(res);
        Assert.DoesNotContain(res, r =>
            r.GetProperty("source").GetString() == "mangaupdates" &&
            r.GetProperty("content_type").GetString() == "novel");
    }

    [Fact]
    public async Task Search_MuManhwa_ExcludedFromResults()
    {
        // MU returns "Solo Leveling" as Manhwa. AniList returns nothing.
        // Must NOT appear — AniList is the manhwa authority; MU results for manhwa are filtered.
        var muJson = MuSearchResponse(MuRecord(1, "Solo Leveling", "Manhwa"));

        var (client, db) = NewFactory(muSearchJson: muJson).CreateClientWithDb();
        var user = await Seed.UserAsync(db, Fake.AdminUser());
        Authorize(client, user);

        var res = await client.GetFromJsonAsync<JsonElement[]>("/api/discover?q=solo+leveling");

        Assert.NotNull(res);
        Assert.DoesNotContain(res, r =>
            r.GetProperty("source").GetString() == "mangaupdates" &&
            r.GetProperty("content_type").GetString() == "manhwa");
    }

    // ── add_manga bg task: metadata routing switch (ADR 0031) ────────────────

    [Fact]
    public async Task AddManga_NovelSource_SyncLog_DoesNotCallMangaUpdates()
    {
        // Add a title with source="novelupdates".
        // Bg task must NOT attempt a MangaUpdates SeriesDetail call.
        // Sync log must NOT contain "Fetching metadata from MangaUpdates".
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = await Seed.UserAsync(db, Fake.AdminUser());
        Authorize(client, user);

        var res = await client.PostAsJsonAsync("/api/discover/add", new
        {
            source = "novelupdates",
            source_id = "i-shall-seal-the-heavens",
            title = "I Shall Seal the Heavens",
            content_type = "novel",
            status = "completed",
        });

        Assert.Equal(HttpStatusCode.OK, res.StatusCode);
        var body = await res.Content.ReadFromJsonAsync<JsonElement>();
        var titleId = body.GetProperty("id").GetString()!;

        // Wait for bg task to complete (sync_status = ready)
        await WaitForSyncReadyAsync(db, titleId);

        db.ChangeTracker.Clear();
        var logs = await db.SyncLogs
            .Where(l => l.TitleId == titleId)
            .Select(l => l.Message)
            .ToListAsync();

        Assert.DoesNotContain(logs, m => m.Contains("MangaUpdates"));
    }

    [Fact]
    public async Task AddManga_MangaSource_SyncLog_CallsMangaUpdates()
    {
        // Add a title with source="mangaupdates" and a valid ulong source_id.
        // Bg task must call MangaUpdates SeriesDetail for aliases.
        // Sync log must contain "Fetching metadata from MangaUpdates".
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = await Seed.UserAsync(db, Fake.AdminUser());
        Authorize(client, user);

        var res = await client.PostAsJsonAsync("/api/discover/add", new
        {
            source = "mangaupdates",
            source_id = "42",
            mangaupdates_id = "42",
            title = "Naruto",
            content_type = "manga",
            status = "ongoing",
        });

        Assert.Equal(HttpStatusCode.OK, res.StatusCode);
        var body = await res.Content.ReadFromJsonAsync<JsonElement>();
        var titleId = body.GetProperty("id").GetString()!;

        await WaitForSyncReadyAsync(db, titleId);

        db.ChangeTracker.Clear();
        var logs = await db.SyncLogs
            .Where(l => l.TitleId == titleId)
            .Select(l => l.Message)
            .ToListAsync();

        Assert.Contains(logs, m => m.Contains("MangaUpdates"));
    }

    // ── Source matching (MatchSourcesAsync) ──────────────────────────────────

    [Fact]
    public async Task AddManga_WithMatchingExternalSource_CreatesSourceLinks()
    {
        // Seed an external source for manga before adding title
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = await Seed.UserAsync(db, Fake.AdminUser());
        Authorize(client, user);

        // Seed a manga external source so MatchSourcesAsync finds it
        db.ExternalSources.Add(new ArrghServer.Data.Models.ExternalSource
        {
            Id = Guid.NewGuid().ToString(),
            SourceKey = "mangadex",
            Name = "MangaDex",
            BaseUrl = "http://plugin-host:4000",
            ContentTypes = "manga,manhwa,manhua,one-shot",
            Enabled = true,
            Priority = 10,
            CreatedAt = DateTime.UtcNow,
        });
        await db.SaveChangesAsync();

        var res = await client.PostAsJsonAsync("/api/discover/add", new
        {
            source = "mangaupdates",
            source_id = "1",
            mangaupdates_id = "1",
            title = "Naruto",
            content_type = "manga",
            status = "ongoing",
        });

        Assert.Equal(HttpStatusCode.OK, res.StatusCode);
        var body = await res.Content.ReadFromJsonAsync<JsonElement>();
        var titleId = body.GetProperty("id").GetString()!;

        // Wait for bg task
        await WaitForSyncReadyAsync(db, titleId);

        db.ChangeTracker.Clear();
        var sourceLinks = await db.TitleSources
            .Where(ts => ts.TitleId == titleId)
            .ToListAsync();

        Assert.NotEmpty(sourceLinks);
        Assert.Contains(sourceLinks, ts => ts.Source == "mangadex");
    }

    [Fact]
    public async Task AddManga_WithMatchingExternalSource_CreatesChapters()
    {
        // Source match finds title + fetches chapters → chapter rows + chapter_sources created
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = await Seed.UserAsync(db, Fake.AdminUser());
        Authorize(client, user);

        db.ExternalSources.Add(new ArrghServer.Data.Models.ExternalSource
        {
            Id = Guid.NewGuid().ToString(), SourceKey = "mangadex", Name = "MangaDex",
            BaseUrl = "http://plugin-host:4000", ContentTypes = "manga,manhwa,manhua,one-shot",
            Enabled = true, Priority = 10, CreatedAt = DateTime.UtcNow,
        });
        await db.SaveChangesAsync();

        var res = await client.PostAsJsonAsync("/api/discover/add", new
        {
            source = "mangaupdates", source_id = "1", mangaupdates_id = "1",
            title = "Naruto", content_type = "manga", status = "ongoing",
        });

        Assert.Equal(HttpStatusCode.OK, res.StatusCode);
        var body = await res.Content.ReadFromJsonAsync<JsonElement>();
        var titleId = body.GetProperty("id").GetString()!;

        await WaitForSyncReadyAsync(db, titleId);
        db.ChangeTracker.Clear();

        // Chapter rows must exist (created from plugin chapters response)
        var chapters = await db.Chapters.Where(c => c.TitleId == titleId).ToListAsync();
        Assert.NotEmpty(chapters);

        // chapter_sources must link them
        var chapterIds = chapters.Select(c => c.Id).ToList();
        var sources = await db.ChapterSources.Where(cs => chapterIds.Contains(cs.ChapterId)).ToListAsync();
        Assert.NotEmpty(sources);
    }

    [Fact]
    public async Task AddManga_NoMatchingExternalSource_NoSourceLinks()
    {
        // No external sources seeded → source matching finds nothing → no title_sources rows
        var (client, db) = NewFactory().CreateClientWithDb();
        var user = await Seed.UserAsync(db, Fake.AdminUser());
        Authorize(client, user);

        var res = await client.PostAsJsonAsync("/api/discover/add", new
        {
            source = "anilist",
            source_id = "101517",
            title = "Solo Leveling",
            content_type = "manhwa",
            status = "completed",
        });

        Assert.Equal(HttpStatusCode.OK, res.StatusCode);
        var body = await res.Content.ReadFromJsonAsync<JsonElement>();
        var titleId = body.GetProperty("id").GetString()!;

        await WaitForSyncReadyAsync(db, titleId);

        db.ChangeTracker.Clear();
        var count = await db.TitleSources.CountAsync(ts => ts.TitleId == titleId);
        Assert.Equal(0, count);
    }

    // ── AniList synonym alias storage + fuzzy source matching ────────────────

    [Fact]
    public async Task AddManhwa_AniListSource_StoresSynonymsAsAliases()
    {
        // AniList synonyms call returns known alternate titles
        var synonymsJson = """{"data":{"Media":{"synonyms":["Everything Is Agreed Upon","다 합의됐어"]}}}""";
        var (client, db) = NewFactory(anilistSynonymsJson: synonymsJson).CreateClientWithDb();
        var user = await Seed.UserAsync(db, Fake.AdminUser());
        Authorize(client, user, allowExplicit: true);

        var res = await client.PostAsJsonAsync("/api/discover/add", new
        {
            source = "anilist",
            source_id = "128789",
            title = "Only With Consent",
            content_type = "manhwa",
            status = "ongoing",
            is_explicit = true,
        });

        Assert.Equal(HttpStatusCode.OK, res.StatusCode);
        var body = await res.Content.ReadFromJsonAsync<JsonElement>();
        var titleId = body.GetProperty("id").GetString()!;
        await WaitForSyncReadyAsync(db, titleId);

        db.ChangeTracker.Clear();
        var aliases = await db.TitleAliases.Where(a => a.TitleId == titleId).Select(a => a.Alias).ToListAsync();
        Assert.Contains(aliases, a => a.Contains("Everything Is Agreed Upon"));

        var logs = await db.SyncLogs.Where(l => l.TitleId == titleId).Select(l => l.Message).ToListAsync();
        Assert.Contains(logs, m => m.Contains("synonym"));
    }

    [Fact]
    public async Task AddManhwa_AniListSource_EmptySynonyms_SyncStillCompletes()
    {
        // GetSynonymsAsync returns [] — sync should reach "ready" with zero aliases stored
        var synonymsJson = """{"data":{"Media":{"synonyms":[]}}}""";
        var (client, db) = NewFactory(anilistSynonymsJson: synonymsJson).CreateClientWithDb();
        var user = await Seed.UserAsync(db, Fake.AdminUser());
        Authorize(client, user);

        var res = await client.PostAsJsonAsync("/api/discover/add", new
        {
            source = "anilist",
            source_id = "999",
            title = "Some Manhwa",
            content_type = "manhwa",
            status = "ongoing",
        });

        Assert.Equal(HttpStatusCode.OK, res.StatusCode);
        var body = await res.Content.ReadFromJsonAsync<JsonElement>();
        var titleId = body.GetProperty("id").GetString()!;
        await WaitForSyncReadyAsync(db, titleId);

        db.ChangeTracker.Clear();
        var aliasCount = await db.TitleAliases.CountAsync(a => a.TitleId == titleId);
        Assert.Equal(0, aliasCount);

        var title = await db.Titles.FindAsync(titleId);
        Assert.Equal("ready", title!.SyncStatus);
    }

    [Fact]
    public async Task SourceMatching_FuzzyTitle_LinksSourceWhenHyphenVariant()
    {
        // Plugin search echoes "Soeun" (no hyphen) for query "So-Eun" (hyphenated)
        var pluginOverrides = new Dictionary<string, string> { ["manga18fx"] = "Soeun" };
        var synonymsJson = """{"data":{"Media":{"synonyms":[]}}}""";
        var (client, db) = NewFactory(anilistSynonymsJson: synonymsJson, pluginSearchTitleMap: pluginOverrides).CreateClientWithDb();
        var user = await Seed.UserAsync(db, Fake.AdminUser());
        Authorize(client, user, allowExplicit: true);

        db.ExternalSources.Add(new ArrghServer.Data.Models.ExternalSource
        {
            Id = Guid.NewGuid().ToString(), SourceKey = "manga18fx", Name = "Manga18fx",
            BaseUrl = "http://plugin-host:4000", ContentTypes = "manhwa",
            Enabled = true, DefaultExplicit = true, Priority = 75, CreatedAt = DateTime.UtcNow,
        });
        await db.SaveChangesAsync();

        var res = await client.PostAsJsonAsync("/api/discover/add", new
        {
            source = "anilist",
            source_id = "99999",
            title = "So-Eun",
            content_type = "manhwa",
            status = "ongoing",
            is_explicit = true,
        });

        Assert.Equal(HttpStatusCode.OK, res.StatusCode);
        var body = await res.Content.ReadFromJsonAsync<JsonElement>();
        var titleId = body.GetProperty("id").GetString()!;
        await WaitForSyncReadyAsync(db, titleId);

        db.ChangeTracker.Clear();
        var sources = await db.TitleSources.Where(ts => ts.TitleId == titleId).ToListAsync();
        Assert.NotEmpty(sources);
    }

    [Fact]
    public async Task SourceMatching_AliasMatch_LinksSourceWhenTranslationDiffers()
    {
        // Plugin search returns "Everything Is Agreed" for query "Only With Consent"
        // AniList synonym "Everything Is Agreed Upon" bridges the gap via TitleMatches
        var pluginOverrides = new Dictionary<string, string> { ["manga18fx"] = "Everything Is Agreed" };
        var synonymsJson = """{"data":{"Media":{"synonyms":["Everything Is Agreed Upon"]}}}""";
        var (client, db) = NewFactory(anilistSynonymsJson: synonymsJson, pluginSearchTitleMap: pluginOverrides).CreateClientWithDb();
        var user = await Seed.UserAsync(db, Fake.AdminUser());
        Authorize(client, user, allowExplicit: true);

        db.ExternalSources.Add(new ArrghServer.Data.Models.ExternalSource
        {
            Id = Guid.NewGuid().ToString(), SourceKey = "manga18fx", Name = "Manga18fx",
            BaseUrl = "http://plugin-host:4000", ContentTypes = "manhwa",
            Enabled = true, DefaultExplicit = true, Priority = 75, CreatedAt = DateTime.UtcNow,
        });
        await db.SaveChangesAsync();

        var res = await client.PostAsJsonAsync("/api/discover/add", new
        {
            source = "anilist",
            source_id = "128789",
            title = "Only With Consent",
            content_type = "manhwa",
            status = "ongoing",
            is_explicit = true,
        });

        Assert.Equal(HttpStatusCode.OK, res.StatusCode);
        var body = await res.Content.ReadFromJsonAsync<JsonElement>();
        var titleId = body.GetProperty("id").GetString()!;
        await WaitForSyncReadyAsync(db, titleId);

        db.ChangeTracker.Clear();
        var sources = await db.TitleSources.Where(ts => ts.TitleId == titleId).ToListAsync();
        Assert.NotEmpty(sources);
    }

    [Fact]
    public async Task SourceMatching_NoAliasMatch_NoSourceLink()
    {
        // Source exists, plugin returns a result, but the title is completely unrelated —
        // no match via fuzzy on main title or any alias → warning logged, no title_source row
        var pluginOverrides = new Dictionary<string, string> { ["manga18fx"] = "Tower of God" };
        var synonymsJson = """{"data":{"Media":{"synonyms":["소의 탑"]}}}""";
        var (client, db) = NewFactory(anilistSynonymsJson: synonymsJson, pluginSearchTitleMap: pluginOverrides).CreateClientWithDb();
        var user = await Seed.UserAsync(db, Fake.AdminUser());
        Authorize(client, user, allowExplicit: true);

        db.ExternalSources.Add(new ArrghServer.Data.Models.ExternalSource
        {
            Id = Guid.NewGuid().ToString(), SourceKey = "manga18fx", Name = "Manga18fx",
            BaseUrl = "http://plugin-host:4000", ContentTypes = "manhwa",
            Enabled = true, DefaultExplicit = true, Priority = 75, CreatedAt = DateTime.UtcNow,
        });
        await db.SaveChangesAsync();

        var res = await client.PostAsJsonAsync("/api/discover/add", new
        {
            source = "anilist",
            source_id = "99",
            title = "Only With Consent",
            content_type = "manhwa",
            status = "ongoing",
            is_explicit = true,
        });

        Assert.Equal(HttpStatusCode.OK, res.StatusCode);
        var body = await res.Content.ReadFromJsonAsync<JsonElement>();
        var titleId = body.GetProperty("id").GetString()!;
        await WaitForSyncReadyAsync(db, titleId);

        db.ChangeTracker.Clear();
        var sourceCount = await db.TitleSources.CountAsync(ts => ts.TitleId == titleId);
        Assert.Equal(0, sourceCount);

        var warnings = await db.SyncWarnings.Where(w => w.TitleId == titleId).ToListAsync();
        Assert.NotEmpty(warnings);
    }

    [Fact]
    public async Task SourceMatching_PartialMiss_NoWarning_WhenOtherSourceLinked()
    {
        // Two sources: toonily matches, asurascans returns no results.
        // Warning must NOT fire because at least one source was linked.
        var pluginOverrides = new Dictionary<string, string> { ["asurascans"] = "" };
        var (client, db) = NewFactory(pluginSearchTitleMap: pluginOverrides).CreateClientWithDb();
        var user = await Seed.UserAsync(db, Fake.AdminUser());
        Authorize(client, user, allowExplicit: true);

        foreach (var (key, name, priority) in new[]
        {
            ("toonily",    "Toonily",    50),
            ("asurascans", "AsuraScans", 60),
        })
        {
            db.ExternalSources.Add(new ArrghServer.Data.Models.ExternalSource
            {
                Id = Guid.NewGuid().ToString(), SourceKey = key, Name = name,
                BaseUrl = "http://plugin-host:4000", ContentTypes = "manhwa",
                Enabled = true, DefaultExplicit = false, Priority = priority, CreatedAt = DateTime.UtcNow,
            });
        }
        await db.SaveChangesAsync();

        // Override: asurascans returns empty list, toonily returns matching title
        var res = await client.PostAsJsonAsync("/api/discover/add", new
        {
            source = "anilist", source_id = "42", title = "Solo Leveling",
            content_type = "manhwa", status = "ongoing",
        });
        Assert.Equal(HttpStatusCode.OK, res.StatusCode);
        var titleId = (await res.Content.ReadFromJsonAsync<JsonElement>()).GetProperty("id").GetString()!;
        await WaitForSyncReadyAsync(db, titleId);

        db.ChangeTracker.Clear();
        var sourceCount = await db.TitleSources.CountAsync(ts => ts.TitleId == titleId);
        Assert.True(sourceCount >= 1, "toonily should have been linked");

        var warnings = await db.SyncWarnings.Where(w => w.TitleId == titleId).ToListAsync();
        Assert.Empty(warnings);
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    static async Task WaitForSyncReadyAsync(ArrghServer.Data.AppDbContext db, string titleId, int maxMs = 3000)
    {
        var sw = System.Diagnostics.Stopwatch.StartNew();
        while (sw.ElapsedMilliseconds < maxMs)
        {
            db.ChangeTracker.Clear();
            var status = await db.Titles
                .Where(t => t.Id == titleId)
                .Select(t => t.SyncStatus)
                .FirstOrDefaultAsync();
            if (status == "ready") return;
            await Task.Delay(50);
        }
    }
}

// ── Fan-out test factory ──────────────────────────────────────────────────────

public class FanOutDiscoverFactory : AppFactory
{
    readonly string _muSearchJson;
    readonly string _anilistJson;
    readonly string _mangadexJson;
    readonly string _novelupdatesHtml;
    readonly string _wuxiaworldJson;
    readonly string _ehentaiSearchHtml;
    readonly string _ehentaiGdataJson;
    readonly bool _muSearchThrows;
    readonly bool _anilistThrows;
    readonly bool _mangadexThrows;
    readonly bool _novelupdatesThrows;
    readonly bool _wuxiaworldThrows;
    readonly string? _anilistSynonymsJson;
    readonly Dictionary<string, string>? _pluginSearchTitleMap;

    public FanOutDiscoverFactory(
        string muSearchJson = """{"results":[]}""",
        string anilistJson = """{"data":{"Page":{"media":[]}}}""",
        string mangadexJson = """{"data":[]}""",
        string novelupdatesHtml = """<html><body></body></html>""",
        string wuxiaworldJson = """{"items":[]}""",
        string ehentaiSearchHtml = """<html><body></body></html>""",
        string ehentaiGdataJson = """{"gmetadata":[]}""",
        bool muSearchThrows = false,
        bool anilistThrows = false,
        bool mangadexThrows = false,
        bool novelupdatesThrows = false,
        bool wuxiaworldThrows = false,
        string? anilistSynonymsJson = null,
        Dictionary<string, string>? pluginSearchTitleMap = null)
    {
        _muSearchJson = muSearchJson;
        _anilistJson = anilistJson;
        _mangadexJson = mangadexJson;
        _novelupdatesHtml = novelupdatesHtml;
        _wuxiaworldJson = wuxiaworldJson;
        _ehentaiSearchHtml = ehentaiSearchHtml;
        _ehentaiGdataJson = ehentaiGdataJson;
        _muSearchThrows = muSearchThrows;
        _anilistThrows = anilistThrows;
        _mangadexThrows = mangadexThrows;
        _novelupdatesThrows = novelupdatesThrows;
        _wuxiaworldThrows = wuxiaworldThrows;
        _anilistSynonymsJson = anilistSynonymsJson;
        _pluginSearchTitleMap = pluginSearchTitleMap;
    }

    protected override IHost CreateHost(IHostBuilder builder)
    {
        var muSearch = _muSearchJson;
        var anilist = _anilistJson;
        var mangadex = _mangadexJson;
        var nu = _novelupdatesHtml;
        var ww = _wuxiaworldJson;
        var ehHtml = _ehentaiSearchHtml;
        var ehGdata = _ehentaiGdataJson;
        var muThrows = _muSearchThrows;
        var alThrows = _anilistThrows;
        var mdThrows = _mangadexThrows;
        var nuThrows = _novelupdatesThrows;
        var wwThrows = _wuxiaworldThrows;
        var alSynonyms = _anilistSynonymsJson;
        var pluginTitleMap = _pluginSearchTitleMap;

        builder.ConfigureServices(services =>
        {
            services.RemoveAll<IHttpClientFactory>();
            services.AddSingleton<IHttpClientFactory>(
                new FanOutFakeHttpClientFactory(
                    muSearch, anilist, mangadex, nu, ww, ehHtml, ehGdata,
                    muThrows, alThrows, mdThrows, nuThrows, wwThrows,
                    alSynonyms, pluginTitleMap));
        });
        return base.CreateHost(builder);
    }
}

file class FanOutFakeHttpClientFactory(
    string muSearchJson, string anilistJson, string mangadexJson,
    string novelupdatesHtml, string wuxiaworldJson, string ehentaiSearchHtml, string ehentaiGdataJson,
    bool muSearchThrows, bool anilistThrows, bool mangadexThrows, bool novelupdatesThrows, bool wuxiaworldThrows,
    string? anilistSynonymsJson = null, Dictionary<string, string>? pluginSearchTitleMap = null)
    : IHttpClientFactory
{
    public HttpClient CreateClient(string name) =>
        new(new FanOutFakeHandler(
            muSearchJson, anilistJson, mangadexJson, novelupdatesHtml, wuxiaworldJson,
            ehentaiSearchHtml, ehentaiGdataJson,
            muSearchThrows, anilistThrows, mangadexThrows, novelupdatesThrows, wuxiaworldThrows,
            anilistSynonymsJson, pluginSearchTitleMap));
}

file class FanOutFakeHandler(
    string muSearchJson, string anilistJson, string mangadexJson,
    string novelupdatesHtml, string wuxiaworldJson, string ehentaiSearchHtml, string ehentaiGdataJson,
    bool muSearchThrows, bool anilistThrows, bool mangadexThrows, bool novelupdatesThrows, bool wuxiaworldThrows,
    string? anilistSynonymsJson = null, Dictionary<string, string>? pluginSearchTitleMap = null)
    : HttpMessageHandler
{
    protected override async Task<HttpResponseMessage> SendAsync(
        HttpRequestMessage req, CancellationToken ct)
    {
        var host = req.RequestUri?.Host ?? "";
        var path = req.RequestUri?.PathAndQuery ?? "";

        // ── MangaUpdates ────────────────────────────────────────────────────
        if (host.Contains("mangaupdates"))
        {
            if (path.Contains("/series/search"))
            {
                if (muSearchThrows) throw new HttpRequestException("MU unavailable");
                return Json(muSearchJson);
            }
            if (path.Contains("/releases/search"))
                return Json("""{"results":[]}""");
            if (path.Contains("/series/"))
                return Json("""{"series_id":1,"title":"Detail","description":null,"image":null,"type":null,"year":null,"status":null,"authors":null,"genres":null}""");
        }

        // ── AniList ─────────────────────────────────────────────────────────
        if (host.Contains("anilist"))
        {
            if (anilistThrows) throw new HttpRequestException("AniList unavailable");
            // Differentiate search (contains "isAdult") from synonyms detail call
            if (anilistSynonymsJson is not null && req.Content is not null)
            {
                var bodyStr = await req.Content.ReadAsStringAsync(ct);
                if (!bodyStr.Contains("isAdult"))
                    return Json(anilistSynonymsJson);
            }
            return Json(anilistJson);
        }

        // ── MangaDex Metadata ───────────────────────────────────────────────
        if (host.Contains("mangadex"))
        {
            if (mangadexThrows) throw new HttpRequestException("MangaDex unavailable");
            return Json(mangadexJson);
        }

        // ── WuxiaWorld Metadata ─────────────────────────────────────────────
        if (host.Contains("wuxiaworld"))
        {
            if (wuxiaworldThrows) throw new HttpRequestException("WuxiaWorld unavailable");
            return Json(wuxiaworldJson);
        }

        // ── NovelUpdates — proxied through plugin-host (CF-protected, needs CloakBrowser)
        // Dev: http://localhost:4000 (from appsettings.Development.json)
        // Prod: http://plugin-host:4000
        if ((host == "plugin-host" || host == "localhost") && path.Contains("/novelupdates/search"))
        {
            if (novelupdatesThrows) throw new HttpRequestException("NovelUpdates unavailable");
            // Convert HTML fixture to the plugin search JSON format
            var parsed = NovelUpdatesService.ParseHtml(novelupdatesHtml);
            var json = System.Text.Json.JsonSerializer.Serialize(
                parsed.Select(s => new { id = s.SourceId, title = s.Title, cover_url = s.CoverUrl, status = s.Status, content_type = "novel" }));
            return Json(json);
        }

        // ── E-Hentai ────────────────────────────────────────────────────────
        if (host.Contains("e-hentai"))
        {
            // api.e-hentai.org/api.php → gdata JSON
            if (path.Contains("api.php"))
                return Json(ehentaiGdataJson);
            // e-hentai.org search → HTML gallery listing
            return Html(ehentaiSearchHtml);
        }

        // ── Source matching calls (MatchSourcesAsync) ────────────────────────
        // plugin-host /{sourceKey}/search?q=... → returns [{id,title}]
        if (host == "plugin-host" && path.Contains("/search?q="))
        {
            // Extract q= from query string manually
            var query = req.RequestUri?.Query ?? "";
            var qIdx = query.IndexOf("q=", StringComparison.Ordinal);
            var rawQ = qIdx >= 0 ? query[(qIdx + 2)..].Split('&')[0] : "";
            var qParam = Uri.UnescapeDataString(rawQ.Replace('+', ' '));
            var sourceKey = path.TrimStart('/').Split('/')[0];
            // Allow test to override what title the plugin returns
            var returnTitle = pluginSearchTitleMap is not null && pluginSearchTitleMap.TryGetValue(sourceKey, out var mapped)
                ? mapped : qParam;
            var json = System.Text.Json.JsonSerializer.Serialize(new[]
            {
                new { id = $"{sourceKey}-source-id", title = returnTitle }
            });
            return Json(json);
        }

        // plugin-host /{sourceKey}/manga/{id}/chapters → returns [{source_id,number}]
        if (host == "plugin-host" && path.Contains("/manga/") && path.Contains("/chapters"))
        {
            var json = System.Text.Json.JsonSerializer.Serialize(new[]
            {
                new { source_id = "ch-source-1", number = 1.0 },
                new { source_id = "ch-source-2", number = 2.0 },
            });
            return Json(json);
        }

        // catch-all: covers, etc. — 200 empty
        return new HttpResponseMessage(HttpStatusCode.OK) { Content = new ByteArrayContent([]) };
    }

    static HttpResponseMessage Json(string json) => new(HttpStatusCode.OK)
    {
        Content = new StringContent(json, System.Text.Encoding.UTF8, "application/json"),
    };

    static HttpResponseMessage Html(string html) => new(HttpStatusCode.OK)
    {
        Content = new StringContent(html, System.Text.Encoding.UTF8, "text/html"),
    };
}
