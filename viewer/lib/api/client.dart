import 'package:dio/dio.dart';
import 'package:flutter/foundation.dart' show kIsWeb;
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:shared_preferences/shared_preferences.dart';

const _defaultServerUrl = 'http://localhost:3000';

String _initialBaseUrl() {
  if (kIsWeb) {
    final base = Uri.base;
    return '${base.scheme}://${base.host}${base.hasPort ? ':${base.port}' : ''}';
  }
  return _defaultServerUrl;
}

final apiClientProvider = Provider<ApiClient>((ref) => ApiClient());

class ApiClient {
  late final Dio _dio;

  ApiClient() {
    _dio = Dio(BaseOptions(
      baseUrl: _initialBaseUrl(),
      connectTimeout: const Duration(seconds: 10),
      receiveTimeout: const Duration(seconds: 30),
    ));
  }

  Future<void> setBaseUrl(String url) async {
    _dio.options.baseUrl = url;
    final prefs = await SharedPreferences.getInstance();
    await prefs.setString('server_url', url);
  }

  Future<Map<String, dynamic>> get(String path, {Map<String, dynamic>? params}) async {
    final res = await _dio.get(path, queryParameters: params);
    return res.data as Map<String, dynamic>;
  }

  Future<List<dynamic>> getList(String path, {Map<String, dynamic>? params}) async {
    final res = await _dio.get(path, queryParameters: params);
    return res.data as List<dynamic>;
  }

  Future<void> put(String path, Map<String, dynamic> body) async {
    await _dio.put(path, data: body);
  }

  String pageUrl(String chapterId, int page) =>
      '${_dio.options.baseUrl}/api/media/page/$chapterId/$page';

  String coverUrl(String mangaId) =>
      '${_dio.options.baseUrl}/api/media/cover/$mangaId';
}
