using System.ComponentModel.DataAnnotations;

namespace ArrghServer.Data.Models;

public class TitleSource
{
    [Key] public string Id { get; set; } = null!;
    public string TitleId { get; set; } = null!;
    public string Source { get; set; } = null!;
    public string SourceId { get; set; } = null!;
    public DateTime DiscoveredAt { get; set; }

    public Title Title { get; set; } = null!;
}
