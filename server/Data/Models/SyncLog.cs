using System.ComponentModel.DataAnnotations;

namespace ArrghServer.Data.Models;

public class SyncLog
{
    [Key] public string Id { get; set; } = null!;
    public string TitleId { get; set; } = null!;
    public string Message { get; set; } = null!;
    public DateTime CreatedAt { get; set; }

    public Title Title { get; set; } = null!;
}
