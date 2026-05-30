using System.IdentityModel.Tokens.Jwt;
using System.Security.Claims;
using System.Text;
using ArrghServer.Data;
using ArrghServer.Data.Models;
using Microsoft.AspNetCore.Authorization;
using Microsoft.EntityFrameworkCore;
using Microsoft.IdentityModel.Tokens;

namespace ArrghServer.Api;

public static class Auth
{
    public static RouteGroupBuilder MapAuthRoutes(this RouteGroupBuilder group, IConfiguration config)
    {
        // Public
        group.MapGet("/status", Status).AllowAnonymous().WithSummary("Server status — returns registered/unregistered");
        group.MapPost("/register", Register).AllowAnonymous().WithSummary("Register first user (auto-admin) or additional users (admin-only after first)");
        group.MapPost("/login", Login).AllowAnonymous().WithSummary("Login and receive JWT");

        // Protected — current user
        group.MapGet("/me", Me).RequireAuthorization().WithSummary("Get current user profile");
        group.MapPatch("/me", PatchMe).RequireAuthorization().WithSummary("Update current user profile");

        return group;
    }

    public static RouteGroupBuilder MapUserRoutes(this RouteGroupBuilder group)
    {
        group.MapGet("/", ListUsers).RequireAuthorization().WithSummary("List all users (admin only)");
        group.MapPost("/", CreateUser).RequireAuthorization().WithSummary("Create a new user (admin only)");
        group.MapPatch("/{id}", PatchUser).RequireAuthorization().WithSummary("Update user (admin only)");
        group.MapDelete("/{id}", DeleteUser).RequireAuthorization().WithSummary("Delete user (admin only)");
        return group;
    }

    // -------------------------------------------------------------------------

    static async Task<IResult> Status(AppDbContext db)
    {
        var count = await db.Users.CountAsync();
        return Results.Ok(new { needs_setup = count == 0 });
    }

    static async Task<IResult> Register(RegisterBody body, AppDbContext db, IConfiguration config)
    {
        var count = await db.Users.CountAsync();
        if (count > 0) return Results.Forbid();

        if (string.IsNullOrWhiteSpace(body.Username) || body.Password.Length < 6)
            return Results.UnprocessableEntity();

        var id = Guid.NewGuid().ToString();
        var hash = BCrypt.Net.BCrypt.HashPassword(body.Password);
        var username = body.Username.Trim();

        db.Users.Add(new User
        {
            Id = id,
            Username = username,
            PasswordHash = hash,
            Role = "admin",
            AllowExplicit = true,
            CreatedAt = DateTime.UtcNow,
        });
        await db.SaveChangesAsync();

        var token = CreateToken(id, username, "admin", true, config);
        return Results.Ok(new AuthResponse(token, username, id, "admin", true));
    }

    static async Task<IResult> Login(LoginBody body, AppDbContext db, IConfiguration config)
    {
        var user = await db.Users.FirstOrDefaultAsync(u => u.Username == body.Username);
        if (user is null) return Results.Unauthorized();

        if (!BCrypt.Net.BCrypt.Verify(body.Password, user.PasswordHash))
            return Results.Unauthorized();

        var token = CreateToken(user.Id, user.Username, user.Role, user.AllowExplicit, config);
        return Results.Ok(new AuthResponse(token, user.Username, user.Id, user.Role, user.AllowExplicit));
    }

    static async Task<IResult> Me(ClaimsPrincipal principal, AppDbContext db)
    {
        var userId = principal.FindFirstValue(ClaimTypes.NameIdentifier)!;
        var user = await db.Users.FindAsync(userId);
        if (user is null) return Results.NotFound();

        return Results.Ok(new MeResponse(user.Id, user.Username, user.Role, user.AllowExplicit));
    }

    static async Task<IResult> PatchMe(PatchMeBody body, ClaimsPrincipal principal, AppDbContext db)
    {
        if (body.Password.Length < 6) return Results.UnprocessableEntity();

        var userId = principal.FindFirstValue(ClaimTypes.NameIdentifier)!;
        var user = await db.Users.FindAsync(userId);
        if (user is null) return Results.NotFound();

        user.PasswordHash = BCrypt.Net.BCrypt.HashPassword(body.Password);
        await db.SaveChangesAsync();
        return Results.NoContent();
    }

    static async Task<IResult> ListUsers(ClaimsPrincipal principal, AppDbContext db)
    {
        if (!IsAdmin(principal)) return Results.Forbid();

        var users = await db.Users
            .OrderBy(u => u.CreatedAt)
            .Select(u => new UserListItem(u.Id, u.Username, u.Role, u.AllowExplicit, u.CreatedAt.ToString("o")))
            .ToListAsync();

        return Results.Ok(users);
    }

    static async Task<IResult> CreateUser(CreateUserBody body, ClaimsPrincipal principal, AppDbContext db)
    {
        if (!IsAdmin(principal)) return Results.Forbid();
        if (string.IsNullOrWhiteSpace(body.Username) || body.Password.Length < 6)
            return Results.UnprocessableEntity();

        var exists = await db.Users.AnyAsync(u => u.Username == body.Username);
        if (exists) return Results.Conflict();

        db.Users.Add(new User
        {
            Id = Guid.NewGuid().ToString(),
            Username = body.Username.Trim(),
            PasswordHash = BCrypt.Net.BCrypt.HashPassword(body.Password),
            Role = "member",
            AllowExplicit = false,
            CreatedAt = DateTime.UtcNow,
        });
        await db.SaveChangesAsync();
        return Results.StatusCode(201);
    }

    static async Task<IResult> PatchUser(string id, PatchUserBody body, ClaimsPrincipal principal, AppDbContext db)
    {
        if (!IsAdmin(principal)) return Results.Forbid();

        var user = await db.Users.FindAsync(id);
        if (user is null) return Results.NotFound();

        if (body.Role is not null)
        {
            if (body.Role != "admin" && body.Role != "member")
                return Results.UnprocessableEntity();
            user.Role = body.Role;
        }

        if (body.AllowExplicit is not null)
            user.AllowExplicit = body.AllowExplicit.Value;

        if (body.Password is not null)
        {
            if (body.Password.Length < 6) return Results.UnprocessableEntity();
            user.PasswordHash = BCrypt.Net.BCrypt.HashPassword(body.Password);
        }

        await db.SaveChangesAsync();
        return Results.NoContent();
    }

    static async Task<IResult> DeleteUser(string id, ClaimsPrincipal principal, AppDbContext db)
    {
        if (!IsAdmin(principal)) return Results.Forbid();

        var callerId = principal.FindFirstValue(ClaimTypes.NameIdentifier)!;
        if (callerId == id) return Results.Forbid();

        var user = await db.Users.FindAsync(id);
        if (user is null) return Results.NotFound();

        db.Users.Remove(user);
        await db.SaveChangesAsync();
        return Results.NoContent();
    }

    // -------------------------------------------------------------------------

    static bool IsAdmin(ClaimsPrincipal p) =>
        p.FindFirstValue(ClaimTypes.Role) == "admin";

    public static string CreateToken(string userId, string username, string role, bool allowExplicit, IConfiguration config)
    {
        var secret = config["JwtSecret"]!;
        var key = new SymmetricSecurityKey(Encoding.UTF8.GetBytes(secret));
        var creds = new SigningCredentials(key, SecurityAlgorithms.HmacSha256);

        var claims = new[]
        {
            new Claim(ClaimTypes.NameIdentifier, userId),
            new Claim("username", username),
            new Claim(ClaimTypes.Role, role),
            new Claim("allow_explicit", allowExplicit ? "true" : "false"),
        };

        var token = new JwtSecurityToken(
            claims: claims,
            expires: DateTime.UtcNow.AddDays(30),
            signingCredentials: creds);

        return new JwtSecurityTokenHandler().WriteToken(token);
    }
}

// -------------------------------------------------------------------------
// Request / response records

record RegisterBody(string Username, string Password);
record LoginBody(string Username, string Password);
record CreateUserBody(string Username, string Password);
record PatchMeBody(string Password);
record PatchUserBody(string? Role, bool? AllowExplicit, string? Password);

record AuthResponse(string Token, string Username, string UserId, string Role, bool AllowExplicit);
record MeResponse(string Id, string Username, string Role, bool AllowExplicit);
record UserListItem(string Id, string Username, string Role, bool AllowExplicit, string CreatedAt);
