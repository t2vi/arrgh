using Microsoft.EntityFrameworkCore.Migrations;

#nullable disable

namespace ArrghServer.Migrations
{
    /// <inheritdoc />
    public partial class AddMetadataSourceColumns : Migration
    {
        /// <inheritdoc />
        protected override void Up(MigrationBuilder migrationBuilder)
        {
            migrationBuilder.AddColumn<string>(
                name: "metadata_source",
                table: "titles",
                type: "TEXT",
                nullable: true);

            migrationBuilder.AddColumn<string>(
                name: "metadata_source_id",
                table: "titles",
                type: "TEXT",
                nullable: true);
        }

        /// <inheritdoc />
        protected override void Down(MigrationBuilder migrationBuilder)
        {
            migrationBuilder.DropColumn(
                name: "metadata_source",
                table: "titles");

            migrationBuilder.DropColumn(
                name: "metadata_source_id",
                table: "titles");
        }
    }
}
