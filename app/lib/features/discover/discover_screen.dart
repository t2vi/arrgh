import 'package:cached_network_image/cached_network_image.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import '../../core/api/api_client.dart';
import '../../core/api/api_service.dart';
import '../../core/models/models.dart';
import '../../core/theme/app_theme.dart';
import '../../shared/utils/platform_utils.dart';

class DiscoverScreen extends ConsumerStatefulWidget {
  const DiscoverScreen({super.key});

  @override
  ConsumerState<DiscoverScreen> createState() => _DiscoverScreenState();
}

class _DiscoverScreenState extends ConsumerState<DiscoverScreen> {
  final _ctrl = TextEditingController();
  List<SearchResult>? _results;
  bool _loading = false;
  String? _error;
  final Map<String, String> _added = {};

  Future<void> _search() async {
    final q = _ctrl.text.trim();
    if (q.isEmpty) return;
    setState(() { _loading = true; _error = null; });
    try {
      final res = await svc.search(q, 'mangapill');
      if (mounted) setState(() { _results = res; _loading = false; });
    } catch (e) {
      if (mounted) setState(() { _error = 'Search failed'; _loading = false; });
    }
  }

  @override
  Widget build(BuildContext context) {
    final tv = isTV(context);
    return Scaffold(
      appBar: AppBar(
        title: const Text('Discover', style: TextStyle(fontWeight: FontWeight.w700)),
        leading: IconButton(
          icon: const Icon(Icons.chevron_left),
          onPressed: () => context.go('/'),
        ),
      ),
      body: Column(
        children: [
          // Search bar
          Padding(
            padding: EdgeInsets.all(tv ? 24 : 16),
            child: Row(
              children: [
                Expanded(
                  child: TextField(
                    controller: _ctrl,
                    decoration: InputDecoration(
                      hintText: 'Search Mangapill…',
                      prefixIcon: const Icon(Icons.search, color: kMutedFg),
                    ),
                    onSubmitted: (_) => _search(),
                    autofocus: !tv,
                  ),
                ),
                const SizedBox(width: 10),
                ElevatedButton(
                  onPressed: _loading ? null : _search,
                  child: _loading
                      ? const SizedBox(
                          width: 18, height: 18,
                          child: CircularProgressIndicator(
                              strokeWidth: 2, color: Colors.white))
                      : const Text('Search'),
                ),
              ],
            ),
          ),

          if (_error != null)
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 16),
              child: Text(_error!, style: const TextStyle(color: Colors.redAccent)),
            ),

          Expanded(
            child: _results == null
                ? const SizedBox.shrink()
                : _results!.isEmpty
                    ? const Center(
                        child: Text('No results', style: TextStyle(color: kMutedFg)))
                    : ListView.builder(
                        padding: EdgeInsets.symmetric(
                            horizontal: tv ? 24 : 16, vertical: 8),
                        itemCount: _results!.length,
                        itemBuilder: (ctx, i) {
                          final r = _results![i];
                          return _SearchRow(
                            result: r,
                            addedId: _added[r.id],
                            onAdd: () async {
                              try {
                                final manga = await svc.addManga(r);
                                setState(() => _added[r.id] = manga.id);
                              } catch (_) {}
                            },
                            onView: (id) => ctx.push('/manga/$id'),
                          );
                        },
                      ),
          ),
        ],
      ),
    );
  }
}

class _SearchRow extends StatefulWidget {
  final SearchResult result;
  final String? addedId;
  final VoidCallback onAdd;
  final ValueChanged<String> onView;

  const _SearchRow({
    required this.result,
    this.addedId,
    required this.onAdd,
    required this.onView,
  });

  @override
  State<_SearchRow> createState() => _SearchRowState();
}

class _SearchRowState extends State<_SearchRow> {
  MangaDetail? _detail;

  @override
  void initState() {
    super.initState();
    if (widget.result.description == null) {
      svc.discoverDetail('mangapill', widget.result.id).then((d) {
        if (mounted) setState(() => _detail = d);
      }).catchError((_) {});
    }
  }

  String? get _coverUrl {
    final url = widget.result.coverUrl;
    if (url == null) return null;
    return api.proxyUrl(url);
  }

  @override
  Widget build(BuildContext context) {
    final r = widget.result;
    final inLibrary = r.inLibrary || widget.addedId != null;
    final libraryId = widget.addedId ?? r.libraryId;
    final description = r.description ?? _detail?.description;

    return Container(
      margin: const EdgeInsets.only(bottom: 10),
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: kCard,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: kBorder),
      ),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          // Cover
          ClipRRect(
            borderRadius: BorderRadius.circular(8),
            child: SizedBox(
              width: 56,
              child: AspectRatio(
                aspectRatio: 2 / 3,
                child: _coverUrl != null
                    ? CachedNetworkImage(imageUrl: _coverUrl!, fit: BoxFit.cover,
                        errorWidget: (_, __, ___) => Container(color: kMuted))
                    : Container(color: kMuted,
                        child: const Icon(Icons.book, color: kMutedFg, size: 20)),
              ),
            ),
          ),
          const SizedBox(width: 12),
          // Info
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(r.title,
                    style: const TextStyle(
                        fontWeight: FontWeight.w600, fontSize: 14)),
                if (r.status != 'unknown') ...[
                  const SizedBox(height: 4),
                  Container(
                    padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
                    decoration: BoxDecoration(
                      color: kMuted,
                      borderRadius: BorderRadius.circular(4),
                    ),
                    child: Text(r.status,
                        style: const TextStyle(fontSize: 10, color: kMutedFg)),
                  ),
                ],
                if (description != null) ...[
                  const SizedBox(height: 6),
                  Text(description,
                      maxLines: 2,
                      overflow: TextOverflow.ellipsis,
                      style: const TextStyle(fontSize: 12, color: kMutedFg, height: 1.4)),
                ] else if (_detail == null) ...[
                  const SizedBox(height: 6),
                  Container(
                    height: 10, width: 160,
                    decoration: BoxDecoration(
                      color: kMuted, borderRadius: BorderRadius.circular(4)),
                  ),
                ],
              ],
            ),
          ),
          const SizedBox(width: 8),
          // Action
          inLibrary
              ? TextButton(
                  onPressed: libraryId != null ? () => widget.onView(libraryId) : null,
                  child: const Text('In Library',
                      style: TextStyle(color: Color(0xFF34d399), fontSize: 12)),
                )
              : ElevatedButton(
                  onPressed: widget.onAdd,
                  style: ElevatedButton.styleFrom(
                      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
                      minimumSize: Size.zero,
                      tapTargetSize: MaterialTapTargetSize.shrinkWrap),
                  child: const Text('Add', style: TextStyle(fontSize: 12)),
                ),
        ],
      ),
    );
  }
}
