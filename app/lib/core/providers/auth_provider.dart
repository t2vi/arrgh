import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../api/api_client.dart';
import '../api/api_service.dart';

enum AuthState { loading, needsSetup, needsLogin, authenticated }

final authProvider = StateNotifierProvider<AuthNotifier, AuthState>((ref) {
  return AuthNotifier();
});

class AuthNotifier extends StateNotifier<AuthState> {
  AuthNotifier() : super(AuthState.loading) {
    _check();
  }

  Future<void> _check() async {
    try {
      final status = await svc.authStatus();
      if (status['needs_setup'] == true) {
        state = AuthState.needsSetup;
        return;
      }
      final token = api.getTokenSync();
      if (token == null) {
        state = AuthState.needsLogin;
        return;
      }
      await svc.me();
      state = AuthState.authenticated;
    } catch (_) {
      final token = api.getTokenSync();
      state = token != null ? AuthState.authenticated : AuthState.needsLogin;
    }
  }

  Future<void> login(String username, String password) async {
    await svc.login(username, password);
    state = AuthState.authenticated;
  }

  Future<void> register(String username, String password) async {
    await svc.register(username, password);
    state = AuthState.authenticated;
  }

  Future<void> logout() async {
    await api.logout();
    state = AuthState.needsLogin;
  }

  void setAuthenticated() => state = AuthState.authenticated;

  Future<void> recheck() => _check();
}
