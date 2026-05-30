namespace ArrghServer.Data.Models;

public class UserTitleSettings
{
    public string UserId { get; set; } = null!;
    public string TitleId { get; set; } = null!;
    public string? ReaderMode { get; set; }

    public User User { get; set; } = null!;
    public Title Title { get; set; } = null!;
}
