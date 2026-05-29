using System.Text;
using System.Text.Json;
using ArrghServer.Api;
using ArrghServer.Data;
using ArrghServer.Services;
using Microsoft.AspNetCore.Authentication.JwtBearer;
using Microsoft.AspNetCore.OpenApi;
using Microsoft.EntityFrameworkCore;
using Microsoft.IdentityModel.Tokens;
using Microsoft.OpenApi;
using Scalar.AspNetCore;

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
builder.Services.AddScoped<AniListService>();
builder.Services.AddScoped<MangaDexMetaService>();
builder.Services.AddScoped<NovelUpdatesService>();
builder.Services.AddScoped<EHentaiService>();
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

// OpenAPI + Scalar
builder.Services.AddOpenApi("v1", options =>
{
    options.AddDocumentTransformer((doc, ctx, ct) =>
    {
        doc.Info = new()
        {
            Title = "*ARRgh API",
            Version = "v1",
            Description = "Self-hosted manga / manhwa / manhua / novel manager, downloader, and reader.",
        };
        return Task.CompletedTask;
    });

    // JWT Bearer security scheme
    options.AddDocumentTransformer<BearerSecurityTransformer>();
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

// OpenAPI JSON + Scalar UI — dev only
if (app.Environment.IsDevelopment())
{
    app.MapOpenApi("/api/openapi.json");
    app.MapScalarApiReference("/api/docs", opt =>
    {
        opt.Title = "*ARRgh API";
        opt.OpenApiRoutePattern = "/api/openapi.json";
        opt.Authentication = new ScalarAuthenticationOptions
        {
            PreferredSecuritySchemes = ["Bearer"],
        };
    });
}

// Route groups — one per feature module, all under /api
var api = app.MapGroup("/api");

api.MapGroup("/auth").WithTags("Auth").MapAuthRoutes(app.Configuration);
api.MapGroup("/users").WithTags("Users").MapUserRoutes();
api.MapGroup("/titles").WithTags("Titles").MapTitlesRoutes();
api.MapGroup("/chapters").WithTags("Chapters").MapChaptersRoutes();
api.MapGroup("/queue").WithTags("Queue").MapQueueRoutes();
api.MapGroup("/progress").WithTags("Progress").MapProgressRoutes();
api.MapGroup("/settings").WithTags("Settings").MapSettingsRoutes();
api.MapGroup("/sources").WithTags("Sources").MapSourcesRoutes();
api.MapGroup("/plugins").WithTags("Plugins").MapPluginsRoutes();
api.MapGroup("/media").WithTags("Media").MapMediaRoutes();
api.MapGroup("/discover").WithTags("Discover").MapDiscoverRoutes();
api.MapGroup("/logs").WithTags("Logs").MapLogsRoutes();
api.MapGroup("/version").WithTags("Version").MapVersionRoutes();

app.Run();

public partial class Program {}

// Adds JWT Bearer security scheme to the OpenAPI document components.
// Scalar uses PreferredSecuritySchemes (below) to apply it globally in the UI.
file sealed class BearerSecurityTransformer : IOpenApiDocumentTransformer
{
    public Task TransformAsync(OpenApiDocument document, OpenApiDocumentTransformerContext ctx, CancellationToken ct)
    {
        document.Components ??= new OpenApiComponents();
        document.Components.SecuritySchemes["Bearer"] = new OpenApiSecurityScheme
        {
            Type = SecuritySchemeType.Http,
            Scheme = "bearer",
            BearerFormat = "JWT",
            Description = "Paste your JWT (without 'Bearer ' prefix). Obtain via POST /api/auth/login.",
        };
        return Task.CompletedTask;
    }
}
