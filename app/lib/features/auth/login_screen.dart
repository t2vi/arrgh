import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../../core/api/api_client.dart';
import '../../core/providers/auth_provider.dart';
import '../../core/theme/app_theme.dart';

class LoginScreen extends ConsumerStatefulWidget {
  const LoginScreen({super.key});

  @override
  ConsumerState<LoginScreen> createState() => _LoginScreenState();
}

class _LoginScreenState extends ConsumerState<LoginScreen> {
  final _serverCtrl = TextEditingController(text: 'http://');
  final _userCtrl = TextEditingController();
  final _passCtrl = TextEditingController();
  bool _loading = false;
  String? _error;
  bool _showServer = false;

  @override
  void initState() {
    super.initState();
    _serverCtrl.text = api.baseUrl;
  }

  Future<void> _login() async {
    setState(() { _loading = true; _error = null; });
    try {
      if (_showServer) {
        await api.setServerUrl(_serverCtrl.text.trim());
      }
      await ref.read(authProvider.notifier).login(
          _userCtrl.text.trim(), _passCtrl.text);
    } catch (e) {
      setState(() => _error = _friendlyError(e));
    } finally {
      if (mounted) setState(() => _loading = false);
    }
  }

  String _friendlyError(Object e) {
    final s = e.toString();
    if (s.contains('401')) return 'Invalid credentials';
    if (s.contains('connect') || s.contains('SocketException')) {
      return 'Cannot reach server — check URL';
    }
    return s; // show raw error for debugging
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      body: Center(
        child: SingleChildScrollView(
          padding: const EdgeInsets.all(24),
          child: ConstrainedBox(
            constraints: const BoxConstraints(maxWidth: 400),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                const Text('*ARRgh',
                    textAlign: TextAlign.center,
                    style: TextStyle(
                        fontSize: 36,
                        fontWeight: FontWeight.w800,
                        color: kPrimary)),
                const SizedBox(height: 8),
                const Text('Sign in to your library',
                    textAlign: TextAlign.center,
                    style: TextStyle(color: kMutedFg)),
                const SizedBox(height: 32),

                // Server URL toggle
                GestureDetector(
                  onTap: () => setState(() => _showServer = !_showServer),
                  child: Row(
                    mainAxisAlignment: MainAxisAlignment.end,
                    children: [
                      Text(
                        _showServer ? 'Hide server URL' : 'Change server URL',
                        style: const TextStyle(fontSize: 12, color: kMutedFg),
                      ),
                    ],
                  ),
                ),
                if (_showServer) ...[
                  const SizedBox(height: 8),
                  TextField(
                    controller: _serverCtrl,
                    decoration: const InputDecoration(
                      labelText: 'Server URL',
                      hintText: 'http://192.168.1.x:3000',
                    ),
                    keyboardType: TextInputType.url,
                    autocorrect: false,
                  ),
                ],
                const SizedBox(height: 12),

                TextField(
                  controller: _userCtrl,
                  decoration: const InputDecoration(labelText: 'Username'),
                  textInputAction: TextInputAction.next,
                  autocorrect: false,
                ),
                const SizedBox(height: 12),
                TextField(
                  controller: _passCtrl,
                  decoration: const InputDecoration(labelText: 'Password'),
                  obscureText: true,
                  onSubmitted: (_) => _login(),
                ),
                const SizedBox(height: 20),

                if (_error != null)
                  Padding(
                    padding: const EdgeInsets.only(bottom: 12),
                    child: Text(_error!,
                        style: const TextStyle(color: Colors.redAccent)),
                  ),

                ElevatedButton(
                  onPressed: _loading ? null : _login,
                  child: _loading
                      ? const SizedBox(
                          height: 18, width: 18,
                          child: CircularProgressIndicator(
                              strokeWidth: 2, color: Colors.white))
                      : const Text('Sign In',
                          style: TextStyle(fontWeight: FontWeight.w700)),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}
