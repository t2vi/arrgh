using System.ComponentModel.DataAnnotations;
using System.ComponentModel.DataAnnotations.Schema;

namespace ArrghServer.Data.Models;

public class Title
{
    [Key] public string Id { get; set; } = null!;
    public string TitleName { get; set; } = null!;
    public string? Description { get; set; }
    public string? CoverUrl { get; set; }
    public string Status { get; set; } = "unknown";
    public DateTime CreatedAt { get; set; }
    public DateTime UpdatedAt { get; set; }
    public string? Author { get; set; }
    public int? Year { get; set; }
    public string? Tags { get; set; }
    public string SyncStatus { get; set; } = "ready";
    public string ContentType { get; set; } = "manga";
    public bool? AutoDownload { get; set; }
    public string? ReaderMode { get; set; }
    public string? DownloadDir { get; set; }
    public bool IsExplicit { get; set; } = false;
    public string? MangaupdatesId { get; set; }
    public string? MetadataSource { get; set; }
    public string? MetadataSourceId { get; set; }
    public string? LocalPath { get; set; }

    public ICollection<Chapter> Chapters { get; set; } = [];
    public ICollection<UserTitle> UserTitles { get; set; } = [];
    public ICollection<TitleSource> TitleSources { get; set; } = [];
    public ICollection<TitleAlias> TitleAliases { get; set; } = [];
    public ICollection<SyncWarning> SyncWarnings { get; set; } = [];
    public ICollection<SyncLog> SyncLogs { get; set; } = [];
}
