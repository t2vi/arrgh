using System.ComponentModel.DataAnnotations;

namespace ArrghServer.Data.Models;

public class Chapter
{
    [Key] public string Id { get; set; } = null!;
    public string TitleId { get; set; } = null!;
    public string? ChapterTitle { get; set; }
    public double Number { get; set; }
    public double? Volume { get; set; }
    public string? LocalPath { get; set; }
    public int PageCount { get; set; } = 0;
    public bool Downloaded { get; set; } = false;
    public DateTime CreatedAt { get; set; }
    public bool IsNew { get; set; } = false;
    public string ChapterFormat { get; set; } = "pages";

    public Title Title { get; set; } = null!;
    public ICollection<ChapterSource> ChapterSources { get; set; } = [];
    public ICollection<ReadProgress> ReadProgresses { get; set; } = [];
}
