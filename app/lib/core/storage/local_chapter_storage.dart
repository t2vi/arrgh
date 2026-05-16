import 'dart:io';
import 'package:path_provider/path_provider.dart';
import 'package:dio/dio.dart';
import '../api/api_client.dart';

class LocalChapterStorage {
  LocalChapterStorage._();
  static final LocalChapterStorage instance = LocalChapterStorage._();

  Future<Directory> _chapterDir(String chapterId) async {
    final base = await getApplicationDocumentsDirectory();
    final dir = Directory('${base.path}/arrgh/chapters/$chapterId');
    await dir.create(recursive: true);
    return dir;
  }

  Future<File> _pageFile(String chapterId, int page) async {
    final dir = await _chapterDir(chapterId);
    return File('${dir.path}/$page.jpg');
  }

  /// Returns local file path if cached, null if not.
  Future<String?> getLocalPage(String chapterId, int page) async {
    final file = await _pageFile(chapterId, page);
    return file.existsSync() ? file.path : null;
  }

  /// True if all pages 0..<pageCount are locally cached.
  Future<bool> isFullyDownloaded(String chapterId, int pageCount) async {
    for (var i = 0; i < pageCount; i++) {
      final file = await _pageFile(chapterId, i);
      if (!file.existsSync()) return false;
    }
    return true;
  }

  /// Downloads all pages from server to local storage.
  /// [onProgress] called with (completedPages, totalPages).
  Future<void> downloadChapter(
    String chapterId,
    int pageCount, {
    void Function(int done, int total)? onProgress,
    void Function()? onCancel,
  }) async {
    final dio = Dio();
    for (var i = 0; i < pageCount; i++) {
      final file = await _pageFile(chapterId, i);
      if (file.existsSync()) {
        onProgress?.call(i + 1, pageCount);
        continue;
      }
      final url = api.pageUrl(chapterId, i);
      final token = api.getTokenSync();
      try {
        await dio.download(
          url,
          file.path,
          options: Options(
            headers: token != null ? {'Authorization': 'Bearer $token'} : null,
            followRedirects: true,
            maxRedirects: 5,
          ),
        );
      } catch (_) {
        // partial download — delete and rethrow
        if (file.existsSync()) await file.delete();
        rethrow;
      }
      onProgress?.call(i + 1, pageCount);
    }
  }

  /// Deletes all locally cached pages for a chapter.
  Future<void> deleteChapter(String chapterId) async {
    final base = await getApplicationDocumentsDirectory();
    final dir = Directory('${base.path}/arrgh/chapters/$chapterId');
    if (dir.existsSync()) await dir.delete(recursive: true);
  }

  /// Deletes all locally cached data for all chapters.
  Future<void> deleteAll() async {
    final base = await getApplicationDocumentsDirectory();
    final dir = Directory('${base.path}/arrgh/chapters');
    if (dir.existsSync()) await dir.delete(recursive: true);
  }

  /// Disk usage in bytes for a single chapter.
  Future<int> chapterSizeBytes(String chapterId) async {
    final base = await getApplicationDocumentsDirectory();
    final dir = Directory('${base.path}/arrgh/chapters/$chapterId');
    if (!dir.existsSync()) return 0;
    var total = 0;
    await for (final f in dir.list()) {
      if (f is File) total += await f.length();
    }
    return total;
  }
}

final localChapters = LocalChapterStorage.instance;
