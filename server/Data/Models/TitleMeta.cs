using System.ComponentModel.DataAnnotations;

namespace ArrghServer.Data.Models;

public class TitleMeta
{
    [Key] public string TitleKey { get; set; } = null!;
    public string? CoverLocalPath { get; set; }
    public string? CoverCdnUrl { get; set; }
    public string? Description { get; set; }
    public string? Tags { get; set; }
    public int ChapterCount { get; set; } = 0;
    public string Source { get; set; } = null!;
    public string SourceId { get; set; } = null!;
    public DateTime FetchedAt { get; set; }
}
