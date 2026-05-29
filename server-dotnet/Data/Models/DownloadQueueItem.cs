using System.ComponentModel.DataAnnotations;

namespace ArrghServer.Data.Models;

public class DownloadQueueItem
{
    [Key] public string Id { get; set; } = null!;
    public string ChapterId { get; set; } = null!;
    public string MangaTitle { get; set; } = null!;
    public double ChapterNum { get; set; }
    public string Status { get; set; } = "pending";
    public string? Error { get; set; }
    public DateTime CreatedAt { get; set; }
    public DateTime UpdatedAt { get; set; }
    public int PagesDownloaded { get; set; } = 0;
    public int PagesTotal { get; set; } = 0;
    public string? QueuedBy { get; set; }

    public Chapter Chapter { get; set; } = null!;
    public User? QueuedByUser { get; set; }
}
