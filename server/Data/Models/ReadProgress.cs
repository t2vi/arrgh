using System.ComponentModel.DataAnnotations;

namespace ArrghServer.Data.Models;

public class ReadProgress
{
    [Key] public string Id { get; set; } = null!;
    public string UserId { get; set; } = null!;
    public string ChapterId { get; set; } = null!;
    public int CurrentPage { get; set; } = 0;
    public bool Completed { get; set; } = false;
    public DateTime UpdatedAt { get; set; }

    public User User { get; set; } = null!;
    public Chapter Chapter { get; set; } = null!;
}
