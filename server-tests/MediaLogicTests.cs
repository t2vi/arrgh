using ArrghServer.Api;
using ArrghServer.Services;
using Xunit;

namespace ArrghServer.Tests;

/// <summary>
/// Unit tests for Media pure helpers — no HTTP, no DB.
/// </summary>
[Trait("Category", TestCategories.Unit)]
public class MediaLogicTests
{
    // ── DetectContentType ─────────────────────────────────────────────────────

    [Fact] public void DetectContentType_Jpeg() =>
        Assert.Equal("image/jpeg", Media.DetectContentType([0xFF, 0xD8, 0x00, 0x00]));

    [Fact] public void DetectContentType_Png() =>
        Assert.Equal("image/png", Media.DetectContentType([0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]));

    [Fact] public void DetectContentType_Webp()
    {
        byte[] data = [0x52, 0x49, 0x46, 0x46,  // RIFF
                       0x00, 0x00, 0x00, 0x00,
                       0x57, 0x45, 0x42, 0x50]; // WEBP
        Assert.Equal("image/webp", Media.DetectContentType(data));
    }

    [Fact] public void DetectContentType_Gif() =>
        Assert.Equal("image/gif", Media.DetectContentType("GIF89a\x00\x00"u8.ToArray()));

    [Fact] public void DetectContentType_Avif()
    {
        byte[] data = [0x00, 0x00, 0x00, 0x1C, 0x66, 0x74, 0x79, 0x70, 0x61, 0x76, 0x69, 0x66];
        Assert.Equal("image/avif", Media.DetectContentType(data));
    }

    [Fact] public void DetectContentType_TooShort_ReturnsNull() =>
        Assert.Null(Media.DetectContentType([0xFF, 0xD8, 0x00]));

    [Fact] public void DetectContentType_Empty_ReturnsNull() =>
        Assert.Null(Media.DetectContentType([]));

    [Fact] public void DetectContentType_Unknown_ReturnsNull() =>
        Assert.Null(Media.DetectContentType([0x00, 0x01, 0x02, 0x03]));

    // ── StripJpegIcc ──────────────────────────────────────────────────────────

    [Fact]
    public void StripJpegIcc_NonJpeg_ReturnedUnchanged()
    {
        byte[] png = [0x89, 0x50, 0x4E, 0x47, 0x00];
        Assert.Equal(png, Media.StripJpegIcc(png));
    }

    [Fact]
    public void StripJpegIcc_TooShort_ReturnedUnchanged()
    {
        Assert.Equal<byte>([0xFF], Media.StripJpegIcc([0xFF]));
        Assert.Empty(Media.StripJpegIcc([]));
    }

    [Fact]
    public void StripJpegIcc_JpegWithoutIcc_PreservesSoi()
    {
        byte[] jpeg = [0xFF, 0xD8, 0xFF, 0xD9]; // SOI + EOI
        var result = Media.StripJpegIcc(jpeg);
        Assert.Equal<byte>([0xFF, 0xD8], result[..2]);
    }

    [Fact]
    public void StripJpegIcc_StripsIccSegment()
    {
        var iccPayload = "ICC_PROFILE\0fake icc data"u8.ToArray();
        var segLen = (ushort)(2 + iccPayload.Length);
        var jpeg = new List<byte> { 0xFF, 0xD8 };     // SOI
        jpeg.AddRange([0xFF, 0xE2]);                   // APP2
        jpeg.Add((byte)(segLen >> 8));
        jpeg.Add((byte)(segLen & 0xFF));
        jpeg.AddRange(iccPayload);
        jpeg.AddRange([0xFF, 0xD9]);                   // EOI

        var result = Media.StripJpegIcc(jpeg.ToArray());

        Assert.False(ContainsSequence(result, "ICC_PROFILE\0"u8.ToArray()),
            "ICC_PROFILE segment should be stripped");
        Assert.Equal<byte>([0xFF, 0xD8], result[..2]);
    }

    // ── IsImage ───────────────────────────────────────────────────────────────

    [Theory]
    [InlineData("page.jpg")]
    [InlineData("page.jpeg")]
    [InlineData("page.PNG")]
    [InlineData("page.WEBP")]
    [InlineData("page.avif")]
    public void IsImage_KnownExtensions_ReturnsTrue(string name) =>
        Assert.True(Media.IsImage(name));

    [Theory]
    [InlineData("page.txt")]
    [InlineData("page.cbz")]
    [InlineData("page")]
    [InlineData("page.html")]
    public void IsImage_NonImage_ReturnsFalse(string name) =>
        Assert.False(Media.IsImage(name));

    // ── RootDomainReferer ─────────────────────────────────────────────────────

    [Fact] public void RootDomainReferer_Subdomain_ExtractsRoot() =>
        Assert.Equal("https://mangapill.com", Media.RootDomainReferer("https://cdn.mangapill.com/img/page.jpg"));

    [Fact] public void RootDomainReferer_ApexDomain_ReturnsSelf() =>
        Assert.Equal("https://mangadex.org", Media.RootDomainReferer("https://mangadex.org/chapter/abc"));

    [Fact] public void RootDomainReferer_InvalidUrl_ReturnsEmpty() =>
        Assert.Equal("", Media.RootDomainReferer("not-a-url"));

    [Fact] public void RootDomainReferer_Empty_ReturnsEmpty() =>
        Assert.Equal("", Media.RootDomainReferer(""));

    [Fact] public void RootDomainReferer_PreservesScheme() =>
        Assert.Equal("http://example.com", Media.RootDomainReferer("http://sub.example.com/path"));

    // ── NormalizeTitle ────────────────────────────────────────────────────────

    [Fact] public void NormalizeTitle_ReplacesNonAlphanumericWithSpaces() =>
        Assert.Equal("one piece", Media.NormalizeTitle("One Piece"));

    [Fact] public void NormalizeTitle_CollapseMultipleSpaces() =>
        Assert.Equal("a b c", Media.NormalizeTitle("a  --  b   c"));

    [Fact] public void NormalizeTitle_Lowercase() =>
        Assert.Equal("naruto", Media.NormalizeTitle("NARUTO"));

    [Fact] public void NormalizeTitle_StripsPunctuation() =>
        Assert.Equal("i shall seal the heavens", Media.NormalizeTitle("I Shall Seal the Heavens!"));

    // ── GetChapterPage ────────────────────────────────────────────────────────

    [Fact]
    public async Task GetChapterPage_MissingPath_ReturnsNull() =>
        Assert.Null(await Media.GetChapterPageAsync("/nonexistent/path", 0));

    [Fact]
    public async Task GetChapterPage_DirPage_ReadsCorrectFile()
    {
        var dir = Directory.CreateTempSubdirectory("arrgh-test-");
        try
        {
            await File.WriteAllBytesAsync(Path.Combine(dir.FullName, "01.jpg"), [0xFF, 0xD8, 0x01]);
            await File.WriteAllBytesAsync(Path.Combine(dir.FullName, "02.jpg"), [0xFF, 0xD8, 0x02]);

            var page0 = await Media.GetChapterPageAsync(dir.FullName, 0);
            var page1 = await Media.GetChapterPageAsync(dir.FullName, 1);

            Assert.Equal<byte>([0xFF, 0xD8, 0x01], page0!);
            Assert.Equal<byte>([0xFF, 0xD8, 0x02], page1!);
        }
        finally { dir.Delete(recursive: true); }
    }

    [Fact]
    public async Task GetChapterPage_DirOutOfRange_ReturnsNull()
    {
        var dir = Directory.CreateTempSubdirectory("arrgh-test-");
        try
        {
            await File.WriteAllBytesAsync(Path.Combine(dir.FullName, "01.jpg"), [0xFF, 0xD8]);
            Assert.Null(await Media.GetChapterPageAsync(dir.FullName, 5));
        }
        finally { dir.Delete(recursive: true); }
    }

    [Fact]
    public async Task GetChapterPage_ZipPage_ExtractsCorrectEntry()
    {
        var zipPath = Path.GetTempFileName() + ".cbz";
        try
        {
            using (var zip = System.IO.Compression.ZipFile.Open(zipPath, System.IO.Compression.ZipArchiveMode.Create))
            {
                using (var s1 = zip.CreateEntry("01.jpg").Open())
                    await s1.WriteAsync(new byte[] { 0xFF, 0xD8, 0x01 });
                using (var s2 = zip.CreateEntry("02.jpg").Open())
                    await s2.WriteAsync(new byte[] { 0xFF, 0xD8, 0x02 });
            }

            var page0 = await Media.GetChapterPageAsync(zipPath, 0);
            Assert.Equal<byte>([0xFF, 0xD8, 0x01], page0!);
        }
        finally { if (File.Exists(zipPath)) File.Delete(zipPath); }
    }

    // ── PageCacheService ──────────────────────────────────────────────────────

    [Fact]
    public void PageCache_MissOnEmpty() =>
        Assert.Null(new PageCacheService().Get("key"));

    [Fact]
    public void PageCache_HitAfterSet()
    {
        var cache = new PageCacheService();
        var pages = new List<PageUrlEntry> { new("https://example.com/1.jpg", null) };
        cache.Set("key", pages);
        Assert.Equal(pages, cache.Get("key"));
    }

    // ── helpers ───────────────────────────────────────────────────────────────

    static bool ContainsSequence(byte[] haystack, byte[] needle)
    {
        for (var i = 0; i <= haystack.Length - needle.Length; i++)
            if (haystack.AsSpan(i, needle.Length).SequenceEqual(needle)) return true;
        return false;
    }
}
