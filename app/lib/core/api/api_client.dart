import 'package:dio/dio.dart';
import 'package:shared_preferences/shared_preferences.dart';

const _tokenKey = 'arrgh_token';
const _usernameKey = 'arrgh_username';
const _serverUrlKey = 'arrgh_server_url';
const _defaultUrl = 'http://localhost:3000';

class ApiClient {
  ApiClient._();
  static final ApiClient instance = ApiClient._();

  late final Dio _dio;
  late SharedPreferences _prefs;
  String _baseUrl = _defaultUrl;
  bool _initialized = false;

  Future<void> init() async {
    if (_initialized) return;
    _initialized = true;

    _prefs = await SharedPreferences.getInstance();
    _baseUrl = _prefs.getString(_serverUrlKey) ?? _defaultUrl;

    _dio = Dio(BaseOptions(
      baseUrl: '$_baseUrl/api/',
      connectTimeout: const Duration(seconds: 10),
      receiveTimeout: const Duration(seconds: 30),
      headers: {'Content-Type': 'application/json'},
    ));

    _dio.interceptors.add(InterceptorsWrapper(
      onRequest: (opts, handler) async {
        final token = _prefs.getString(_tokenKey);
        if (token != null) {
          opts.headers['Authorization'] = 'Bearer $token';
        }
        handler.next(opts);
      },
      onError: (err, handler) async {
        if (err.response?.statusCode == 401) {
          await logout();
        }
        handler.next(err);
      },
    ));
  }

  Dio get dio => _dio;
  String get baseUrl => _baseUrl;

  Future<void> setServerUrl(String url) async {
    _baseUrl = url.trimRight().replaceAll(RegExp(r'/$'), '');
    await _prefs.setString(_serverUrlKey, _baseUrl);
    _dio.options.baseUrl = '$_baseUrl/api/';
  }

  Future<void> setToken(String token, String username) async {
    await _prefs.setString(_tokenKey, token);
    await _prefs.setString(_usernameKey, username);
  }

  String? getTokenSync() => _prefs.getString(_tokenKey);
  String? getUsernameSync() => _prefs.getString(_usernameKey);

  Future<void> logout() async {
    await _prefs.remove(_tokenKey);
    await _prefs.remove(_usernameKey);
  }

  String coverUrl(String mangaId) => '$_baseUrl/api/media/cover/$mangaId';
  String pageUrl(String chapterId, int page) =>
      '$_baseUrl/api/media/page/$chapterId/$page';
  String proxyUrl(String imageUrl) =>
      '$_baseUrl/api/media/proxy?url=${Uri.encodeComponent(imageUrl)}';
}

final api = ApiClient.instance;
