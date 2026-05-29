using ArrghServer.Data.Models;
using Microsoft.EntityFrameworkCore;

namespace ArrghServer.Data;

public class AppDbContext(DbContextOptions<AppDbContext> options) : DbContext(options)
{
    public DbSet<Title> Titles => Set<Title>();
    public DbSet<Chapter> Chapters => Set<Chapter>();
    public DbSet<User> Users => Set<User>();
    public DbSet<UserTitle> UserTitles => Set<UserTitle>();
    public DbSet<UserTitleSettings> UserTitleSettings => Set<UserTitleSettings>();
    public DbSet<ReadProgress> ReadProgresses => Set<ReadProgress>();
    public DbSet<DownloadQueueItem> DownloadQueue => Set<DownloadQueueItem>();
    public DbSet<ExternalSource> ExternalSources => Set<ExternalSource>();
    public DbSet<TitleMeta> TitleMeta => Set<TitleMeta>();
    public DbSet<TitleSource> TitleSources => Set<TitleSource>();
    public DbSet<ChapterSource> ChapterSources => Set<ChapterSource>();
    public DbSet<TitleAlias> TitleAliases => Set<TitleAlias>();
    public DbSet<SyncWarning> SyncWarnings => Set<SyncWarning>();
    public DbSet<SyncLog> SyncLogs => Set<SyncLog>();
    public DbSet<ServerSetting> ServerSettings => Set<ServerSetting>();

    protected override void OnModelCreating(ModelBuilder b)
    {
        // titles table — column name differs from property name
        b.Entity<Title>(e =>
        {
            e.ToTable("titles");
            e.Property(t => t.Id).HasColumnName("id");
            e.Property(t => t.TitleName).HasColumnName("title");
            e.Property(t => t.Description).HasColumnName("description");
            e.Property(t => t.CoverUrl).HasColumnName("cover_url");
            e.Property(t => t.Status).HasColumnName("status");
            e.Property(t => t.CreatedAt).HasColumnName("created_at");
            e.Property(t => t.UpdatedAt).HasColumnName("updated_at");
            e.Property(t => t.Author).HasColumnName("author");
            e.Property(t => t.Year).HasColumnName("year");
            e.Property(t => t.Tags).HasColumnName("tags");
            e.Property(t => t.SyncStatus).HasColumnName("sync_status");
            e.Property(t => t.ContentType).HasColumnName("content_type");
            e.Property(t => t.AutoDownload).HasColumnName("auto_download");
            e.Property(t => t.ReaderMode).HasColumnName("reader_mode");
            e.Property(t => t.DownloadDir).HasColumnName("download_dir");
            e.Property(t => t.IsExplicit).HasColumnName("is_explicit");
            e.Property(t => t.MangaupdatesId).HasColumnName("mangaupdates_id");
            e.Property(t => t.MetadataSource).HasColumnName("metadata_source");
            e.Property(t => t.MetadataSourceId).HasColumnName("metadata_source_id");
            e.Property(t => t.LocalPath).HasColumnName("local_path");
            // partial unique index: WHERE mangaupdates_id IS NOT NULL
            e.HasIndex(t => t.MangaupdatesId)
                .IsUnique()
                .HasFilter("mangaupdates_id IS NOT NULL");
        });

        b.Entity<Chapter>(e =>
        {
            e.ToTable("chapters");
            e.Property(c => c.Id).HasColumnName("id");
            e.Property(c => c.TitleId).HasColumnName("title_id");
            e.Property(c => c.ChapterTitle).HasColumnName("title");
            e.Property(c => c.Number).HasColumnName("number");
            e.Property(c => c.Volume).HasColumnName("volume");
            e.Property(c => c.LocalPath).HasColumnName("local_path");
            e.Property(c => c.PageCount).HasColumnName("page_count");
            e.Property(c => c.Downloaded).HasColumnName("downloaded");
            e.Property(c => c.CreatedAt).HasColumnName("created_at");
            e.Property(c => c.IsNew).HasColumnName("is_new");
            e.Property(c => c.ChapterFormat).HasColumnName("chapter_format");
            e.HasIndex(c => c.TitleId).HasDatabaseName("idx_chapters_title_id");
            e.HasOne(c => c.Title).WithMany(t => t.Chapters).HasForeignKey(c => c.TitleId).OnDelete(DeleteBehavior.Cascade);
        });

        b.Entity<User>(e =>
        {
            e.ToTable("users");
            e.Property(u => u.Id).HasColumnName("id");
            e.Property(u => u.Username).HasColumnName("username");
            e.Property(u => u.PasswordHash).HasColumnName("password_hash");
            e.Property(u => u.CreatedAt).HasColumnName("created_at");
            e.Property(u => u.Role).HasColumnName("role");
            e.Property(u => u.AllowExplicit).HasColumnName("allow_explicit");
            e.HasIndex(u => u.Username).IsUnique();
        });

        b.Entity<UserTitle>(e =>
        {
            e.ToTable("user_titles");
            e.HasKey(ut => new { ut.UserId, ut.TitleId });
            e.Property(ut => ut.UserId).HasColumnName("user_id");
            e.Property(ut => ut.TitleId).HasColumnName("title_id");
            e.Property(ut => ut.AddedAt).HasColumnName("added_at");
            e.HasIndex(ut => ut.UserId).HasDatabaseName("idx_user_titles_user");
            e.HasOne(ut => ut.User).WithMany(u => u.UserTitles).HasForeignKey(ut => ut.UserId).OnDelete(DeleteBehavior.Cascade);
            e.HasOne(ut => ut.Title).WithMany(t => t.UserTitles).HasForeignKey(ut => ut.TitleId).OnDelete(DeleteBehavior.Cascade);
        });

        b.Entity<UserTitleSettings>(e =>
        {
            e.ToTable("user_title_settings");
            e.HasKey(s => new { s.UserId, s.TitleId });
            e.Property(s => s.UserId).HasColumnName("user_id");
            e.Property(s => s.TitleId).HasColumnName("title_id");
            e.Property(s => s.ReaderMode).HasColumnName("reader_mode");
            e.HasOne(s => s.User).WithMany().HasForeignKey(s => s.UserId).OnDelete(DeleteBehavior.Cascade);
            e.HasOne(s => s.Title).WithMany().HasForeignKey(s => s.TitleId).OnDelete(DeleteBehavior.Cascade);
        });

        b.Entity<ReadProgress>(e =>
        {
            e.ToTable("read_progress");
            e.Property(r => r.Id).HasColumnName("id");
            e.Property(r => r.UserId).HasColumnName("user_id");
            e.Property(r => r.ChapterId).HasColumnName("chapter_id");
            e.Property(r => r.CurrentPage).HasColumnName("current_page");
            e.Property(r => r.Completed).HasColumnName("completed");
            e.Property(r => r.UpdatedAt).HasColumnName("updated_at");
            e.HasIndex(r => new { r.UserId, r.ChapterId }).IsUnique();
            e.HasIndex(r => r.UserId).HasDatabaseName("idx_read_progress_user");
            e.HasIndex(r => r.ChapterId).HasDatabaseName("idx_read_progress_chapter");
            e.HasOne(r => r.User).WithMany(u => u.ReadProgresses).HasForeignKey(r => r.UserId).OnDelete(DeleteBehavior.Cascade);
            e.HasOne(r => r.Chapter).WithMany(c => c.ReadProgresses).HasForeignKey(r => r.ChapterId).OnDelete(DeleteBehavior.Cascade);
        });

        b.Entity<DownloadQueueItem>(e =>
        {
            e.ToTable("download_queue");
            e.Property(q => q.Id).HasColumnName("id");
            e.Property(q => q.ChapterId).HasColumnName("chapter_id");
            e.Property(q => q.MangaTitle).HasColumnName("manga_title");
            e.Property(q => q.ChapterNum).HasColumnName("chapter_num");
            e.Property(q => q.Status).HasColumnName("status");
            e.Property(q => q.Error).HasColumnName("error");
            e.Property(q => q.CreatedAt).HasColumnName("created_at");
            e.Property(q => q.UpdatedAt).HasColumnName("updated_at");
            e.Property(q => q.PagesDownloaded).HasColumnName("pages_downloaded");
            e.Property(q => q.PagesTotal).HasColumnName("pages_total");
            e.Property(q => q.QueuedBy).HasColumnName("queued_by");
            e.HasIndex(q => q.ChapterId).IsUnique();
            e.HasIndex(q => new { q.Status, q.CreatedAt }).HasDatabaseName("idx_queue_status");
            e.HasOne(q => q.Chapter).WithMany().HasForeignKey(q => q.ChapterId).OnDelete(DeleteBehavior.Cascade);
            e.HasOne(q => q.QueuedByUser).WithMany(u => u.QueueItems).HasForeignKey(q => q.QueuedBy).OnDelete(DeleteBehavior.SetNull);
        });

        b.Entity<ExternalSource>(e =>
        {
            e.ToTable("external_sources");
            e.Property(s => s.Id).HasColumnName("id");
            e.Property(s => s.Name).HasColumnName("name");
            e.Property(s => s.BaseUrl).HasColumnName("base_url");
            e.Property(s => s.ApiKey).HasColumnName("api_key");
            e.Property(s => s.ContentTypes).HasColumnName("content_types");
            e.Property(s => s.Enabled).HasColumnName("enabled");
            e.Property(s => s.CreatedAt).HasColumnName("created_at");
            e.Property(s => s.IsCommunity).HasColumnName("is_community");
            e.Property(s => s.Priority).HasColumnName("priority");
            e.Property(s => s.SourceKey).HasColumnName("source_key");
            e.Property(s => s.DefaultExplicit).HasColumnName("default_explicit");
        });

        b.Entity<TitleMeta>(e =>
        {
            e.ToTable("title_meta");
            e.Property(m => m.TitleKey).HasColumnName("title_key");
            e.Property(m => m.CoverLocalPath).HasColumnName("cover_local_path");
            e.Property(m => m.CoverCdnUrl).HasColumnName("cover_cdn_url");
            e.Property(m => m.Description).HasColumnName("description");
            e.Property(m => m.Tags).HasColumnName("tags");
            e.Property(m => m.ChapterCount).HasColumnName("chapter_count");
            e.Property(m => m.Source).HasColumnName("source");
            e.Property(m => m.SourceId).HasColumnName("source_id");
            e.Property(m => m.FetchedAt).HasColumnName("fetched_at");
        });

        b.Entity<TitleSource>(e =>
        {
            e.ToTable("title_sources");
            e.Property(s => s.Id).HasColumnName("id");
            e.Property(s => s.TitleId).HasColumnName("title_id");
            e.Property(s => s.Source).HasColumnName("source");
            e.Property(s => s.SourceId).HasColumnName("source_id");
            e.Property(s => s.DiscoveredAt).HasColumnName("discovered_at");
            e.HasIndex(s => new { s.TitleId, s.Source }).IsUnique();
            e.HasOne(s => s.Title).WithMany(t => t.TitleSources).HasForeignKey(s => s.TitleId).OnDelete(DeleteBehavior.Cascade);
        });

        b.Entity<ChapterSource>(e =>
        {
            e.ToTable("chapter_sources");
            e.Property(s => s.Id).HasColumnName("id");
            e.Property(s => s.ChapterId).HasColumnName("chapter_id");
            e.Property(s => s.Source).HasColumnName("source");
            e.Property(s => s.SourceId).HasColumnName("source_id");
            e.HasIndex(s => new { s.ChapterId, s.Source }).IsUnique();
            e.HasIndex(s => s.ChapterId).HasDatabaseName("idx_chapter_sources_chapter_id");
            e.HasOne(s => s.Chapter).WithMany(c => c.ChapterSources).HasForeignKey(s => s.ChapterId).OnDelete(DeleteBehavior.Cascade);
        });

        b.Entity<TitleAlias>(e =>
        {
            e.ToTable("title_aliases");
            e.Property(a => a.Id).HasColumnName("id");
            e.Property(a => a.TitleId).HasColumnName("title_id");
            e.Property(a => a.Alias).HasColumnName("alias");
            e.HasIndex(a => a.TitleId).HasDatabaseName("idx_title_aliases_title_id");
            e.HasOne(a => a.Title).WithMany(t => t.TitleAliases).HasForeignKey(a => a.TitleId).OnDelete(DeleteBehavior.Cascade);
        });

        b.Entity<SyncWarning>(e =>
        {
            e.ToTable("sync_warnings");
            e.Property(w => w.Id).HasColumnName("id");
            e.Property(w => w.TitleId).HasColumnName("title_id");
            e.Property(w => w.PluginId).HasColumnName("plugin_id");
            e.Property(w => w.Message).HasColumnName("message");
            e.Property(w => w.CreatedAt).HasColumnName("created_at");
            e.HasIndex(w => new { w.TitleId, w.PluginId }).IsUnique();
            e.HasIndex(w => w.TitleId).HasDatabaseName("idx_sync_warnings_title_id");
            e.HasOne(w => w.Title).WithMany(t => t.SyncWarnings).HasForeignKey(w => w.TitleId).OnDelete(DeleteBehavior.Cascade);
        });

        b.Entity<SyncLog>(e =>
        {
            e.ToTable("sync_log");
            e.Property(l => l.Id).HasColumnName("id");
            e.Property(l => l.TitleId).HasColumnName("title_id");
            e.Property(l => l.Message).HasColumnName("message");
            e.Property(l => l.CreatedAt).HasColumnName("created_at");
            e.HasIndex(l => new { l.TitleId, l.CreatedAt }).HasDatabaseName("idx_sync_log_title_id");
            e.HasOne(l => l.Title).WithMany(t => t.SyncLogs).HasForeignKey(l => l.TitleId).OnDelete(DeleteBehavior.Cascade);
        });

        b.Entity<ServerSetting>(e =>
        {
            e.ToTable("server_settings");
            e.Property(s => s.Key).HasColumnName("key");
            e.Property(s => s.Value).HasColumnName("value");
        });

    }
}
