class Manga {
  final String id;
  final String title;
  final String? description;
  final String? coverUrl;
  final String status;
  final String source;

  const Manga({
    required this.id,
    required this.title,
    this.description,
    this.coverUrl,
    required this.status,
    required this.source,
  });

  factory Manga.fromJson(Map<String, dynamic> json) => Manga(
        id: json['id'] as String,
        title: json['title'] as String,
        description: json['description'] as String?,
        coverUrl: json['cover_url'] as String?,
        status: json['status'] as String,
        source: json['source'] as String,
      );
}

class Chapter {
  final String id;
  final String mangaId;
  final String? title;
  final double number;
  final double? volume;
  final int pageCount;
  final bool downloaded;

  const Chapter({
    required this.id,
    required this.mangaId,
    this.title,
    required this.number,
    this.volume,
    required this.pageCount,
    required this.downloaded,
  });

  factory Chapter.fromJson(Map<String, dynamic> json) => Chapter(
        id: json['id'] as String,
        mangaId: json['manga_id'] as String,
        title: json['title'] as String?,
        number: (json['number'] as num).toDouble(),
        volume: (json['volume'] as num?)?.toDouble(),
        pageCount: json['page_count'] as int,
        downloaded: json['downloaded'] as bool,
      );

  String get displayTitle => title ?? 'Chapter ${number.toStringAsFixed(number % 1 == 0 ? 0 : 1)}';
}
