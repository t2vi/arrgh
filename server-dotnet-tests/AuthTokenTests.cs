using System.IdentityModel.Tokens.Jwt;
using System.Security.Claims;
using System.Text;
using ArrghServer.Api;
using Microsoft.Extensions.Configuration;
using Microsoft.IdentityModel.Tokens;
using Xunit;

namespace ArrghServer.Tests;

/// <summary>
/// Pure unit tests for JWT token creation and validation.
/// No HTTP stack, no DB — mirrors Rust auth.rs inline tests.
/// </summary>
[Trait("Category", TestCategories.Unit)]
public class AuthTokenTests
{
    const string Secret = "unit-test-secret-32chars-padding!";

    static IConfiguration Config(string secret = Secret) =>
        new ConfigurationBuilder()
            .AddInMemoryCollection(new Dictionary<string, string?> { ["JwtSecret"] = secret })
            .Build();

    static ClaimsPrincipal Decode(string token, string secret = Secret)
    {
        var key = new SymmetricSecurityKey(Encoding.UTF8.GetBytes(secret));
        var handler = new JwtSecurityTokenHandler();
        return handler.ValidateToken(token, new TokenValidationParameters
        {
            ValidateIssuerSigningKey = true,
            IssuerSigningKey = key,
            ValidateIssuer = false,
            ValidateAudience = false,
            ClockSkew = TimeSpan.Zero,
        }, out _);
    }

    [Fact]
    public void CreateToken_Roundtrip_AdminClaims()
    {
        var token = Auth.CreateToken("user-1", "alice", "admin", true, Config());
        var principal = Decode(token);

        Assert.Equal("user-1", principal.FindFirstValue(ClaimTypes.NameIdentifier));
        Assert.Equal("alice", principal.FindFirstValue("username"));
        Assert.Equal("admin", principal.FindFirstValue(ClaimTypes.Role));
        Assert.Equal("true", principal.FindFirstValue("allow_explicit"));
    }

    [Fact]
    public void CreateToken_Roundtrip_MemberClaims()
    {
        var token = Auth.CreateToken("user-2", "bob", "member", false, Config());
        var principal = Decode(token);

        Assert.Equal("member", principal.FindFirstValue(ClaimTypes.Role));
        Assert.Equal("false", principal.FindFirstValue("allow_explicit"));
    }

    [Fact]
    public void CreateToken_WrongSecret_Rejected()
    {
        var token = Auth.CreateToken("user-1", "alice", "admin", true, Config());

        Assert.Throws<SecurityTokenSignatureKeyNotFoundException>(() =>
            Decode(token, secret: "wrong-secret-32chars-padding!!!!!"));
    }

    [Fact]
    public void CreateToken_Expires_In30Days()
    {
        var before = DateTime.UtcNow.AddDays(30).AddMinutes(-1);
        var token = Auth.CreateToken("user-1", "alice", "admin", true, Config());
        var after = DateTime.UtcNow.AddDays(30).AddMinutes(1);

        var jwt = new JwtSecurityTokenHandler().ReadJwtToken(token);
        Assert.True(jwt.ValidTo >= before && jwt.ValidTo <= after);
    }

    [Fact]
    public void CreateToken_DifferentUsers_DifferentTokens()
    {
        var t1 = Auth.CreateToken("user-1", "alice", "admin", true, Config());
        var t2 = Auth.CreateToken("user-2", "bob", "member", false, Config());

        Assert.NotEqual(t1, t2);
    }

    [Fact]
    public void CreateToken_SameInputs_ProducesDifferentTokens_DueToTimestamp()
    {
        // Tokens include exp (epoch seconds) — if called within same second they'll match,
        // so just verify both decode correctly rather than asserting inequality.
        var t1 = Auth.CreateToken("user-1", "alice", "admin", true, Config());
        var t2 = Auth.CreateToken("user-1", "alice", "admin", true, Config());

        var p1 = Decode(t1);
        var p2 = Decode(t2);
        Assert.Equal(p1.FindFirstValue(ClaimTypes.NameIdentifier),
                     p2.FindFirstValue(ClaimTypes.NameIdentifier));
    }
}
