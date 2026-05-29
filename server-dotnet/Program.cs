using System.Text;
using System.Text.Json;
using ArrghServer.Api;
using ArrghServer.Data;
using ArrghServer.Services;
using Microsoft.AspNetCore.Authentication.JwtBearer;
using Microsoft.EntityFrameworkCore;
using Microsoft.IdentityModel.Tokens;

var builder = WebApplication.CreateBuilder(args);

var dbPath = builder.Configuration["DatabasePath"] ?? "arrgh.db";

// Logs
var logService = new LogService();
builder.Services.AddSingleton(logService);
builder.Logging.AddProvider(new RingBufferLoggerProvider(logService));

// Version / update checker
builder.Services.AddSingleton<UpdateCache>();
builder.Services.AddSingleton<PageCacheService>();
builder.Services.AddSingleton<TrendingCacheService>();
builder.Services.AddScoped<MangaUpdatesService>();
builder.Services.AddHttpClient();
builder.Services.AddHostedService<UpdateCheckerService>();

builder.Services.AddDbContext<AppDbContext>(opt =>
    opt.UseSqlite($"Data Source={dbPath}"));

// JwtSecret read at service-configuration time via IConfiguration so tests can PostConfigure it
builder.Services.AddAuthentication(JwtBearerDefaults.AuthenticationScheme)
    .AddJwtBearer(opt =>
    {
        var secret = builder.Configuration["JwtSecret"]
            ?? throw new InvalidOperationException("JwtSecret must be set in config or environment");
        opt.TokenValidationParameters = new TokenValidationParameters
        {
            ValidateIssuerSigningKey = true,
            IssuerSigningKey = new SymmetricSecurityKey(Encoding.UTF8.GetBytes(secret)),
            ValidateIssuer = false,
            ValidateAudience = false,
        };
    });

builder.Services.AddAuthorization();

builder.Services.ConfigureHttpJsonOptions(o =>
{
    o.SerializerOptions.PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower;
    o.SerializerOptions.PropertyNameCaseInsensitive = true;
});

var app = builder.Build();

// Apply EF migrations on startup
using (var scope = app.Services.CreateScope())
{
    var db = scope.ServiceProvider.GetRequiredService<AppDbContext>();
    db.Database.Migrate();
}

app.UseAuthentication();
app.UseAuthorization();

// Route groups — one per feature module, all under /api
var api = app.MapGroup("/api");

api.MapGroup("/auth").MapAuthRoutes(app.Configuration);
api.MapGroup("/users").MapUserRoutes();
api.MapGroup("/titles").MapTitlesRoutes();
api.MapGroup("/chapters").MapChaptersRoutes();
api.MapGroup("/queue").MapQueueRoutes();
api.MapGroup("/progress").MapProgressRoutes();
api.MapGroup("/settings").MapSettingsRoutes();
api.MapGroup("/sources").MapSourcesRoutes();
api.MapGroup("/plugins").MapPluginsRoutes();
api.MapGroup("/media").MapMediaRoutes();
api.MapGroup("/discover").MapDiscoverRoutes();
api.MapGroup("/logs").MapLogsRoutes();
api.MapGroup("/version").MapVersionRoutes();

app.Run();

public partial class Program {}
