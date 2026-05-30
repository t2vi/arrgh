using System.ComponentModel.DataAnnotations;

namespace ArrghServer.Data.Models;

public class ChapterSource
{
    [Key] public string Id { get; set; } = null!;
    public string ChapterId { get; set; } = null!;
    public string Source { get; set; } = null!;
    public string SourceId { get; set; } = null!;

    public Chapter Chapter { get; set; } = null!;
}
