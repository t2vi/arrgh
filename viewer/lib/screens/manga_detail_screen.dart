import 'package:cached_network_image/cached_network_image.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../api/client.dart';
import '../models/manga.dart';

class MangaDetailScreen extends ConsumerWidget {
  final String mangaId;
  const MangaDetailScreen({super.key, required this.mangaId});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final client = ref.read(apiClientProvider);

    return Scaffold(
      body: FutureBuilder(
        future: Future.wait([
          client.get('/api/manga/$mangaId'),
          client.getList('/api/chapters/manga/$mangaId'),
        ]),
        builder: (context, snapshot) {
          if (!snapshot.hasData) {
            return const Center(child: CircularProgressIndicator());
          }
          if (snapshot.hasError) {
            return Center(child: Text('Error: ${snapshot.error}'));
          }

          final manga = Manga.fromJson(snapshot.data![0] as Map<String, dynamic>);
          final chapters = (snapshot.data![1] as List<dynamic>)
              .map((e) => Chapter.fromJson(e as Map<String, dynamic>))
              .toList();

          return CustomScrollView(
            slivers: [
              SliverAppBar(
                expandedHeight: 300,
                pinned: true,
                flexibleSpace: FlexibleSpaceBar(
                  title: Text(manga.title),
                  background: CachedNetworkImage(
                    imageUrl: client.coverUrl(manga.id),
                    fit: BoxFit.cover,
                    errorWidget: (_, __, ___) => Container(color: Colors.grey[900]),
                  ),
                ),
              ),
              if (manga.description != null)
                SliverToBoxAdapter(
                  child: Padding(
                    padding: const EdgeInsets.all(16),
                    child: Text(manga.description!),
                  ),
                ),
              SliverToBoxAdapter(
                child: Padding(
                  padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
                  child: Text(
                    '${chapters.length} Chapters',
                    style: Theme.of(context).textTheme.titleMedium,
                  ),
                ),
              ),
              SliverList(
                delegate: SliverChildBuilderDelegate(
                  (context, index) {
                    final chapter = chapters[index];
                    return ListTile(
                      title: Text(chapter.displayTitle),
                      subtitle: chapter.volume != null
                          ? Text('Vol. ${chapter.volume!.toStringAsFixed(0)}')
                          : null,
                      trailing: chapter.downloaded
                          ? const Icon(Icons.download_done, color: Colors.green)
                          : const Icon(Icons.cloud_outlined),
                      onTap: chapter.downloaded
                          ? () => context.push('/reader/${chapter.id}')
                          : null,
                    );
                  },
                  childCount: chapters.length,
                ),
              ),
            ],
          );
        },
      ),
    );
  }
}
