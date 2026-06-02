using Microsoft.EntityFrameworkCore.Migrations;

#nullable disable

namespace ArrghServer.Migrations
{
    /// <inheritdoc />
    public partial class NhentaiContentTypeHentai : Migration
    {
        /// <inheritdoc />
        protected override void Up(MigrationBuilder migrationBuilder)
        {
            // RAW SQL — intentional: data migration; no EF equivalent for targeted row update by source_key.
            migrationBuilder.Sql(
                "UPDATE external_sources SET content_types = 'hentai' WHERE source_key = 'nhentai' AND content_types = 'manga'");
        }

        /// <inheritdoc />
        protected override void Down(MigrationBuilder migrationBuilder)
        {
            migrationBuilder.Sql(
                "UPDATE external_sources SET content_types = 'manga' WHERE source_key = 'nhentai' AND content_types = 'hentai'");
        }
    }
}
