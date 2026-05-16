import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../../core/api/api_service.dart';
import '../../core/providers/auth_provider.dart';
import '../../core/theme/app_theme.dart';

class SetupScreen extends ConsumerStatefulWidget {
  const SetupScreen({super.key});

  @override
  ConsumerState<SetupScreen> createState() => _SetupScreenState();
}

class _SetupScreenState extends ConsumerState<SetupScreen> {
  int _step = 1;

  void _onAccountCreated() => setState(() => _step = 2);
  void _onSettingsDone() => ref.read(authProvider.notifier).setAuthenticated();

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
                    style: TextStyle(fontSize: 36, fontWeight: FontWeight.w800, color: kPrimary)),
                const SizedBox(height: 8),
                Text(
                  _step == 1 ? 'Create your account' : 'Configure your library',
                  textAlign: TextAlign.center,
                  style: const TextStyle(color: kMutedFg),
                ),
                const SizedBox(height: 4),
                Text(
                  _step == 1
                      ? 'First-time setup. Use these credentials on any device.'
                      : 'These can be changed later in Settings.',
                  textAlign: TextAlign.center,
                  style: const TextStyle(fontSize: 12, color: kMutedFg),
                ),
                const SizedBox(height: 16),
                Row(
                  mainAxisAlignment: MainAxisAlignment.center,
                  children: [1, 2].map((s) => Container(
                    width: 8, height: 8,
                    margin: const EdgeInsets.symmetric(horizontal: 3),
                    decoration: BoxDecoration(
                      color: _step >= s ? kPrimary : kMuted,
                      shape: BoxShape.circle,
                    ),
                  )).toList(),
                ),
                const SizedBox(height: 32),
                if (_step == 1)
                  _StepAccount(onDone: _onAccountCreated)
                else
                  _StepSettings(onDone: _onSettingsDone),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

// ── Step 1 ────────────────────────────────────────────────────────────────────

class _StepAccount extends StatefulWidget {
  final VoidCallback onDone;
  const _StepAccount({required this.onDone});

  @override
  State<_StepAccount> createState() => _StepAccountState();
}

class _StepAccountState extends State<_StepAccount> {
  final _userCtrl = TextEditingController();
  final _passCtrl = TextEditingController();
  final _confirmCtrl = TextEditingController();
  bool _loading = false;
  String? _error;

  Future<void> _register() async {
    if (_userCtrl.text.trim().isEmpty || _passCtrl.text.isEmpty) {
      setState(() => _error = 'Fill in all fields');
      return;
    }
    if (_passCtrl.text.length < 6) {
      setState(() => _error = 'Password must be at least 6 characters');
      return;
    }
    if (_passCtrl.text != _confirmCtrl.text) {
      setState(() => _error = 'Passwords do not match');
      return;
    }
    setState(() { _loading = true; _error = null; });
    try {
      await svc.register(_userCtrl.text.trim(), _passCtrl.text);
      widget.onDone();
    } catch (e) {
      setState(() => _error = 'Setup failed — ${e.toString()}');
    } finally {
      if (mounted) setState(() => _loading = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    return Column(crossAxisAlignment: CrossAxisAlignment.stretch, children: [
      TextField(controller: _userCtrl,
          decoration: const InputDecoration(labelText: 'Username'),
          textInputAction: TextInputAction.next, autocorrect: false),
      const SizedBox(height: 12),
      TextField(controller: _passCtrl,
          decoration: const InputDecoration(labelText: 'Password'),
          obscureText: true, textInputAction: TextInputAction.next),
      const SizedBox(height: 12),
      TextField(controller: _confirmCtrl,
          decoration: const InputDecoration(labelText: 'Confirm Password'),
          obscureText: true, onSubmitted: (_) => _register()),
      const SizedBox(height: 20),
      if (_error != null)
        Padding(
          padding: const EdgeInsets.only(bottom: 12),
          child: Text(_error!, style: const TextStyle(color: Colors.redAccent)),
        ),
      ElevatedButton(
        onPressed: _loading ? null : _register,
        child: _loading
            ? const SizedBox(height: 18, width: 18,
                child: CircularProgressIndicator(strokeWidth: 2, color: Colors.white))
            : const Text('Create Account', style: TextStyle(fontWeight: FontWeight.w700)),
      ),
    ]);
  }
}

// ── Step 2 ────────────────────────────────────────────────────────────────────

class _StepSettings extends StatefulWidget {
  final VoidCallback onDone;
  const _StepSettings({required this.onDone});

  @override
  State<_StepSettings> createState() => _StepSettingsState();
}

class _StepSettingsState extends State<_StepSettings> {
  int _workers = 2;
  int _hours = 6;
  bool _autoDownload = false;
  String _readerMode = 'paged';
  bool _loading = false;

  Future<void> _save() async {
    setState(() => _loading = true);
    try {
      await svc.saveSettings(
        downloadWorkers: _workers,
        indexIntervalHours: _hours,
        autoDownload: _autoDownload,
        readerMode: _readerMode,
      );
    } finally {
      if (mounted) { setState(() => _loading = false); widget.onDone(); }
    }
  }

  @override
  Widget build(BuildContext context) {
    return Column(crossAxisAlignment: CrossAxisAlignment.stretch, children: [
      _SettingRow(label: 'Download workers', hint: 'Concurrent chapter downloads',
          child: _Stepper(value: _workers, min: 1, max: 10,
              onChanged: (v) => setState(() => _workers = v))),
      const SizedBox(height: 20),
      _SettingRow(label: 'Sync interval (hours)', hint: 'How often to check for new chapters',
          child: _Stepper(value: _hours, min: 1, max: 24,
              onChanged: (v) => setState(() => _hours = v))),
      const SizedBox(height: 20),
      _SettingRow(label: 'Auto-download new chapters', hint: 'Queue when new chapters appear',
          child: Switch(value: _autoDownload,
              onChanged: (v) => setState(() => _autoDownload = v),
              activeColor: kPrimary)),
      const SizedBox(height: 20),
      _SettingRow(label: 'Default reader mode', hint: 'Can be overridden per manga',
          child: _SegmentedControl(value: _readerMode,
              options: const ['paged', 'scroll'],
              onChanged: (v) => setState(() => _readerMode = v))),
      const SizedBox(height: 32),
      ElevatedButton(
        onPressed: _loading ? null : _save,
        child: _loading
            ? const SizedBox(height: 18, width: 18,
                child: CircularProgressIndicator(strokeWidth: 2, color: Colors.white))
            : const Text('Save & go to library', style: TextStyle(fontWeight: FontWeight.w700)),
      ),
      const SizedBox(height: 8),
      TextButton(
        onPressed: widget.onDone,
        child: const Text('Skip, use defaults', style: TextStyle(fontSize: 12, color: kMutedFg)),
      ),
    ]);
  }
}

// ── Primitives ────────────────────────────────────────────────────────────────

class _SettingRow extends StatelessWidget {
  final String label;
  final String hint;
  final Widget child;
  const _SettingRow({required this.label, required this.hint, required this.child});

  @override
  Widget build(BuildContext context) {
    return Row(children: [
      Expanded(child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
        Text(label, style: const TextStyle(fontWeight: FontWeight.w600, fontSize: 14)),
        Text(hint, style: const TextStyle(fontSize: 11, color: kMutedFg)),
      ])),
      const SizedBox(width: 12),
      child,
    ]);
  }
}

class _Stepper extends StatelessWidget {
  final int value;
  final int min;
  final int max;
  final ValueChanged<int> onChanged;
  const _Stepper({required this.value, required this.min, required this.max, required this.onChanged});

  @override
  Widget build(BuildContext context) {
    return Row(mainAxisSize: MainAxisSize.min, children: [
      _Btn(label: '−', enabled: value > min, onTap: () => onChanged(value - 1)),
      SizedBox(width: 28, child: Text('$value',
          textAlign: TextAlign.center,
          style: const TextStyle(fontWeight: FontWeight.w700, fontSize: 15))),
      _Btn(label: '+', enabled: value < max, onTap: () => onChanged(value + 1)),
    ]);
  }
}

class _Btn extends StatelessWidget {
  final String label;
  final bool enabled;
  final VoidCallback onTap;
  const _Btn({required this.label, required this.enabled, required this.onTap});

  @override
  Widget build(BuildContext context) {
    return GestureDetector(
      onTap: enabled ? onTap : null,
      child: Container(
        width: 28, height: 28,
        decoration: BoxDecoration(color: kMuted, borderRadius: BorderRadius.circular(6)),
        child: Center(child: Text(label, style: TextStyle(
            fontWeight: FontWeight.w700, color: enabled ? kForeground : kMutedFg))),
      ),
    );
  }
}

class _SegmentedControl extends StatelessWidget {
  final String value;
  final List<String> options;
  final ValueChanged<String> onChanged;
  const _SegmentedControl({required this.value, required this.options, required this.onChanged});

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.all(2),
      decoration: BoxDecoration(color: kMuted, borderRadius: BorderRadius.circular(8)),
      child: Row(mainAxisSize: MainAxisSize.min, children: options.map((o) {
        final sel = o == value;
        return GestureDetector(
          onTap: () => onChanged(o),
          child: AnimatedContainer(
            duration: const Duration(milliseconds: 150),
            padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 5),
            decoration: BoxDecoration(
              color: sel ? kCard : Colors.transparent,
              borderRadius: BorderRadius.circular(6),
            ),
            child: Text(
              '${o[0].toUpperCase()}${o.substring(1)}',
              style: TextStyle(fontSize: 12, fontWeight: FontWeight.w600,
                  color: sel ? kForeground : kMutedFg),
            ),
          ),
        );
      }).toList()),
    );
  }
}
