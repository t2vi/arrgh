import 'dart:async';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../../core/api/api_service.dart';
import '../../core/models/models.dart';
import '../../core/theme/app_theme.dart';

const _statusOrder = {
  'downloading': 0,
  'pending': 1,
  'done': 2,
  'error': 3,
  'cancelled': 4,
};

bool _isActive(QueueItem item) =>
    item.status == 'pending' || item.status == 'downloading';

final _queueProvider = StreamProvider.autoDispose<List<QueueItem>>((ref) async* {
  while (true) {
    try {
      final items = await svc.getQueue();
      items.sort((a, b) =>
          (_statusOrder[a.status] ?? 9).compareTo(_statusOrder[b.status] ?? 9));
      yield items;
    } catch (_) {}
    await Future.delayed(const Duration(seconds: 2));
  }
});

class DownloadsScreen extends ConsumerStatefulWidget {
  const DownloadsScreen({super.key});

  @override
  ConsumerState<DownloadsScreen> createState() => _DownloadsScreenState();
}

class _DownloadsScreenState extends ConsumerState<DownloadsScreen> {
  bool _hadActive = false;
  bool _clearing = false;

  Future<void> _clearCompleted() async {
    setState(() => _clearing = true);
    try {
      await svc.clearCompletedQueue();
      ref.invalidate(_queueProvider);
    } finally {
      if (mounted) setState(() => _clearing = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final queueAsync = ref.watch(_queueProvider);
    final items = queueAsync.valueOrNull ?? [];

    // Auto-clear when queue drains from active to empty
    final hasActive = items.any(_isActive);
    final hasFinished = items.any((i) => i.status == 'done' || i.status == 'cancelled');
    if (_hadActive && !hasActive && hasFinished) {
      WidgetsBinding.instance.addPostFrameCallback((_) => _clearCompleted());
    }
    _hadActive = hasActive;

    final canClear = items.any((i) => !_isActive(i));

    return Scaffold(
      appBar: AppBar(
        title: const Text('Downloads', style: TextStyle(fontWeight: FontWeight.w700)),
        actions: [
          if (canClear)
            TextButton.icon(
              onPressed: _clearing ? null : _clearCompleted,
              icon: _clearing
                  ? const SizedBox(width: 14, height: 14,
                      child: CircularProgressIndicator(strokeWidth: 2, color: kMutedFg))
                  : const Icon(Icons.delete_sweep_outlined, size: 16, color: kMutedFg),
              label: const Text('Clear',
                  style: TextStyle(fontSize: 12, color: kMutedFg)),
            ),
        ],
      ),
      body: queueAsync.isLoading && items.isEmpty
          ? const Center(child: CircularProgressIndicator(color: kPrimary))
          : items.isEmpty
              ? const Center(
                  child: Text('Queue is empty', style: TextStyle(color: kMutedFg)))
              : ListView.separated(
                  itemCount: items.length,
                  separatorBuilder: (_, __) =>
                      const Divider(height: 1, color: kBorder),
                  itemBuilder: (ctx, i) => _QueueRow(
                    item: items[i],
                    onRemove: () async {
                      await svc.removeFromQueue(items[i].id);
                      ref.invalidate(_queueProvider);
                    },
                  ),
                ),
    );
  }
}

class _QueueRow extends StatelessWidget {
  final QueueItem item;
  final VoidCallback onRemove;

  const _QueueRow({required this.item, required this.onRemove});

  @override
  Widget build(BuildContext context) {
    final (icon, color) = switch (item.status) {
      'downloading' => (Icons.downloading, kPrimary),
      'done'        => (Icons.check_circle_outline, const Color(0xFF34d399)),
      'error'       => (Icons.error_outline, Colors.redAccent),
      'cancelled'   => (Icons.cancel_outlined, kMutedFg),
      _             => (Icons.schedule, kMutedFg),
    };

    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 14),
      child: Row(
        children: [
          item.status == 'downloading'
              ? const SizedBox(
                  width: 20,
                  height: 20,
                  child: CircularProgressIndicator(strokeWidth: 2, color: kPrimary),
                )
              : Icon(icon, size: 20, color: color),
          const SizedBox(width: 14),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(item.mangaTitle,
                    style: const TextStyle(fontSize: 14, fontWeight: FontWeight.w500),
                    overflow: TextOverflow.ellipsis),
                Text('Ch. ${_fmt(item.chapterNum)}',
                    style: const TextStyle(fontSize: 12, color: kMutedFg)),
                if (item.error != null)
                  Text(item.error!,
                      style: const TextStyle(fontSize: 11, color: Colors.redAccent),
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis),
              ],
            ),
          ),
          const SizedBox(width: 8),
          _StatusChip(item.status),
          if (item.status != 'downloading') ...[
            const SizedBox(width: 4),
            IconButton(
              icon: const Icon(Icons.close, size: 16, color: kMutedFg),
              onPressed: onRemove,
              padding: const EdgeInsets.all(4),
              constraints: const BoxConstraints(),
            ),
          ],
        ],
      ),
    );
  }

  String _fmt(double n) =>
      n == n.truncateToDouble() ? n.toInt().toString() : n.toString();
}

class _StatusChip extends StatelessWidget {
  final String status;
  const _StatusChip(this.status);

  @override
  Widget build(BuildContext context) {
    final (label, bg, fg) = switch (status) {
      'downloading' => ('Downloading', kPrimary.withValues(alpha: 0.2), kPrimary),
      'done'        => ('Done', const Color(0x2234d399), const Color(0xFF34d399)),
      'error'       => ('Error', Colors.redAccent.withValues(alpha: 0.2), Colors.redAccent),
      'cancelled'   => ('Cancelled', kMuted, kMutedFg),
      _             => ('Pending', kMuted, kMutedFg),
    };
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
      decoration: BoxDecoration(color: bg, borderRadius: BorderRadius.circular(20)),
      child: Text(label,
          style: TextStyle(fontSize: 11, fontWeight: FontWeight.w600, color: fg)),
    );
  }
}
