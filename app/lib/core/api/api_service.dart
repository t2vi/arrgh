import '../models/models.dart';
import 'api_client.dart';

class ApiService {
  ApiService._();
  static final ApiService instance = ApiService._();

  // ——— Auth ———

  Future<Map<String, dynamic>> authStatus() async {
    final res = await api.dio.get('auth/status');
    return res.data as Map<String, dynamic>;
  }

  Future<String> login(String username, String password) async {
    final res = await api.dio.post('auth/login', data: {
      'username': username,
      'password': password,
    });
    final token = res.data['token'] as String;
    await api.setToken(token, username);
    return token;
  }

  Future<String> register(String username, String password) async {
    final res = await api.dio.post('auth/register', data: {
      'username': username,
      'password': password,
    });
    final token = res.data['token'] as String;
    await api.setToken(token, username);
    return token;
  }

  Future<Map<String, dynamic>> me() async {
    final res = await api.dio.get('auth/me');
    return res.data as Map<String, dynamic>;
  }

  // ——— Manga ———

  Future<PaginatedManga> listManga({int page = 1, String? search}) async {
    final res = await api.dio.get('manga', queryParameters: {
      'page': page,
      'limit': 40,
      if (search != null && search.isNotEmpty) 'search': search,
    });
    return PaginatedManga.fromJson(res.data as Map<String, dynamic>);
  }

  Future<Manga> getManga(String id) async {
    final res = await api.dio.get('manga/$id');
    return Manga.fromJson(res.data as Map<String, dynamic>);
  }

  Future<void> removeManga(String id, {bool deleteFiles = false}) async {
    await api.dio.delete('manga/$id',
        queryParameters: {'delete_files': deleteFiles});
  }

  Future<void> syncManga(String id) async {
    await api.dio.post('manga/$id/sync');
  }

  Future<List<ContinueItem>> continueReading() async {
    final res = await api.dio.get('progress/continue');
    return (res.data as List)
        .map((e) => ContinueItem.fromJson(e as Map<String, dynamic>))
        .toList();
  }

  Future<List<NewRelease>> newReleases() async {
    final res = await api.dio.get('manga/new-releases');
    return (res.data as List)
        .map((e) => NewRelease.fromJson(e as Map<String, dynamic>))
        .toList();
  }

  // ——— Chapters ———

  Future<List<Chapter>> listChapters(String mangaId) async {
    final res = await api.dio.get('chapters/manga/$mangaId');
    return (res.data as List)
        .map((e) => Chapter.fromJson(e as Map<String, dynamic>))
        .toList();
  }

  Future<Chapter> getChapter(String id) async {
    final res = await api.dio.get('chapters/$id');
    return Chapter.fromJson(res.data as Map<String, dynamic>);
  }

  Future<void> downloadChapter(String id) async {
    await api.dio.post('chapters/$id/download');
  }

  // ——— Queue ———

  Future<List<QueueItem>> getQueue() async {
    final res = await api.dio.get('queue');
    return (res.data as List)
        .map((e) => QueueItem.fromJson(e as Map<String, dynamic>))
        .toList();
  }

  Future<List<QueueItem>> getMangaQueue(String mangaId) async {
    final res = await api.dio.get('queue/manga/$mangaId');
    return (res.data as List)
        .map((e) => QueueItem.fromJson(e as Map<String, dynamic>))
        .toList();
  }

  Future<void> removeFromQueue(String id) async {
    await api.dio.delete('queue/$id');
  }

  Future<void> clearCompletedQueue() async {
    await api.dio.delete('queue/completed');
  }

  // ——— Progress ———

  Future<List<ReadProgress>> mangaProgress(String mangaId) async {
    final res = await api.dio.get('progress/manga/$mangaId');
    return (res.data as List)
        .map((e) => ReadProgress.fromJson(e as Map<String, dynamic>))
        .toList();
  }

  Future<ReadProgress?> getProgress(String chapterId) async {
    try {
      final res = await api.dio.get('progress/$chapterId');
      return ReadProgress.fromJson(res.data as Map<String, dynamic>);
    } catch (_) {
      return null;
    }
  }

  Future<void> updateProgress(
      String chapterId, int page, bool completed) async {
    await api.dio.put('progress/$chapterId', data: {
      'current_page': page,
      'completed': completed,
    });
  }

  // ——— Discover ———

  Future<List<SearchResult>> search(String query, String source) async {
    final res = await api.dio.get('discover', queryParameters: {
      'q': query,
      'source': source,
    });
    return (res.data as List)
        .map((e) => SearchResult.fromJson(e as Map<String, dynamic>))
        .toList();
  }

  Future<List<SearchResult>> trending() async {
    final res = await api.dio.get('discover/trending');
    return (res.data as List)
        .map((e) => SearchResult.fromJson(e as Map<String, dynamic>))
        .toList();
  }

  Future<MangaDetail> discoverDetail(String source, String sourceId) async {
    final res = await api.dio.get('discover/detail', queryParameters: {
      'source': source,
      'source_id': sourceId,
    });
    return MangaDetail.fromJson(res.data as Map<String, dynamic>);
  }

  Future<Manga> addManga(SearchResult result) async {
    final res =
        await api.dio.post('discover/add', data: result.toAddRequest());
    return Manga.fromJson(res.data as Map<String, dynamic>);
  }

  Future<void> setMangaAutoDownload(String id, bool? value) async {
    await api.dio.patch('manga/$id', data: {'auto_download': value});
  }

  Future<void> setMangaReaderMode(String id, String? value) async {
    await api.dio.patch('manga/$id', data: {'reader_mode': value});
  }

  Future<void> setMangaDownloadDir(String id, String? value) async {
    await api.dio.patch('manga/$id', data: {'download_dir': value});
  }

  // ——— Settings ———

  Future<Map<String, dynamic>> getSettings() async {
    final res = await api.dio.get('settings');
    return res.data as Map<String, dynamic>;
  }

  Future<void> saveSettings({
    int? downloadWorkers,
    int? indexIntervalHours,
    bool? autoDownload,
    String? readerMode,
  }) async {
    await api.dio.post('settings', data: {
      if (downloadWorkers != null) 'download_workers': downloadWorkers,
      if (indexIntervalHours != null) 'index_interval_hours': indexIntervalHours,
      if (autoDownload != null) 'auto_download': autoDownload,
      if (readerMode != null) 'reader_mode': readerMode,
    });
  }
}

final svc = ApiService.instance;
