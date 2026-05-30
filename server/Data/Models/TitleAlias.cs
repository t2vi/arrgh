using System.ComponentModel.DataAnnotations;

namespace ArrghServer.Data.Models;

public class TitleAlias
{
    [Key] public string Id { get; set; } = null!;
    public string TitleId { get; set; } = null!;
    public string Alias { get; set; } = null!;

    public Title Title { get; set; } = null!;
}
