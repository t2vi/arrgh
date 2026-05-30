using System.Text.Json;
using ArrghServer.Api;
using Xunit;

namespace ArrghServer.Tests;

/// <summary>
/// Unit tests for PatchTitleBody JSON deserialization.
/// Catches naming-policy / JsonElement? edge cases without spinning up HTTP stack.
/// </summary>
[Trait("Category", TestCategories.Unit)]
public class PatchTitleBodyTests
{
    // Global app options: snake_case + case-insensitive (matches Program.cs ConfigureHttpJsonOptions)
    static readonly JsonSerializerOptions Opts = new()
    {
        PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower,
        PropertyNameCaseInsensitive = true,
    };

    static PatchTitleBody Deserialize(string json) =>
        JsonSerializer.Deserialize<PatchTitleBody>(json, Opts)!;

    // ── auto_download ─────────────────────────────────────────────────────────

    [Fact]
    public void AutoDownload_True_Parsed()
    {
        var b = Deserialize("""{"auto_download":true}""");
        Assert.True(b.AutoDownload);
    }

    [Fact]
    public void AutoDownload_False_Parsed()
    {
        var b = Deserialize("""{"auto_download":false}""");
        Assert.False(b.AutoDownload);
    }

    [Fact]
    public void AutoDownload_Absent_IsNull()
    {
        var b = Deserialize("{}");
        Assert.Null(b.AutoDownload);
    }

    // ── reader_mode ───────────────────────────────────────────────────────────

    [Fact]
    public void ReaderMode_String_HasValue()
    {
        var b = Deserialize("""{"reader_mode":"scroll"}""");
        Assert.True(b.ReaderMode.HasValue);
        Assert.Equal(JsonValueKind.String, b.ReaderMode!.Value.ValueKind);
        Assert.Equal("scroll", b.ReaderMode.Value.GetString());
    }

    [Fact]
    public void ReaderMode_Absent_IsNull()
    {
        var b = Deserialize("{}");
        Assert.False(b.ReaderMode.HasValue);
    }

    [Fact]
    public void ReaderMode_JsonNull_BehaviorDocumented()
    {
        // System.Text.Json deserializes JSON null → C# null for JsonElement?
        // HasValue=false, indistinguishable from absent. Known limitation — tracked.
        var b = Deserialize("""{"reader_mode":null}""");
        Assert.False(b.ReaderMode.HasValue);
    }

    // ── is_explicit / content_type ────────────────────────────────────────────

    [Fact]
    public void IsExplicit_True_Parsed()
    {
        var b = Deserialize("""{"is_explicit":true}""");
        Assert.True(b.IsExplicit);
    }

    [Fact]
    public void ContentType_Manga_Parsed()
    {
        var b = Deserialize("""{"content_type":"manga"}""");
        Assert.Equal("manga", b.ContentType);
    }

    [Fact]
    public void ContentType_Absent_IsNull()
    {
        var b = Deserialize("{}");
        Assert.Null(b.ContentType);
    }

    // ── combined payload ──────────────────────────────────────────────────────

    [Fact]
    public void MultipleFields_AllParsed()
    {
        var b = Deserialize("""
            {
                "auto_download": true,
                "reader_mode": "paged",
                "is_explicit": false,
                "content_type": "manhwa"
            }
            """);

        Assert.True(b.AutoDownload);
        Assert.Equal("paged", b.ReaderMode!.Value.GetString());
        Assert.False(b.IsExplicit);
        Assert.Equal("manhwa", b.ContentType);
    }

    [Fact]
    public void EmptyObject_AllFieldsNull()
    {
        var b = Deserialize("{}");
        Assert.Null(b.AutoDownload);
        Assert.False(b.ReaderMode.HasValue);
        Assert.False(b.DownloadDir.HasValue);
        Assert.Null(b.IsExplicit);
        Assert.Null(b.CoverUrl);
        Assert.Null(b.ContentType);
    }
}
