using Microsoft.EntityFrameworkCore.Migrations;

#nullable disable

namespace ArrghServer.Migrations
{
    /// <inheritdoc />
    public partial class RemoveBrokenPlugins : Migration
    {
        /// <inheritdoc />
        protected override void Up(MigrationBuilder migrationBuilder)
        {
            // RAW SQL — intentional: data migration deleting dead plugin rows by source_key.
            migrationBuilder.Sql("DELETE FROM external_sources WHERE source_key IN ('royalroad', 'manhuafast', 'boxnovel')");
        }

        /// <inheritdoc />
        protected override void Down(MigrationBuilder migrationBuilder)
        {
            // Restore rows are not worth recreating on rollback — they were broken plugins.
        }
    }
}
