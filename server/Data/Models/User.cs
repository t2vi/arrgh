using System.ComponentModel.DataAnnotations;

namespace ArrghServer.Data.Models;

public class User
{
    [Key] public string Id { get; set; } = null!;
    public string Username { get; set; } = null!;
    public string PasswordHash { get; set; } = null!;
    public DateTime CreatedAt { get; set; } = DateTime.UtcNow;
    public string Role { get; set; } = "admin";
    public bool AllowExplicit { get; set; } = false;

    public ICollection<UserTitle> UserTitles { get; set; } = [];
    public ICollection<ReadProgress> ReadProgresses { get; set; } = [];
    public ICollection<DownloadQueueItem> QueueItems { get; set; } = [];
}
