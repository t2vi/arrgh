using System;
using Microsoft.EntityFrameworkCore.Migrations;

#nullable disable

namespace ArrghServer.Migrations
{
    /// <inheritdoc />
    public partial class InitialSchema : Migration
    {
        /// <inheritdoc />
        protected override void Up(MigrationBuilder migrationBuilder)
        {
            migrationBuilder.CreateTable(
                name: "external_sources",
                columns: table => new
                {
                    id = table.Column<string>(type: "TEXT", nullable: false),
                    name = table.Column<string>(type: "TEXT", nullable: false),
                    base_url = table.Column<string>(type: "TEXT", nullable: false),
                    api_key = table.Column<string>(type: "TEXT", nullable: true),
                    content_types = table.Column<string>(type: "TEXT", nullable: false),
                    enabled = table.Column<bool>(type: "INTEGER", nullable: false),
                    created_at = table.Column<DateTime>(type: "TEXT", nullable: false),
                    is_community = table.Column<bool>(type: "INTEGER", nullable: false),
                    priority = table.Column<int>(type: "INTEGER", nullable: false),
                    source_key = table.Column<string>(type: "TEXT", nullable: true),
                    default_explicit = table.Column<bool>(type: "INTEGER", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("PK_external_sources", x => x.id);
                });

            migrationBuilder.CreateTable(
                name: "server_settings",
                columns: table => new
                {
                    key = table.Column<string>(type: "TEXT", nullable: false),
                    value = table.Column<string>(type: "TEXT", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("PK_server_settings", x => x.key);
                });

            migrationBuilder.CreateTable(
                name: "title_meta",
                columns: table => new
                {
                    title_key = table.Column<string>(type: "TEXT", nullable: false),
                    cover_local_path = table.Column<string>(type: "TEXT", nullable: true),
                    cover_cdn_url = table.Column<string>(type: "TEXT", nullable: true),
                    description = table.Column<string>(type: "TEXT", nullable: true),
                    tags = table.Column<string>(type: "TEXT", nullable: true),
                    chapter_count = table.Column<int>(type: "INTEGER", nullable: false),
                    source = table.Column<string>(type: "TEXT", nullable: false),
                    source_id = table.Column<string>(type: "TEXT", nullable: false),
                    fetched_at = table.Column<DateTime>(type: "TEXT", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("PK_title_meta", x => x.title_key);
                });

            migrationBuilder.CreateTable(
                name: "titles",
                columns: table => new
                {
                    id = table.Column<string>(type: "TEXT", nullable: false),
                    title = table.Column<string>(type: "TEXT", nullable: false),
                    description = table.Column<string>(type: "TEXT", nullable: true),
                    cover_url = table.Column<string>(type: "TEXT", nullable: true),
                    status = table.Column<string>(type: "TEXT", nullable: false),
                    created_at = table.Column<DateTime>(type: "TEXT", nullable: false),
                    updated_at = table.Column<DateTime>(type: "TEXT", nullable: false),
                    author = table.Column<string>(type: "TEXT", nullable: true),
                    year = table.Column<int>(type: "INTEGER", nullable: true),
                    tags = table.Column<string>(type: "TEXT", nullable: true),
                    sync_status = table.Column<string>(type: "TEXT", nullable: false),
                    content_type = table.Column<string>(type: "TEXT", nullable: false),
                    auto_download = table.Column<bool>(type: "INTEGER", nullable: true),
                    reader_mode = table.Column<string>(type: "TEXT", nullable: true),
                    download_dir = table.Column<string>(type: "TEXT", nullable: true),
                    is_explicit = table.Column<bool>(type: "INTEGER", nullable: false),
                    mangaupdates_id = table.Column<string>(type: "TEXT", nullable: true),
                    local_path = table.Column<string>(type: "TEXT", nullable: true)
                },
                constraints: table =>
                {
                    table.PrimaryKey("PK_titles", x => x.id);
                });

            migrationBuilder.CreateTable(
                name: "users",
                columns: table => new
                {
                    id = table.Column<string>(type: "TEXT", nullable: false),
                    username = table.Column<string>(type: "TEXT", nullable: false),
                    password_hash = table.Column<string>(type: "TEXT", nullable: false),
                    created_at = table.Column<DateTime>(type: "TEXT", nullable: false),
                    role = table.Column<string>(type: "TEXT", nullable: false),
                    allow_explicit = table.Column<bool>(type: "INTEGER", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("PK_users", x => x.id);
                });

            migrationBuilder.CreateTable(
                name: "chapters",
                columns: table => new
                {
                    id = table.Column<string>(type: "TEXT", nullable: false),
                    title_id = table.Column<string>(type: "TEXT", nullable: false),
                    title = table.Column<string>(type: "TEXT", nullable: true),
                    number = table.Column<double>(type: "REAL", nullable: false),
                    volume = table.Column<double>(type: "REAL", nullable: true),
                    local_path = table.Column<string>(type: "TEXT", nullable: true),
                    page_count = table.Column<int>(type: "INTEGER", nullable: false),
                    downloaded = table.Column<bool>(type: "INTEGER", nullable: false),
                    created_at = table.Column<DateTime>(type: "TEXT", nullable: false),
                    is_new = table.Column<bool>(type: "INTEGER", nullable: false),
                    chapter_format = table.Column<string>(type: "TEXT", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("PK_chapters", x => x.id);
                    table.ForeignKey(
                        name: "FK_chapters_titles_title_id",
                        column: x => x.title_id,
                        principalTable: "titles",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Cascade);
                });

            migrationBuilder.CreateTable(
                name: "sync_log",
                columns: table => new
                {
                    id = table.Column<string>(type: "TEXT", nullable: false),
                    title_id = table.Column<string>(type: "TEXT", nullable: false),
                    message = table.Column<string>(type: "TEXT", nullable: false),
                    created_at = table.Column<DateTime>(type: "TEXT", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("PK_sync_log", x => x.id);
                    table.ForeignKey(
                        name: "FK_sync_log_titles_title_id",
                        column: x => x.title_id,
                        principalTable: "titles",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Cascade);
                });

            migrationBuilder.CreateTable(
                name: "sync_warnings",
                columns: table => new
                {
                    id = table.Column<string>(type: "TEXT", nullable: false),
                    title_id = table.Column<string>(type: "TEXT", nullable: false),
                    plugin_id = table.Column<string>(type: "TEXT", nullable: false),
                    message = table.Column<string>(type: "TEXT", nullable: false),
                    created_at = table.Column<DateTime>(type: "TEXT", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("PK_sync_warnings", x => x.id);
                    table.ForeignKey(
                        name: "FK_sync_warnings_titles_title_id",
                        column: x => x.title_id,
                        principalTable: "titles",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Cascade);
                });

            migrationBuilder.CreateTable(
                name: "title_aliases",
                columns: table => new
                {
                    id = table.Column<string>(type: "TEXT", nullable: false),
                    title_id = table.Column<string>(type: "TEXT", nullable: false),
                    alias = table.Column<string>(type: "TEXT", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("PK_title_aliases", x => x.id);
                    table.ForeignKey(
                        name: "FK_title_aliases_titles_title_id",
                        column: x => x.title_id,
                        principalTable: "titles",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Cascade);
                });

            migrationBuilder.CreateTable(
                name: "title_sources",
                columns: table => new
                {
                    id = table.Column<string>(type: "TEXT", nullable: false),
                    title_id = table.Column<string>(type: "TEXT", nullable: false),
                    source = table.Column<string>(type: "TEXT", nullable: false),
                    source_id = table.Column<string>(type: "TEXT", nullable: false),
                    discovered_at = table.Column<DateTime>(type: "TEXT", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("PK_title_sources", x => x.id);
                    table.ForeignKey(
                        name: "FK_title_sources_titles_title_id",
                        column: x => x.title_id,
                        principalTable: "titles",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Cascade);
                });

            migrationBuilder.CreateTable(
                name: "user_title_settings",
                columns: table => new
                {
                    user_id = table.Column<string>(type: "TEXT", nullable: false),
                    title_id = table.Column<string>(type: "TEXT", nullable: false),
                    reader_mode = table.Column<string>(type: "TEXT", nullable: true)
                },
                constraints: table =>
                {
                    table.PrimaryKey("PK_user_title_settings", x => new { x.user_id, x.title_id });
                    table.ForeignKey(
                        name: "FK_user_title_settings_titles_title_id",
                        column: x => x.title_id,
                        principalTable: "titles",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Cascade);
                    table.ForeignKey(
                        name: "FK_user_title_settings_users_user_id",
                        column: x => x.user_id,
                        principalTable: "users",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Cascade);
                });

            migrationBuilder.CreateTable(
                name: "user_titles",
                columns: table => new
                {
                    user_id = table.Column<string>(type: "TEXT", nullable: false),
                    title_id = table.Column<string>(type: "TEXT", nullable: false),
                    added_at = table.Column<DateTime>(type: "TEXT", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("PK_user_titles", x => new { x.user_id, x.title_id });
                    table.ForeignKey(
                        name: "FK_user_titles_titles_title_id",
                        column: x => x.title_id,
                        principalTable: "titles",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Cascade);
                    table.ForeignKey(
                        name: "FK_user_titles_users_user_id",
                        column: x => x.user_id,
                        principalTable: "users",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Cascade);
                });

            migrationBuilder.CreateTable(
                name: "chapter_sources",
                columns: table => new
                {
                    id = table.Column<string>(type: "TEXT", nullable: false),
                    chapter_id = table.Column<string>(type: "TEXT", nullable: false),
                    source = table.Column<string>(type: "TEXT", nullable: false),
                    source_id = table.Column<string>(type: "TEXT", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("PK_chapter_sources", x => x.id);
                    table.ForeignKey(
                        name: "FK_chapter_sources_chapters_chapter_id",
                        column: x => x.chapter_id,
                        principalTable: "chapters",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Cascade);
                });

            migrationBuilder.CreateTable(
                name: "download_queue",
                columns: table => new
                {
                    id = table.Column<string>(type: "TEXT", nullable: false),
                    chapter_id = table.Column<string>(type: "TEXT", nullable: false),
                    manga_title = table.Column<string>(type: "TEXT", nullable: false),
                    chapter_num = table.Column<double>(type: "REAL", nullable: false),
                    status = table.Column<string>(type: "TEXT", nullable: false),
                    error = table.Column<string>(type: "TEXT", nullable: true),
                    created_at = table.Column<DateTime>(type: "TEXT", nullable: false),
                    updated_at = table.Column<DateTime>(type: "TEXT", nullable: false),
                    pages_downloaded = table.Column<int>(type: "INTEGER", nullable: false),
                    pages_total = table.Column<int>(type: "INTEGER", nullable: false),
                    queued_by = table.Column<string>(type: "TEXT", nullable: true)
                },
                constraints: table =>
                {
                    table.PrimaryKey("PK_download_queue", x => x.id);
                    table.ForeignKey(
                        name: "FK_download_queue_chapters_chapter_id",
                        column: x => x.chapter_id,
                        principalTable: "chapters",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Cascade);
                    table.ForeignKey(
                        name: "FK_download_queue_users_queued_by",
                        column: x => x.queued_by,
                        principalTable: "users",
                        principalColumn: "id",
                        onDelete: ReferentialAction.SetNull);
                });

            migrationBuilder.CreateTable(
                name: "read_progress",
                columns: table => new
                {
                    id = table.Column<string>(type: "TEXT", nullable: false),
                    user_id = table.Column<string>(type: "TEXT", nullable: false),
                    chapter_id = table.Column<string>(type: "TEXT", nullable: false),
                    current_page = table.Column<int>(type: "INTEGER", nullable: false),
                    completed = table.Column<bool>(type: "INTEGER", nullable: false),
                    updated_at = table.Column<DateTime>(type: "TEXT", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("PK_read_progress", x => x.id);
                    table.ForeignKey(
                        name: "FK_read_progress_chapters_chapter_id",
                        column: x => x.chapter_id,
                        principalTable: "chapters",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Cascade);
                    table.ForeignKey(
                        name: "FK_read_progress_users_user_id",
                        column: x => x.user_id,
                        principalTable: "users",
                        principalColumn: "id",
                        onDelete: ReferentialAction.Cascade);
                });

            migrationBuilder.CreateIndex(
                name: "idx_chapter_sources_chapter_id",
                table: "chapter_sources",
                column: "chapter_id");

            migrationBuilder.CreateIndex(
                name: "IX_chapter_sources_chapter_id_source",
                table: "chapter_sources",
                columns: new[] { "chapter_id", "source" },
                unique: true);

            migrationBuilder.CreateIndex(
                name: "idx_chapters_title_id",
                table: "chapters",
                column: "title_id");

            migrationBuilder.CreateIndex(
                name: "idx_queue_status",
                table: "download_queue",
                columns: new[] { "status", "created_at" });

            migrationBuilder.CreateIndex(
                name: "IX_download_queue_chapter_id",
                table: "download_queue",
                column: "chapter_id",
                unique: true);

            migrationBuilder.CreateIndex(
                name: "IX_download_queue_queued_by",
                table: "download_queue",
                column: "queued_by");

            migrationBuilder.CreateIndex(
                name: "idx_read_progress_chapter",
                table: "read_progress",
                column: "chapter_id");

            migrationBuilder.CreateIndex(
                name: "idx_read_progress_user",
                table: "read_progress",
                column: "user_id");

            migrationBuilder.CreateIndex(
                name: "IX_read_progress_user_id_chapter_id",
                table: "read_progress",
                columns: new[] { "user_id", "chapter_id" },
                unique: true);

            migrationBuilder.CreateIndex(
                name: "idx_sync_log_title_id",
                table: "sync_log",
                columns: new[] { "title_id", "created_at" });

            migrationBuilder.CreateIndex(
                name: "idx_sync_warnings_title_id",
                table: "sync_warnings",
                column: "title_id");

            migrationBuilder.CreateIndex(
                name: "IX_sync_warnings_title_id_plugin_id",
                table: "sync_warnings",
                columns: new[] { "title_id", "plugin_id" },
                unique: true);

            migrationBuilder.CreateIndex(
                name: "idx_title_aliases_title_id",
                table: "title_aliases",
                column: "title_id");

            migrationBuilder.CreateIndex(
                name: "IX_title_sources_title_id_source",
                table: "title_sources",
                columns: new[] { "title_id", "source" },
                unique: true);

            migrationBuilder.CreateIndex(
                name: "IX_titles_mangaupdates_id",
                table: "titles",
                column: "mangaupdates_id",
                unique: true,
                filter: "mangaupdates_id IS NOT NULL");

            migrationBuilder.CreateIndex(
                name: "IX_user_title_settings_title_id",
                table: "user_title_settings",
                column: "title_id");

            migrationBuilder.CreateIndex(
                name: "idx_user_titles_user",
                table: "user_titles",
                column: "user_id");

            migrationBuilder.CreateIndex(
                name: "IX_user_titles_title_id",
                table: "user_titles",
                column: "title_id");

            migrationBuilder.CreateIndex(
                name: "IX_users_username",
                table: "users",
                column: "username",
                unique: true);
        }

        /// <inheritdoc />
        protected override void Down(MigrationBuilder migrationBuilder)
        {
            migrationBuilder.DropTable(
                name: "chapter_sources");

            migrationBuilder.DropTable(
                name: "download_queue");

            migrationBuilder.DropTable(
                name: "external_sources");

            migrationBuilder.DropTable(
                name: "read_progress");

            migrationBuilder.DropTable(
                name: "server_settings");

            migrationBuilder.DropTable(
                name: "sync_log");

            migrationBuilder.DropTable(
                name: "sync_warnings");

            migrationBuilder.DropTable(
                name: "title_aliases");

            migrationBuilder.DropTable(
                name: "title_meta");

            migrationBuilder.DropTable(
                name: "title_sources");

            migrationBuilder.DropTable(
                name: "user_title_settings");

            migrationBuilder.DropTable(
                name: "user_titles");

            migrationBuilder.DropTable(
                name: "chapters");

            migrationBuilder.DropTable(
                name: "users");

            migrationBuilder.DropTable(
                name: "titles");
        }
    }
}
