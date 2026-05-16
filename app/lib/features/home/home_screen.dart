import 'package:cached_network_image/cached_network_image.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import '../../core/api/api_client.dart';
import '../../core/api/api_service.dart';
import '../../core/models/models.dart';
import '../../core/theme/app_theme.dart';
import '../../shared/utils/platform_utils.dart';
import '../../shared/widgets/manga_card.dart';
import '../../shared/widgets/tv_focusable.dart';

final _libraryProvider = FutureProvider.autoDispose((ref) => svc.listManga());
final _trendingProvider = FutureProvider.autoDispose((ref) => svc.trending());
final _newReleasesProvider = FutureProvider.autoDispose((ref) => svc.newReleases());
final _continueProvider = FutureProvider.autoDispose((ref) => svc.continueReading());

class HomeScreen extends ConsumerWidget {
  const HomeScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final tv = isTV(context);
    final libAsync = ref.watch(_libraryProvider);
    final trendAsync = ref.watch(_trendingProvider);
    final releasesAsync = ref.watch(_newReleasesProvider);
    final continueAsync = ref.watch(_continueProvider);

    final items = libAsync.valueOrNull?.items ?? [];
    final trending = (trendAsync.valueOrNull ?? [])
        .where((r) => !r.inLibrary)
        .take(8)
        .toList();
    final releases = releasesAsync.valueOrNull ?? [];
    final continueItems = continueAsync.valueOrNull ?? [];
    final hero = items.isNotEmpty ? items.first : null;

    return Scaffold(
      body: items.isEmpty && libAsync.isLoading
          ? const Center(child: CircularProgressIndicator(color: kPrimary))
          : items.isEmpty
              ? _EmptyLibrary(onDiscover: () => context.go('/discover'))
              : CustomScrollView(
                  slivers: [
                    if (hero != null)
                      SliverToBoxAdapter(child: _HeroBanner(manga: hero, tv: tv)),

                    // Continue Reading
                    if (continueItems.isNotEmpty) ...[
                      SliverToBoxAdapter(
                        child: Padding(
                          padding: EdgeInsets.fromLTRB(tv ? 40 : 20, 28, tv ? 40 : 20, 12),
                          child: const SectionHeader(title: 'Continue Reading'),
                        ),
                      ),
                      SliverToBoxAdapter(
                        child: SizedBox(
                          height: tv ? 310 : 240,
                          child: ListView.separated(
                            padding: EdgeInsets.symmetric(horizontal: tv ? 40 : 20),
                            scrollDirection: Axis.horizontal,
                            itemCount: continueItems.length,
                            separatorBuilder: (_, __) => const SizedBox(width: 12),
                            itemBuilder: (ctx, i) {
                              final item = continueItems[i];
                              return _ContinueCard(
                                item: item,
                                tv: tv,
                                onPlay: () => ctx.push('/reader/${item.chapterId}'),
                                onDetail: () => ctx.push('/manga/${item.mangaId}'),
                              );
                            },
                          ),
                        ),
                      ),
                    ],

                    // My Library
                    SliverToBoxAdapter(
                      child: Padding(
                        padding: EdgeInsets.fromLTRB(tv ? 40 : 20, 28, tv ? 40 : 20, 12),
                        child: SectionHeader(
                          title: 'My Library',
                          action: 'View All',
                          onAction: () => context.go('/library'),
                        ),
                      ),
                    ),
                    SliverToBoxAdapter(
                      child: SizedBox(
                        height: tv ? 310 : 240,
                        child: ListView.separated(
                          padding: EdgeInsets.symmetric(horizontal: tv ? 40 : 20),
                          scrollDirection: Axis.horizontal,
                          itemCount: items.take(12).length,
                          separatorBuilder: (_, __) => const SizedBox(width: 12),
                          itemBuilder: (ctx, i) {
                            final m = items[i];
                            return MangaCoverCard(
                              title: m.title,
                              mangaId: m.id,
                              coverUrl: m.coverUrl,
                              syncing: m.syncStatus == 'syncing',
                              isTv: tv,
                              onTap: () => ctx.push('/manga/${m.id}'),
                            );
                          },
                        ),
                      ),
                    ),

                    // New Releases
                    if (releases.isNotEmpty) ...[
                      SliverToBoxAdapter(
                        child: Padding(
                          padding: EdgeInsets.fromLTRB(tv ? 40 : 20, 28, tv ? 40 : 20, 12),
                          child: SectionHeader(
                            title: 'New Releases',
                            action: '${releases.length} new',
                          ),
                        ),
                      ),
                      SliverToBoxAdapter(
                        child: SizedBox(
                          height: tv ? 310 : 240,
                          child: ListView.separated(
                            padding: EdgeInsets.symmetric(horizontal: tv ? 40 : 20),
                            scrollDirection: Axis.horizontal,
                            itemCount: releases.length,
                            separatorBuilder: (_, __) => const SizedBox(width: 12),
                            itemBuilder: (ctx, i) {
                              final r = releases[i];
                              return _NewReleaseCard(
                                item: r, tv: tv,
                                onTap: () => ctx.push('/reader/${r.chapterId}'),
                                onMangaTap: () => ctx.push('/manga/${r.mangaId}'),
                              );
                            },
                          ),
                        ),
                      ),
                    ],

                    // Trending Now
                    if (trending.isNotEmpty) ...[
                      SliverToBoxAdapter(
                        child: Padding(
                          padding: EdgeInsets.fromLTRB(tv ? 40 : 20, 28, tv ? 40 : 20, 12),
                          child: const SectionHeader(title: 'Trending Now'),
                        ),
                      ),
                      SliverToBoxAdapter(
                        child: SizedBox(
                          height: tv ? 310 : 240,
                          child: ListView.separated(
                            padding: EdgeInsets.symmetric(horizontal: tv ? 40 : 20),
                            scrollDirection: Axis.horizontal,
                            itemCount: trending.length,
                            separatorBuilder: (_, __) => const SizedBox(width: 12),
                            itemBuilder: (ctx, i) {
                              final r = trending[i];
                              final badges = ['HOT', 'TOP', 'NEW', '#4', '#5', '#6', '#7', '#8'];
                              return ProxiedCoverCard(
                                result: r,
                                isTv: tv,
                                badge: _TrendBadge(label: badges[i]),
                                onTap: () => showDialog(
                                  context: ctx,
                                  builder: (_) => _TrendingModal(result: r),
                                ),
                              );
                            },
                          ),
                        ),
                      ),
                    ],

                    const SliverToBoxAdapter(child: SizedBox(height: 40)),
                  ],
                ),
    );
  }
}

class _TrendBadge extends StatelessWidget {
  final String label;
  const _TrendBadge({required this.label});

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 3),
      decoration: BoxDecoration(
        color: kPrimary,
        borderRadius: BorderRadius.circular(4),
      ),
      child: Text(label,
          style: const TextStyle(
              fontSize: 9, fontWeight: FontWeight.w900, color: Colors.white)),
    );
  }
}

class _HeroBanner extends StatelessWidget {
  final Manga manga;
  final bool tv;
  const _HeroBanner({required this.manga, required this.tv});

  @override
  Widget build(BuildContext context) {
    final imgUrl = manga.coverUrl?.startsWith('http') == true
        ? manga.coverUrl!
        : api.coverUrl(manga.id);
    final h = tv ? 420.0 : 280.0;

    return SizedBox(
      height: h,
      child: Stack(
        fit: StackFit.expand,
        children: [
          CachedNetworkImage(imageUrl: imgUrl, fit: BoxFit.cover,
              errorWidget: (_, __, ___) => Container(color: kMuted)),
          Container(
            decoration: const BoxDecoration(
              gradient: LinearGradient(
                begin: Alignment.centerRight,
                end: Alignment.centerLeft,
                colors: [Colors.transparent, kBackground],
                stops: [0.3, 1.0],
              ),
            ),
          ),
          Container(
            decoration: const BoxDecoration(
              gradient: LinearGradient(
                begin: Alignment.topCenter,
                end: Alignment.bottomCenter,
                colors: [Colors.transparent, Color(0x66121415)],
              ),
            ),
          ),
          Positioned(
            left: tv ? 40 : 20,
            bottom: tv ? 40 : 24,
            right: tv ? 200 : 100,
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              mainAxisSize: MainAxisSize.min,
              children: [
                Text(
                  manga.chaptersRead > 0 ? 'RESUME READING' : 'START READING',
                  style: const TextStyle(
                      fontSize: 10,
                      fontWeight: FontWeight.w800,
                      color: kPrimary,
                      letterSpacing: 2),
                ),
                const SizedBox(height: 6),
                Text(manga.title,
                    style: TextStyle(
                        fontSize: tv ? 40 : 28,
                        fontWeight: FontWeight.w800,
                        color: Colors.white,
                        height: 1.1)),
                if (manga.description != null) ...[
                  const SizedBox(height: 8),
                  Text(manga.description!,
                      maxLines: 2,
                      overflow: TextOverflow.ellipsis,
                      style: const TextStyle(
                          fontSize: 13, color: kMutedFg, height: 1.4)),
                ],
                const SizedBox(height: 16),
                GestureDetector(
                  onTap: () => context.push('/manga/${manga.id}'),
                  child: Container(
                    padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 10),
                    decoration: BoxDecoration(
                      borderRadius: BorderRadius.circular(8),
                      color: kPrimary.withValues(alpha: 0.2),
                      border: Border.all(color: kPrimary.withValues(alpha: 0.4)),
                    ),
                    child: const Text('View Details',
                        style: TextStyle(
                            color: kPrimary,
                            fontWeight: FontWeight.w700,
                            fontSize: 13)),
                  ),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

class _NewReleaseCard extends StatelessWidget {
  final NewRelease item;
  final bool tv;
  final VoidCallback onTap;
  final VoidCallback onMangaTap;

  const _NewReleaseCard({
    required this.item,
    required this.tv,
    required this.onTap,
    required this.onMangaTap,
  });

  @override
  Widget build(BuildContext context) {
    final width = tv ? 180.0 : 128.0;
    final imgUrl = item.coverUrl?.startsWith('http') == true
        ? item.coverUrl!
        : item.coverUrl != null
            ? api.coverUrl(item.mangaId)
            : null;

    final card = GestureDetector(
      onTap: onTap,
      child: SizedBox(
        width: width,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            ClipRRect(
              borderRadius: BorderRadius.circular(12),
              child: AspectRatio(
                aspectRatio: 2 / 3,
                child: Stack(
                  fit: StackFit.expand,
                  children: [
                    imgUrl != null
                        ? CachedNetworkImage(imageUrl: imgUrl, fit: BoxFit.cover)
                        : Container(color: kMuted,
                            child: const Icon(Icons.book, color: kMutedFg)),
                    Positioned(
                      top: 6, left: 6,
                      child: Container(
                        padding: const EdgeInsets.symmetric(
                            horizontal: 6, vertical: 3),
                        decoration: BoxDecoration(
                          color: kPrimary,
                          borderRadius: BorderRadius.circular(4),
                        ),
                        child: Text('Ch.${item.displayNumber}',
                            style: const TextStyle(
                                fontSize: 9,
                                fontWeight: FontWeight.w900,
                                color: Colors.white)),
                      ),
                    ),
                  ],
                ),
              ),
            ),
            const SizedBox(height: 6),
            GestureDetector(
              onTap: onMangaTap,
              child: Text(item.mangaTitle,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: TextStyle(
                      fontSize: tv ? 13 : 12,
                      fontWeight: FontWeight.w600,
                      color: kForeground)),
            ),
            Text(timeAgo(item.chapterCreatedAt),
                style: const TextStyle(fontSize: 10, color: kMutedFg)),
          ],
        ),
      ),
    );
    return TvFocusable(onSelect: onTap, child: card);
  }
}

class _TrendingModal extends ConsumerStatefulWidget {
  final SearchResult result;
  const _TrendingModal({required this.result});

  @override
  ConsumerState<_TrendingModal> createState() => _TrendingModalState();
}

class _TrendingModalState extends ConsumerState<_TrendingModal> {
  MangaDetail? _detail;
  bool _loading = true;
  bool _adding = false;
  bool _added = false;

  @override
  void initState() {
    super.initState();
    _loadDetail();
  }

  Future<void> _loadDetail() async {
    try {
      final d = await svc.discoverDetail(widget.result.source, widget.result.id);
      if (mounted) setState(() { _detail = d; _loading = false; });
    } catch (_) {
      if (mounted) setState(() => _loading = false);
    }
  }

  Future<void> _add() async {
    setState(() => _adding = true);
    try {
      await svc.addManga(widget.result);
      if (mounted) setState(() { _added = true; _adding = false; });
    } catch (e) {
      if (mounted) {
        setState(() => _adding = false);
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('Failed: ${e.toString()}')));
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final r = widget.result;
    final coverUrl = (_detail?.coverUrl ?? r.coverUrl) != null
        ? (r.source == 'mangapill'
            ? api.proxyUrl(_detail?.coverUrl ?? r.coverUrl!)
            : _detail?.coverUrl ?? r.coverUrl!)
        : null;

    return Dialog(
      backgroundColor: kCard,
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(20)),
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 480),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            // Cover hero
            ClipRRect(
              borderRadius: const BorderRadius.vertical(top: Radius.circular(20)),
              child: SizedBox(
                height: 200,
                child: Stack(
                  fit: StackFit.expand,
                  children: [
                    coverUrl != null
                        ? CachedNetworkImage(imageUrl: coverUrl, fit: BoxFit.cover)
                        : Container(color: kMuted),
                    Container(
                      decoration: const BoxDecoration(
                        gradient: LinearGradient(
                          begin: Alignment.topCenter,
                          end: Alignment.bottomCenter,
                          colors: [Colors.transparent, kCard],
                          stops: [0.4, 1.0],
                        ),
                      ),
                    ),
                    Positioned(
                      left: 16, bottom: 12, right: 48,
                      child: Text(r.title,
                          style: const TextStyle(
                              fontSize: 20,
                              fontWeight: FontWeight.w800,
                              color: Colors.white)),
                    ),
                    Positioned(
                      top: 8, right: 8,
                      child: IconButton(
                        icon: const Icon(Icons.close, color: Colors.white70),
                        onPressed: () => Navigator.of(context).pop(),
                      ),
                    ),
                  ],
                ),
              ),
            ),
            Padding(
              padding: const EdgeInsets.all(16),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    children: [
                      if (r.status != 'unknown')
                        _Chip(r.status),
                      if (_detail?.chapterCount != null &&
                          _detail!.chapterCount > 0) ...[
                        const SizedBox(width: 8),
                        _Chip('${_detail!.chapterCount} chapters'),
                      ],
                      if (r.author != null) ...[
                        const SizedBox(width: 8),
                        _Chip(r.author!),
                      ],
                    ],
                  ),
                  const SizedBox(height: 12),
                  if (_loading)
                    const SizedBox(
                        height: 60,
                        child: Center(
                            child: CircularProgressIndicator(color: kPrimary)))
                  else if ((_detail?.description ?? r.description) != null)
                    Text(
                      _detail?.description ?? r.description!,
                      maxLines: 4,
                      overflow: TextOverflow.ellipsis,
                      style:
                          const TextStyle(fontSize: 13, color: kMutedFg, height: 1.5),
                    ),
                  const SizedBox(height: 16),
                  Row(
                    children: [
                      Expanded(
                        child: _added
                            ? OutlinedButton(
                                onPressed: null,
                                style: OutlinedButton.styleFrom(
                                    side: const BorderSide(color: Color(0xFF34d399))),
                                child: const Text('Added to Library',
                                    style: TextStyle(color: Color(0xFF34d399))))
                            : ElevatedButton(
                                onPressed: _adding ? null : _add,
                                child: _adding
                                    ? const SizedBox(
                                        height: 16, width: 16,
                                        child: CircularProgressIndicator(
                                            strokeWidth: 2, color: Colors.white))
                                    : const Text('Add to Library'),
                              ),
                      ),
                      const SizedBox(width: 8),
                      OutlinedButton(
                        onPressed: () => Navigator.of(context).pop(),
                        style: OutlinedButton.styleFrom(
                            side: const BorderSide(color: kBorder)),
                        child: const Text('Close',
                            style: TextStyle(color: kMutedFg)),
                      ),
                    ],
                  ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _Chip extends StatelessWidget {
  final String label;
  const _Chip(this.label);

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
      decoration: BoxDecoration(
        color: kPrimary.withValues(alpha: 0.15),
        border: Border.all(color: kPrimary.withValues(alpha: 0.3)),
        borderRadius: BorderRadius.circular(20),
      ),
      child: Text(label,
          style: const TextStyle(
              fontSize: 11, color: kPrimary, fontWeight: FontWeight.w600)),
    );
  }
}

class _ContinueCard extends StatelessWidget {
  final ContinueItem item;
  final bool tv;
  final VoidCallback onPlay;
  final VoidCallback onDetail;
  const _ContinueCard({required this.item, required this.tv, required this.onPlay, required this.onDetail});

  @override
  Widget build(BuildContext context) {
    final width = tv ? 180.0 : 128.0;
    final imgUrl = item.coverUrl?.startsWith('http') == true
        ? item.coverUrl!
        : item.coverUrl != null ? api.coverUrl(item.mangaId) : null;
    final pct = item.totalChapters > 0 ? item.chaptersRead / item.totalChapters : 0.0;

    final card = GestureDetector(
      onTap: onDetail,
      child: SizedBox(
        width: width,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Expanded(
              child: ClipRRect(
                borderRadius: BorderRadius.circular(12),
                child: Stack(
                  fit: StackFit.expand,
                  children: [
                    imgUrl != null
                        ? CachedNetworkImage(imageUrl: imgUrl, fit: BoxFit.cover)
                        : Container(color: kMuted, child: const Icon(Icons.book, color: kMutedFg)),
                    // Progress bar
                    Positioned(
                      bottom: 0, left: 0, right: 0,
                      child: Column(
                        children: [
                          LinearProgressIndicator(
                            value: pct.toDouble(),
                            backgroundColor: Colors.black38,
                            color: kPrimary,
                            minHeight: 3,
                          ),
                        ],
                      ),
                    ),
                    // Play button
                    Positioned.fill(
                      child: Material(
                        color: Colors.transparent,
                        child: InkWell(
                          onTap: onPlay,
                          child: const Center(
                            child: Icon(Icons.play_circle_fill,
                                size: 40, color: Colors.white70),
                          ),
                        ),
                      ),
                    ),
                  ],
                ),
              ),
            ),
            const SizedBox(height: 6),
            Text(item.mangaTitle,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: TextStyle(
                    fontSize: tv ? 13 : 12,
                    fontWeight: FontWeight.w600,
                    color: kForeground)),
            Text('Ch. ${item.displayNumber}',
                style: const TextStyle(fontSize: 10, color: kPrimary, fontWeight: FontWeight.w600)),
            Text('${item.chaptersRead}/${item.totalChapters} read',
                style: const TextStyle(fontSize: 10, color: kMutedFg)),
          ],
        ),
      ),
    );
    return TvFocusable(onSelect: onPlay, child: card);
  }
}

class _EmptyLibrary extends StatelessWidget {
  final VoidCallback onDiscover;
  const _EmptyLibrary({required this.onDiscover});

  @override
  Widget build(BuildContext context) {
    return Center(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          const Text('Library empty',
              style: TextStyle(color: kMutedFg, fontSize: 16)),
          const SizedBox(height: 16),
          ElevatedButton(
              onPressed: onDiscover, child: const Text('Discover Manga')),
        ],
      ),
    );
  }
}
