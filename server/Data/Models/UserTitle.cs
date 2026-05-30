using System.ComponentModel.DataAnnotations;

namespace ArrghServer.Data.Models;

public class UserTitle
{
    public string UserId { get; set; } = null!;
    public string TitleId { get; set; } = null!;
    public DateTime AddedAt { get; set; }

    public User User { get; set; } = null!;
    public Title Title { get; set; } = null!;
}
