import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import '../../core/api/api_service.dart';
import '../../core/models/models.dart';
import '../../core/theme/app_theme.dart';
import '../../shared/utils/platform_utils.dart';
import '../../shared/widgets/manga_card.dart';

final _libraryProvider = FutureProvider.autoDispose.family<PaginatedManga, String>((ref, search) {
  return svc.listManga(search: search.isEmpty ? null : search);
});

class LibraryScreen extends ConsumerStatefulWidget {
  const LibraryScreen({super.key});

  @override
  ConsumerState<LibraryScreen> createState() => _LibraryScreenState();
}

class _LibraryScreenState extends ConsumerState<LibraryScreen> {
  final _searchCtrl = TextEditingController();
  String _search = '';

  Future<void> _showRemoveSheet(BuildContext ctx, Manga m) async {
    final result = await showModalBottomSheet<String>(
      context: ctx,
      backgroundColor: kCard,
      shape: const RoundedRectangleBorder(
        borderRadius: BorderRadius.vertical(top: Radius.circular(16)),
      ),
      builder: (_) => SafeArea(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Container(
              width: 36, height: 4,
              margin: const EdgeInsets.only(top: 12, bottom: 16),
              decoration: BoxDecoration(
                color: kBorder, borderRadius: BorderRadius.circular(2)),
            ),
            Padding(
              padding: const EdgeInsets.fromLTRB(20, 0, 20, 8),
              child: Text(m.title,
                  style: const TextStyle(
                      fontWeight: FontWeight.w700, fontSize: 15),
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis),
            ),
            ListTile(
              leading: const Icon(Icons.remove_circle_outline, color: kMutedFg),
              title: const Text('Remove from library'),
              onTap: () => Navigator.pop(ctx, 'remove'),
            ),
            if (m.downloadedChapters > 0)
              ListTile(
                leading: const Icon(Icons.delete_outline, color: Colors.redAccent),
                title: const Text('Remove + delete files',
                    style: TextStyle(color: Colors.redAccent)),
                onTap: () => Navigator.pop(ctx, 'remove_files'),
              ),
            const SizedBox(height: 8),
          ],
        ),
      ),
    );
    if (result == null) return;
    await svc.removeManga(m.id, deleteFiles: result == 'remove_files');
    ref.invalidate(_libraryProvider);
  }

  @override
  Widget build(BuildContext context) {
    final tv = isTV(context);
    final layout = layoutType(context);
    final libAsync = ref.watch(_libraryProvider(_search));
    final items = libAsync.valueOrNull?.items ?? [];

    final crossCount = switch (layout) {
      LayoutType.tv => 7,
      LayoutType.tablet => 4,
      LayoutType.phone => 3,
    };

    return Scaffold(
      appBar: AppBar(
        title: const Text('Library',
            style: TextStyle(fontWeight: FontWeight.w700)),
        bottom: PreferredSize(
          preferredSize: const Size.fromHeight(56),
          child: Padding(
            padding: const EdgeInsets.fromLTRB(16, 0, 16, 8),
            child: TextField(
              controller: _searchCtrl,
              decoration: InputDecoration(
                hintText: 'Search library…',
                prefixIcon: const Icon(Icons.search, color: kMutedFg),
                suffixIcon: _search.isNotEmpty
                    ? IconButton(
                        icon: const Icon(Icons.close, color: kMutedFg),
                        onPressed: () {
                          _searchCtrl.clear();
                          setState(() => _search = '');
                        })
                    : null,
                contentPadding: const EdgeInsets.symmetric(vertical: 0),
              ),
              onChanged: (v) => setState(() => _search = v.trim()),
            ),
          ),
        ),
      ),
      body: libAsync.isLoading && items.isEmpty
          ? const Center(child: CircularProgressIndicator(color: kPrimary))
          : items.isEmpty
              ? Center(
                  child: Column(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      Text(
                        _search.isEmpty ? 'Library empty' : 'No results',
                        style: const TextStyle(color: kMutedFg),
                      ),
                      if (_search.isEmpty) ...[
                        const SizedBox(height: 12),
                        ElevatedButton(
                            onPressed: () => context.go('/discover'),
                            child: const Text('Discover Manga')),
                      ],
                    ],
                  ),
                )
              : GridView.builder(
                  padding: EdgeInsets.all(tv ? 24 : 16),
                  gridDelegate: SliverGridDelegateWithFixedCrossAxisCount(
                    crossAxisCount: crossCount,
                    childAspectRatio: 0.58,
                    crossAxisSpacing: tv ? 16 : 10,
                    mainAxisSpacing: tv ? 20 : 14,
                  ),
                  itemCount: items.length,
                  itemBuilder: (ctx, i) {
                    final m = items[i];
                    return MangaCoverCard(
                      title: m.title,
                      mangaId: m.id,
                      coverUrl: m.coverUrl,
                      subtitle: m.author,
                      syncing: m.syncStatus == 'syncing',
                      isTv: tv,
                      onTap: () => ctx.push('/manga/${m.id}'),
                      onLongPress: () => _showRemoveSheet(ctx, m),
                      badge: ContentTypePill(m.contentType),
                    );
                  },
                ),
      floatingActionButton: FloatingActionButton(
        onPressed: () => context.go('/discover'),
        backgroundColor: kPrimary,
        child: const Icon(Icons.add, color: Colors.white),
      ),
    );
  }
}
