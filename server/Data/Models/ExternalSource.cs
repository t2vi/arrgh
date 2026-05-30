using System.ComponentModel.DataAnnotations;

namespace ArrghServer.Data.Models;

public class ExternalSource
{
    [Key] public string Id { get; set; } = null!;
    public string Name { get; set; } = null!;
    public string BaseUrl { get; set; } = null!;
    public string? ApiKey { get; set; }
    public string ContentTypes { get; set; } = "manga";
    public bool Enabled { get; set; } = true;
    public DateTime CreatedAt { get; set; }
    public bool IsCommunity { get; set; } = false;
    public int Priority { get; set; } = 100;
    public string? SourceKey { get; set; }
    public bool DefaultExplicit { get; set; } = false;
}
