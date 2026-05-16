class Manga {
  final String id;
  final String title;
  final String? description;
  final String? coverUrl;
  final String status;
  final String source;
  final String? author;
  final int? year;
  final String? tags;
  final String syncStatus;
  final String contentType;
  final bool? autoDownload;
  final String? readerMode;
  final String? downloadDir;
  final int totalChapters;
  final int downloadedChapters;
  final int chaptersRead;
  final String updatedAt;

  const Manga({
    required this.id,
    required this.title,
    this.description,
    this.coverUrl,
    required this.status,
    required this.source,
    this.author,
    this.year,
    this.tags,
    required this.syncStatus,
    this.contentType = 'manga',
    this.autoDownload,
    this.readerMode,
    this.downloadDir,
    required this.totalChapters,
    required this.downloadedChapters,
    required this.chaptersRead,
    required this.updatedAt,
  });

  factory Manga.fromJson(Map<String, dynamic> j) => Manga(
        id: j['id'] as String,
        title: j['title'] as String,
        description: j['description'] as String?,
        coverUrl: j['cover_url'] as String?,
        status: j['status'] as String? ?? 'unknown',
        source: j['source'] as String? ?? '',
        author: j['author'] as String?,
        year: j['year'] as int?,
        tags: j['tags'] as String?,
        syncStatus: j['sync_status'] as String? ?? 'ready',
        contentType: j['content_type'] as String? ?? 'manga',
        autoDownload: j['auto_download'] as bool?,
        readerMode: j['reader_mode'] as String?,
        downloadDir: j['download_dir'] as String?,
        totalChapters: (j['total_chapters'] as num?)?.toInt() ?? 0,
        downloadedChapters: (j['downloaded_chapters'] as num?)?.toInt() ?? 0,
        chaptersRead: (j['chapters_read'] as num?)?.toInt() ?? 0,
        updatedAt: j['updated_at'] as String? ?? '',
      );
}

class QueueItem {
  final String id;
  final String chapterId;
  final String mangaTitle;
  final double chapterNum;
  final String status;
  final String? error;
  final String createdAt;

  const QueueItem({
    required this.id,
    required this.chapterId,
    required this.mangaTitle,
    required this.chapterNum,
    required this.status,
    this.error,
    required this.createdAt,
  });

  factory QueueItem.fromJson(Map<String, dynamic> j) => QueueItem(
        id: j['id'] as String,
        chapterId: j['chapter_id'] as String,
        mangaTitle: j['manga_title'] as String? ?? '',
        chapterNum: (j['chapter_num'] as num?)?.toDouble() ?? 0,
        status: j['status'] as String? ?? 'pending',
        error: j['error'] as String?,
        createdAt: j['created_at'] as String? ?? '',
      );
}

class Chapter {
  final String id;
  final String mangaId;
  final String? title;
  final double number;
  final double? volume;
  final String? sourceId;
  final String? localPath;
  final int pageCount;
  final bool downloaded;

  const Chapter({
    required this.id,
    required this.mangaId,
    this.title,
    required this.number,
    this.volume,
    this.sourceId,
    this.localPath,
    required this.pageCount,
    required this.downloaded,
  });

  factory Chapter.fromJson(Map<String, dynamic> j) => Chapter(
        id: j['id'] as String,
        mangaId: j['manga_id'] as String,
        title: j['title'] as String?,
        number: (j['number'] as num?)?.toDouble() ?? 0,
        volume: (j['volume'] as num?)?.toDouble(),
        sourceId: j['source_id'] as String?,
        localPath: j['local_path'] as String?,
        pageCount: (j['page_count'] as num?)?.toInt() ?? 0,
        downloaded: j['downloaded'] as bool? ?? false,
      );

  String get displayNumber {
    final n = number;
    return n == n.floorToDouble() ? n.toInt().toString() : n.toString();
  }
}

class ReadProgress {
  final String chapterId;
  final int currentPage;
  final bool completed;

  const ReadProgress({
    required this.chapterId,
    required this.currentPage,
    required this.completed,
  });

  factory ReadProgress.fromJson(Map<String, dynamic> j) => ReadProgress(
        chapterId: j['chapter_id'] as String,
        currentPage: (j['current_page'] as num?)?.toInt() ?? 0,
        completed: j['completed'] as bool? ?? false,
      );
}

class SearchResult {
  final String id;
  final String source;
  final String title;
  final String? description;
  final String? coverUrl;
  final String status;
  final String? author;
  final int? year;
  final String? tags;
  final bool inLibrary;
  final String? libraryId;

  const SearchResult({
    required this.id,
    required this.source,
    required this.title,
    this.description,
    this.coverUrl,
    required this.status,
    this.author,
    this.year,
    this.tags,
    required this.inLibrary,
    this.libraryId,
  });

  factory SearchResult.fromJson(Map<String, dynamic> j) => SearchResult(
        id: j['id'] as String,
        source: j['source'] as String,
        title: j['title'] as String,
        description: j['description'] as String?,
        coverUrl: j['cover_url'] as String?,
        status: j['status'] as String? ?? 'unknown',
        author: j['author'] as String?,
        year: j['year'] as int?,
        tags: j['tags'] as String?,
        inLibrary: j['in_library'] as bool? ?? false,
        libraryId: j['library_id'] as String?,
      );

  Map<String, dynamic> toAddRequest() => {
        'source': source,
        'source_id': id,
        'title': title,
        'description': description,
        'cover_url': coverUrl,
        'status': status,
        'author': author,
        'year': year,
        'tags': tags,
      };
}

class ContinueItem {
  final String mangaId;
  final String mangaTitle;
  final String? coverUrl;
  final String chapterId;
  final double chapterNumber;
  final int chaptersRead;
  final int totalChapters;

  const ContinueItem({
    required this.mangaId,
    required this.mangaTitle,
    this.coverUrl,
    required this.chapterId,
    required this.chapterNumber,
    required this.chaptersRead,
    required this.totalChapters,
  });

  factory ContinueItem.fromJson(Map<String, dynamic> j) => ContinueItem(
        mangaId: j['manga_id'] as String,
        mangaTitle: j['manga_title'] as String,
        coverUrl: j['cover_url'] as String?,
        chapterId: j['chapter_id'] as String,
        chapterNumber: (j['chapter_number'] as num?)?.toDouble() ?? 0,
        chaptersRead: (j['chapters_read'] as num?)?.toInt() ?? 0,
        totalChapters: (j['total_chapters'] as num?)?.toInt() ?? 0,
      );

  String get displayNumber {
    final n = chapterNumber;
    return n == n.floorToDouble() ? n.toInt().toString() : n.toString();
  }
}

class MangaDetail {
  final String? description;
  final String? coverUrl;
  final int chapterCount;

  const MangaDetail({this.description, this.coverUrl, required this.chapterCount});

  factory MangaDetail.fromJson(Map<String, dynamic> j) => MangaDetail(
        description: j['description'] as String?,
        coverUrl: j['cover_url'] as String?,
        chapterCount: (j['chapter_count'] as num?)?.toInt() ?? 0,
      );
}

class NewRelease {
  final String chapterId;
  final double chapterNumber;
  final String? chapterTitle;
  final String mangaId;
  final String mangaTitle;
  final String? coverUrl;
  final bool downloaded;
  final String chapterCreatedAt;

  const NewRelease({
    required this.chapterId,
    required this.chapterNumber,
    this.chapterTitle,
    required this.mangaId,
    required this.mangaTitle,
    this.coverUrl,
    required this.downloaded,
    required this.chapterCreatedAt,
  });

  factory NewRelease.fromJson(Map<String, dynamic> j) => NewRelease(
        chapterId: j['chapter_id'] as String,
        chapterNumber: (j['chapter_number'] as num?)?.toDouble() ?? 0,
        chapterTitle: j['chapter_title'] as String?,
        mangaId: j['manga_id'] as String,
        mangaTitle: j['manga_title'] as String,
        coverUrl: j['cover_url'] as String?,
        downloaded: j['downloaded'] as bool? ?? false,
        chapterCreatedAt: j['chapter_created_at'] as String? ?? '',
      );

  String get displayNumber {
    final n = chapterNumber;
    return n == n.floorToDouble() ? n.toInt().toString() : n.toString();
  }
}

class PaginatedManga {
  final List<Manga> items;
  final int total;
  final int page;
  final int limit;

  const PaginatedManga({
    required this.items,
    required this.total,
    required this.page,
    required this.limit,
  });

  factory PaginatedManga.fromJson(Map<String, dynamic> j) => PaginatedManga(
        items: (j['items'] as List).map((e) => Manga.fromJson(e as Map<String, dynamic>)).toList(),
        total: (j['total'] as num?)?.toInt() ?? 0,
        page: (j['page'] as num?)?.toInt() ?? 1,
        limit: (j['limit'] as num?)?.toInt() ?? 20,
      );
}
