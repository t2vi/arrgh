import 'package:cached_network_image/cached_network_image.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_staggered_grid_view/flutter_staggered_grid_view.dart';
import 'package:go_router/go_router.dart';

import '../api/client.dart';
import '../models/manga.dart';

final libraryProvider = FutureProvider<List<Manga>>((ref) async {
  final client = ref.read(apiClientProvider);
  final data = await client.get('/api/manga');
  final items = data['items'] as List<dynamic>;
  return items.map((e) => Manga.fromJson(e as Map<String, dynamic>)).toList();
});

class LibraryScreen extends ConsumerWidget {
  const LibraryScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final library = ref.watch(libraryProvider);

    return Scaffold(
      appBar: AppBar(
        title: const Text('Library'),
        actions: [
          IconButton(
            icon: const Icon(Icons.search),
            onPressed: () => showSearch(context: context, delegate: _MangaSearchDelegate(ref)),
          ),
          IconButton(
            icon: const Icon(Icons.settings),
            onPressed: () => context.push('/settings'),
          ),
        ],
      ),
      body: library.when(
        data: (mangas) => MangaGrid(mangas: mangas),
        loading: () => const Center(child: CircularProgressIndicator()),
        error: (e, _) => Center(child: Text('Error: $e')),
      ),
    );
  }
}

class MangaGrid extends ConsumerWidget {
  final List<Manga> mangas;
  const MangaGrid({super.key, required this.mangas});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final client = ref.read(apiClientProvider);

    return MasonryGridView.count(
      padding: const EdgeInsets.all(8),
      crossAxisCount: _crossAxisCount(context),
      mainAxisSpacing: 8,
      crossAxisSpacing: 8,
      itemCount: mangas.length,
      itemBuilder: (context, index) {
        final manga = mangas[index];
        return GestureDetector(
          onTap: () => context.push('/manga/${manga.id}'),
          child: ClipRRect(
            borderRadius: BorderRadius.circular(8),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                AspectRatio(
                  aspectRatio: 2 / 3,
                  child: CachedNetworkImage(
                    imageUrl: client.coverUrl(manga.id),
                    fit: BoxFit.cover,
                    placeholder: (_, __) => Container(color: Colors.grey[800]),
                    errorWidget: (_, __, ___) => Container(
                      color: Colors.grey[800],
                      child: const Icon(Icons.book, size: 48, color: Colors.white54),
                    ),
                  ),
                ),
                Padding(
                  padding: const EdgeInsets.all(6),
                  child: Text(
                    manga.title,
                    style: Theme.of(context).textTheme.bodySmall,
                    maxLines: 2,
                    overflow: TextOverflow.ellipsis,
                  ),
                ),
              ],
            ),
          ),
        );
      },
    );
  }

  int _crossAxisCount(BuildContext context) {
    final width = MediaQuery.of(context).size.width;
    if (width > 1200) return 6;
    if (width > 800) return 4;
    if (width > 600) return 3;
    return 2;
  }
}

class _MangaSearchDelegate extends SearchDelegate<Manga?> {
  final WidgetRef ref;
  _MangaSearchDelegate(this.ref);

  @override
  List<Widget> buildActions(BuildContext context) => [
        IconButton(icon: const Icon(Icons.clear), onPressed: () => query = ''),
      ];

  @override
  Widget buildLeading(BuildContext context) =>
      IconButton(icon: const Icon(Icons.arrow_back), onPressed: () => close(context, null));

  @override
  Widget buildResults(BuildContext context) => _SearchResults(query: query, ref: ref);

  @override
  Widget buildSuggestions(BuildContext context) =>
      query.isEmpty ? const SizedBox() : _SearchResults(query: query, ref: ref);
}

class _SearchResults extends ConsumerWidget {
  final String query;
  final WidgetRef ref;
  const _SearchResults({required this.query, required this.ref});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final client = ref.read(apiClientProvider);
    return FutureBuilder(
      future: client.get('/api/manga', params: {'search': query}),
      builder: (context, snapshot) {
        if (!snapshot.hasData) return const Center(child: CircularProgressIndicator());
        final items = (snapshot.data!['items'] as List<dynamic>)
            .map((e) => Manga.fromJson(e as Map<String, dynamic>))
            .toList();
        return MangaGrid(mangas: items);
      },
    );
  }
}
