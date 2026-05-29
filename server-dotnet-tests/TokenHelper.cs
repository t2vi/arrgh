using System.Security.Claims;

namespace ArrghServer.Tests;

/// <summary>
/// Builds a ClaimsPrincipal for unit tests without going through JWT.
/// </summary>
public static class TokenHelper
{
    public static ClaimsPrincipal MakePrincipal(string userId, string? role, bool allowExplicit)
    {
        var claims = new List<Claim>
        {
            new(ClaimTypes.NameIdentifier, userId),
            new("allow_explicit", allowExplicit ? "true" : "false"),
        };
        if (role is not null)
            claims.Add(new Claim(ClaimTypes.Role, role));

        return new ClaimsPrincipal(new ClaimsIdentity(claims, "test"));
    }
}
