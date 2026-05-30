using System.ComponentModel.DataAnnotations;

namespace ArrghServer.Data.Models;

public class ServerSetting
{
    [Key] public string Key { get; set; } = null!;
    public string Value { get; set; } = null!;
}
