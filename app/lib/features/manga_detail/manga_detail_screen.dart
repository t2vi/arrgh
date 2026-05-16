import 'package:cached_network_image/cached_network_image.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import '../../core/api/api_client.dart';
import '../../core/api/api_service.dart';
import '../../core/models/models.dart';
import '../../core/storage/local_chapter_storage.dart';
import '../../core/theme/app_theme.dart';
import '../../shared/utils/platform_utils.dart';
import '../../shared/widgets/manga_card.dart';
import '../../shared/widgets/tv_focusable.dart';

final _mangaProvider = FutureProvider.autoDispose.family<Manga, String>(
    (ref, id) => svc.getManga(id));
final _chaptersProvider = FutureProvider.autoDispose.family<List<Chapter>, String>(
    (ref, id) => svc.listChapters(id));
final _progressProvider = FutureProvider.autoDispose.family<List<ReadProgress>, String>(
    (ref, id) => svc.mangaProgress(id));

final _mangaQueueProvider =
    StreamProvider.autoDispose.family<List<QueueItem>, String>((ref, mangaId) async* {
  while (true) {
    try {
      yield await svc.getMangaQueue(mangaId);
    } catch (_) {
      yield [];
    }
    await Future.delayed(const Duration(seconds: 2));
  }
});

enum _FilterMode { all, downloaded, notDownloaded }
enum _SortDir { desc, asc }

class MangaDetailScreen extends ConsumerStatefulWidget {
  final String mangaId;
  const MangaDetailScreen({super.key, required this.mangaId});

  @override
  ConsumerState<MangaDetailScreen> createState() => _MangaDetailScreenState();
}

class _MangaDetailScreenState extends ConsumerState<MangaDetailScreen>
    with SingleTickerProviderStateMixin {
  _FilterMode _filter = _FilterMode.all;
  _SortDir _sort = _SortDir.desc;
  late TabController _tabController;
  bool _descExpanded = false;

  @override
  void initState() {
    super.initState();
    _tabController = TabController(length: 2, vsync: this);
  }

  @override
  void dispose() {
    _tabController.dispose();
    super.dispose();
  }

  List<Chapter> _applyFilterSort(List<Chapter> chapters) {
    var result = switch (_filter) {
      _FilterMode.downloaded => chapters.where((c) => c.downloaded).toList(),
      _FilterMode.notDownloaded => chapters.where((c) => !c.downloaded).toList(),
      _FilterMode.all => List<Chapter>.from(chapters),
    };
    result.sort((a, b) => _sort == _SortDir.asc
        ? a.number.compareTo(b.number)
        : b.number.compareTo(a.number));
    return result;
  }

  Chapter? _resumeChapter(List<Chapter> all, Map<String, ReadProgress> progressMap) {
    // highest in-progress downloaded chapter
    Chapter? inProgress;
    for (final ch in all) {
      if (!ch.downloaded) continue;
      final p = progressMap[ch.id];
      if (p != null && !p.completed) {
        if (inProgress == null || ch.number > inProgress.number) inProgress = ch;
      }
    }
    if (inProgress != null) return inProgress;

    // lowest unread downloaded chapter
    Chapter? firstUnread;
    for (final ch in all) {
      if (!ch.downloaded) continue;
      final p = progressMap[ch.id];
      if (p == null || !p.completed) {
        if (firstUnread == null || ch.number < firstUnread.number) firstUnread = ch;
      }
    }
    return firstUnread;
  }

  void _sync() async {
    final manga = ref.read(_mangaProvider(widget.mangaId)).valueOrNull;
    if (manga == null) return;
    await svc.syncManga(manga.id);
    ref.invalidate(_mangaProvider(widget.mangaId));
    ref.invalidate(_chaptersProvider(widget.mangaId));
  }

  List<Widget> _appBarActions(BuildContext context, Manga manga) => [
        IconButton(
          icon: const Icon(Icons.sync),
          onPressed: _sync,
          tooltip: 'Sync chapters',
        ),
        PopupMenuButton(
          itemBuilder: (_) => [
            const PopupMenuItem(value: 'remove', child: Text('Remove from library')),
            if (manga.downloadedChapters > 0)
              const PopupMenuItem(
                  value: 'remove_files',
                  child: Text('Remove + delete files',
                      style: TextStyle(color: Colors.redAccent))),
          ],
          onSelected: (v) async {
            final deleteFiles = v == 'remove_files';
            await svc.removeManga(manga.id, deleteFiles: deleteFiles);
            if (context.mounted) context.go('/library');
          },
        ),
      ];

  @override
  Widget build(BuildContext context) {
    final tv = isTV(context);
    final mangaAsync = ref.watch(_mangaProvider(widget.mangaId));
    final chaptersAsync = ref.watch(_chaptersProvider(widget.mangaId));
    final progressAsync = ref.watch(_progressProvider(widget.mangaId));
    final queueItems =
        ref.watch(_mangaQueueProvider(widget.mangaId)).valueOrNull ?? [];

    final manga = mangaAsync.valueOrNull;
    final allChapters = chaptersAsync.valueOrNull ?? [];
    final chapters = _applyFilterSort(allChapters);
    final progressMap = <String, ReadProgress>{
      for (final p in (progressAsync.valueOrNull ?? [])) p.chapterId: p
    };
    final queueMap = <String, QueueItem>{for (final q in queueItems) q.chapterId: q};
    final downloading = queueItems.where((q) => q.status == 'downloading').length;
    final pending = queueItems.where((q) => q.status == 'pending').length;
    final resumeCh = manga != null ? _resumeChapter(allChapters, progressMap) : null;

    if (manga == null && mangaAsync.isLoading) {
      return const Scaffold(
          body: Center(child: CircularProgressIndicator(color: kPrimary)));
    }
    if (manga == null) {
      return Scaffold(
          appBar: AppBar(),
          body: const Center(child: Text('Manga not found')));
    }

    final imgUrl = manga.coverUrl?.startsWith('http') == true
        ? manga.coverUrl!
        : api.coverUrl(manga.id);

    final resumeLabel = resumeCh != null
        ? ((progressMap[resumeCh.id] != null && !progressMap[resumeCh.id]!.completed)
            ? 'Continue Ch. ${resumeCh.displayNumber}'
            : 'Start Ch. ${resumeCh.displayNumber}')
        : null;

    final chaptersTab = _buildChaptersTab(
        context, allChapters, chapters, progressMap, queueMap,
        queueItems, downloading, pending, chaptersAsync.isLoading, tv);
    final infoTab = _buildInfoTab(context, manga, tv);

    if (tv) {
      return Scaffold(
        body: SafeArea(
          child: Column(
            children: [
              _TVHeader(
                manga: manga,
                imgUrl: imgUrl,
                resumeLabel: resumeLabel,
                resumeChapterId: resumeCh?.id,
                downloading: downloading,
                pending: pending,
                actions: _appBarActions(context, manga),
                onBack: () => context.pop(),
              ),
              Padding(
                padding: const EdgeInsets.fromLTRB(32, 4, 32, 8),
                child: Row(
                  children: [
                    GestureDetector(
                      onTap: () => _tabController.animateTo(0),
                      child: TvFocusable(
                        autofocus: true,
                        showFocusDecoration: false,
                        onSelect: () => _tabController.animateTo(0),
                        child: ListenableBuilder(
                          listenable: _tabController,
                          builder: (context, child) =>
                              _TVTabPill('Chapters', _tabController.index == 0),
                        ),
                      ),
                    ),
                    const SizedBox(width: 12),
                    GestureDetector(
                      onTap: () => _tabController.animateTo(1),
                      child: TvFocusable(
                        showFocusDecoration: false,
                        onSelect: () => _tabController.animateTo(1),
                        child: ListenableBuilder(
                          listenable: _tabController,
                          builder: (context, child) =>
                              _TVTabPill('Info', _tabController.index == 1),
                        ),
                      ),
                    ),
                  ],
                ),
              ),
              Expanded(
                child: TabBarView(
                  controller: _tabController,
                  physics: const NeverScrollableScrollPhysics(),
                  children: [chaptersTab, infoTab],
                ),
              ),
            ],
          ),
        ),
      );
    }

    // Phone layout
    return Scaffold(
      body: NestedScrollView(
        headerSliverBuilder: (ctx, _) => [
          SliverAppBar(
            expandedHeight: 240,
            pinned: true,
            leading: IconButton(
              icon: const Icon(Icons.chevron_left),
              onPressed: () => context.pop(),
            ),
            actions: _appBarActions(context, manga),
            flexibleSpace: FlexibleSpaceBar(
              background: Stack(
                fit: StackFit.expand,
                children: [
                  CachedNetworkImage(
                      imageUrl: imgUrl,
                      fit: BoxFit.cover,
                      errorWidget: (_, __, ___) => Container(color: kMuted)),
                  Container(
                    decoration: const BoxDecoration(
                      gradient: LinearGradient(
                        begin: Alignment.topCenter,
                        end: Alignment.bottomCenter,
                        colors: [Colors.transparent, kBackground],
                        stops: [0.4, 1.0],
                      ),
                    ),
                  ),
                ],
              ),
            ),
          ),
          SliverToBoxAdapter(
            child: Padding(
              padding: const EdgeInsets.fromLTRB(20, 16, 20, 8),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(manga.title,
                      style: const TextStyle(
                          fontSize: 24, fontWeight: FontWeight.w800)),
                  const SizedBox(height: 6),
                  ContentTypePill(manga.contentType),
                  if (manga.author != null)
                    Padding(
                      padding: const EdgeInsets.only(top: 4),
                      child: Text(manga.author!,
                          style: const TextStyle(color: kMutedFg, fontSize: 14)),
                    ),
                  const SizedBox(height: 16),
                  _StatsGrid(manga: manga, downloading: downloading, pending: pending),
                  if (resumeLabel != null) ...[
                    const SizedBox(height: 12),
                    SizedBox(
                      width: double.infinity,
                      child: ElevatedButton.icon(
                        style: ElevatedButton.styleFrom(
                          backgroundColor: kPrimary,
                          foregroundColor: Colors.white,
                          padding: const EdgeInsets.symmetric(vertical: 12),
                          shape: RoundedRectangleBorder(
                              borderRadius: BorderRadius.circular(10)),
                          elevation: 0,
                        ),
                        icon: const Icon(Icons.play_arrow, size: 18),
                        label: Text(resumeLabel,
                            style: const TextStyle(fontWeight: FontWeight.w700)),
                        onPressed: () => context.push('/reader/${resumeCh!.id}'),
                      ),
                    ),
                  ],
                ],
              ),
            ),
          ),
          SliverPersistentHeader(
            pinned: true,
            delegate: _TabBarDelegate(
              TabBar(
                controller: _tabController,
                tabs: const [Tab(text: 'Chapters'), Tab(text: 'Info')],
                indicatorColor: kPrimary,
                labelColor: kPrimary,
                unselectedLabelColor: kMutedFg,
                dividerColor: kBorder,
                labelStyle:
                    const TextStyle(fontWeight: FontWeight.w600, fontSize: 13),
              ),
            ),
          ),
        ],
        body: TabBarView(
          controller: _tabController,
          children: [chaptersTab, infoTab],
        ),
      ),
    );
  }

  Widget _buildChaptersTab(
    BuildContext context,
    List<Chapter> allChapters,
    List<Chapter> chapters,
    Map<String, ReadProgress> progressMap,
    Map<String, QueueItem> queueMap,
    List<QueueItem> queueItems,
    int downloading,
    int pending,
    bool isLoading,
    bool isTV,
  ) {
    if (isLoading) {
      return const Center(child: CircularProgressIndicator(color: kPrimary));
    }
    final pad = isTV ? 32.0 : 20.0;
    return ListView.builder(
      primary: false,
      padding: EdgeInsets.zero,
      itemCount: chapters.length + 2,
      itemBuilder: (ctx, i) {
        if (i == 0) {
          return Padding(
            padding: EdgeInsets.fromLTRB(pad, 12, pad, 0),
            child: Row(
              children: [
                _FilterPill('All', _filter == _FilterMode.all,
                    () => setState(() => _filter = _FilterMode.all)),
                const SizedBox(width: 6),
                _FilterPill('Downloaded', _filter == _FilterMode.downloaded,
                    () => setState(() => _filter = _FilterMode.downloaded)),
                const SizedBox(width: 6),
                _FilterPill('Not Downloaded', _filter == _FilterMode.notDownloaded,
                    () => setState(() => _filter = _FilterMode.notDownloaded)),
                const Spacer(),
                IconButton(
                  icon: Icon(
                    _sort == _SortDir.desc ? Icons.arrow_downward : Icons.arrow_upward,
                    size: 18,
                    color: kMutedFg,
                  ),
                  tooltip: _sort == _SortDir.desc ? 'Newest first' : 'Oldest first',
                  onPressed: () => setState(() {
                    _sort = _sort == _SortDir.desc ? _SortDir.asc : _SortDir.desc;
                  }),
                  padding: const EdgeInsets.all(4),
                  constraints: const BoxConstraints(),
                ),
              ],
            ),
          );
        }
        if (i == 1) {
          if (allChapters.isNotEmpty && chapters.isEmpty) {
            return Padding(
              padding: EdgeInsets.fromLTRB(pad, 12, pad, 0),
              child: const Text('No chapters match this filter.',
                  style: TextStyle(color: kMutedFg, fontSize: 13)),
            );
          }
          if (downloading > 0 || pending > 0) {
            return Padding(
              padding: EdgeInsets.fromLTRB(pad, 12, pad, 0),
              child: _QueueBanner(
                  downloading: downloading,
                  pending: pending,
                  queueItems: queueItems),
            );
          }
          return const SizedBox(height: 4);
        }
        final ch = chapters[i - 2];
        return _ChapterRow(
          chapter: ch,
          progress: progressMap[ch.id],
          isTV: isTV,
          mangaId: widget.mangaId,
          queueItem: queueMap[ch.id],
        );
      },
    );
  }

  Widget _buildInfoTab(BuildContext context, Manga manga, bool isTV) {
    final pad = isTV ? 32.0 : 20.0;
    return ListView(
      primary: false,
      padding: EdgeInsets.fromLTRB(pad, 16, pad, 24),
      children: [
        if (manga.description != null) ...[
          _ExpandableDescription(
            text: manga.description!,
            expanded: _descExpanded,
            onToggle: () => setState(() => _descExpanded = !_descExpanded),
          ),
          const SizedBox(height: 12),
        ],
        if (manga.tags != null) ...[
          Wrap(
            spacing: 6,
            runSpacing: 6,
            children: manga.tags!
                .split(', ')
                .take(8)
                .map((t) => _Chip(t, small: true))
                .toList(),
          ),
          const SizedBox(height: 16),
        ],
        if (manga.source != 'local') ...[
          _AutoDownloadCard(
            mangaId: manga.id,
            value: manga.autoDownload,
            onChanged: (v) async {
              await svc.setMangaAutoDownload(manga.id, v);
              ref.invalidate(_mangaProvider(widget.mangaId));
            },
          ),
          const SizedBox(height: 12),
        ],
        _ReaderModeCard(
          mangaId: manga.id,
          value: manga.readerMode,
          onChanged: (v) async {
            await svc.setMangaReaderMode(manga.id, v);
            ref.invalidate(_mangaProvider(widget.mangaId));
          },
        ),
        const SizedBox(height: 12),
        _DownloadDirCard(
          mangaId: manga.id,
          value: manga.downloadDir,
          onChanged: (v) async {
            await svc.setMangaDownloadDir(manga.id, v);
            ref.invalidate(_mangaProvider(widget.mangaId));
          },
        ),
      ],
    );
  }
}

// ── Layout helpers ────────────────────────────────────────────────────────────

class _TabBarDelegate extends SliverPersistentHeaderDelegate {
  final TabBar _tabBar;
  _TabBarDelegate(this._tabBar);

  @override
  double get minExtent => _tabBar.preferredSize.height;
  @override
  double get maxExtent => _tabBar.preferredSize.height;

  @override
  Widget build(BuildContext context, double shrinkOffset, bool overlapsContent) =>
      Container(color: kBackground, child: _tabBar);

  @override
  bool shouldRebuild(covariant _TabBarDelegate old) => false;
}

class _TVTabPill extends StatelessWidget {
  final String label;
  final bool selected;
  const _TVTabPill(this.label, this.selected);

  @override
  Widget build(BuildContext context) {
    return AnimatedContainer(
      duration: const Duration(milliseconds: 150),
      padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 8),
      decoration: BoxDecoration(
        color: selected ? kPrimary : kMuted,
        borderRadius: BorderRadius.circular(20),
        boxShadow: selected
            ? [BoxShadow(color: kPrimary.withValues(alpha: 0.4), blurRadius: 12)]
            : null,
      ),
      child: Text(
        label,
        style: TextStyle(
          fontSize: 14,
          fontWeight: FontWeight.w600,
          color: selected ? Colors.white : kMutedFg,
        ),
      ),
    );
  }
}

class _TVHeader extends StatelessWidget {
  final Manga manga;
  final String imgUrl;
  final String? resumeLabel;
  final String? resumeChapterId;
  final int downloading;
  final int pending;
  final List<Widget> actions;
  final VoidCallback onBack;

  const _TVHeader({
    required this.manga,
    required this.imgUrl,
    required this.resumeLabel,
    required this.resumeChapterId,
    required this.downloading,
    required this.pending,
    required this.actions,
    required this.onBack,
  });

  @override
  Widget build(BuildContext context) {
    return Container(
      color: kBackground,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              IconButton(
                icon: const Icon(Icons.chevron_left),
                onPressed: onBack,
              ),
              const Spacer(),
              ...actions,
            ],
          ),
          Padding(
            padding: const EdgeInsets.fromLTRB(32, 0, 32, 12),
            child: Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                ClipRRect(
                  borderRadius: BorderRadius.circular(8),
                  child: CachedNetworkImage(
                    imageUrl: imgUrl,
                    width: 72,
                    height: 108,
                    fit: BoxFit.cover,
                    errorWidget: (_, __, ___) =>
                        Container(width: 72, height: 108, color: kMuted),
                  ),
                ),
                const SizedBox(width: 20),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(manga.title,
                          style: const TextStyle(
                              fontSize: 28, fontWeight: FontWeight.w800),
                          maxLines: 2,
                          overflow: TextOverflow.ellipsis),
                      const SizedBox(height: 4),
                      ContentTypePill(manga.contentType),
                      if (manga.author != null)
                        Padding(
                          padding: const EdgeInsets.only(top: 4),
                          child: Text(manga.author!,
                              style: const TextStyle(
                                  color: kMutedFg, fontSize: 13)),
                        ),
                      const SizedBox(height: 12),
                      _StatsGrid(
                          manga: manga,
                          downloading: downloading,
                          pending: pending),
                      if (resumeLabel != null) ...[
                        const SizedBox(height: 10),
                        SizedBox(
                          width: 260,
                          child: ElevatedButton.icon(
                            style: ElevatedButton.styleFrom(
                              backgroundColor: kPrimary,
                              foregroundColor: Colors.white,
                              padding:
                                  const EdgeInsets.symmetric(vertical: 10),
                              shape: RoundedRectangleBorder(
                                  borderRadius: BorderRadius.circular(10)),
                              elevation: 0,
                            ),
                            icon: const Icon(Icons.play_arrow, size: 18),
                            label: Text(resumeLabel!,
                                style: const TextStyle(
                                    fontWeight: FontWeight.w700)),
                            onPressed: () =>
                                context.push('/reader/$resumeChapterId'),
                          ),
                        ),
                      ],
                    ],
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

class _StatsGrid extends StatelessWidget {
  final Manga manga;
  final int downloading;
  final int pending;
  const _StatsGrid(
      {required this.manga, required this.downloading, required this.pending});

  @override
  Widget build(BuildContext context) {
    return Container(
      decoration: BoxDecoration(
        color: kCard,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: kBorder),
      ),
      child: Column(
        children: [
          if (manga.totalChapters > 0) ...[
            Padding(
              padding: const EdgeInsets.fromLTRB(14, 12, 14, 10),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    children: [
                      const Text('READING PROGRESS',
                          style: TextStyle(
                              fontSize: 10,
                              color: kMutedFg,
                              fontWeight: FontWeight.w700,
                              letterSpacing: 0.8)),
                      const Spacer(),
                      Text(
                        '${((manga.chaptersRead / manga.totalChapters) * 100).round()}%',
                        style: const TextStyle(
                            fontSize: 12, fontWeight: FontWeight.w700),
                      ),
                    ],
                  ),
                  const SizedBox(height: 6),
                  ClipRRect(
                    borderRadius: BorderRadius.circular(4),
                    child: LinearProgressIndicator(
                      value: manga.chaptersRead / manga.totalChapters,
                      backgroundColor: kMuted,
                      color: kPrimary,
                      minHeight: 4,
                    ),
                  ),
                  if (manga.chaptersRead > 0) ...[
                    const SizedBox(height: 4),
                    Text(
                        '${manga.chaptersRead} of ${manga.totalChapters} completed',
                        style: const TextStyle(fontSize: 11, color: kMutedFg)),
                  ],
                ],
              ),
            ),
            const Divider(height: 1, color: kBorder),
          ],
          IntrinsicHeight(
            child: Row(
              children: [
                _StatCell('TOTAL', '${manga.totalChapters} Ch'),
                const VerticalDivider(width: 1, color: kBorder),
                _StatCell('DOWNLOADED', '${manga.downloadedChapters} Ch',
                    valueColor: manga.downloadedChapters > 0
                        ? const Color(0xFF34d399)
                        : null),
                const VerticalDivider(width: 1, color: kBorder),
                _StatCell('SOURCE',
                    manga.source.isEmpty ? '—' : manga.source,
                    capitalize: true),
                if (manga.year != null) ...[
                  const VerticalDivider(width: 1, color: kBorder),
                  _StatCell('YEAR', '${manga.year}'),
                ],
              ],
            ),
          ),
          if (downloading > 0 || pending > 0) ...[
            const Divider(height: 1, color: kBorder),
            IntrinsicHeight(
              child: Row(
                children: [
                  _StatCell('DOWNLOADING', '$downloading Ch',
                      valueColor: downloading > 0 ? kPrimary : null,
                      icon: downloading > 0 ? Icons.downloading : null),
                  const VerticalDivider(width: 1, color: kBorder),
                  _StatCell('QUEUED', '$pending Ch',
                      valueColor: pending > 0 ? kMutedFg : null,
                      icon: pending > 0 ? Icons.schedule : null),
                ],
              ),
            ),
          ],
        ],
      ),
    );
  }
}

class _QueueBanner extends StatelessWidget {
  final int downloading;
  final int pending;
  final List<QueueItem> queueItems;
  const _QueueBanner(
      {required this.downloading,
      required this.pending,
      required this.queueItems});

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 10),
      decoration: BoxDecoration(
        color: kPrimary.withValues(alpha: 0.1),
        borderRadius: BorderRadius.circular(10),
        border: Border.all(color: kPrimary.withValues(alpha: 0.25)),
      ),
      child: Row(
        children: [
          const SizedBox(
            width: 14,
            height: 14,
            child: CircularProgressIndicator(strokeWidth: 2, color: kPrimary),
          ),
          const SizedBox(width: 10),
          Expanded(
            child: Text(
              [
                if (downloading > 0) '$downloading downloading',
                if (pending > 0) '$pending queued',
              ].join(' · '),
              style: const TextStyle(
                  fontSize: 12, color: kPrimary, fontWeight: FontWeight.w600),
            ),
          ),
          GestureDetector(
            onTap: () async {
              for (final q in queueItems.where(
                  (q) => q.status == 'pending' || q.status == 'downloading')) {
                await svc.removeFromQueue(q.id).catchError((_) {});
              }
            },
            child: const Text('Cancel all',
                style: TextStyle(fontSize: 11, color: kMutedFg)),
          ),
        ],
      ),
    );
  }
}

class _ExpandableDescription extends StatelessWidget {
  final String text;
  final bool expanded;
  final VoidCallback onToggle;
  const _ExpandableDescription(
      {required this.text, required this.expanded, required this.onToggle});

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        AnimatedCrossFade(
          duration: const Duration(milliseconds: 200),
          crossFadeState:
              expanded ? CrossFadeState.showSecond : CrossFadeState.showFirst,
          firstChild: Text(text,
              style:
                  const TextStyle(color: kMutedFg, fontSize: 14, height: 1.5),
              maxLines: 4,
              overflow: TextOverflow.ellipsis),
          secondChild: Text(text,
              style:
                  const TextStyle(color: kMutedFg, fontSize: 14, height: 1.5)),
        ),
        const SizedBox(height: 4),
        GestureDetector(
          onTap: onToggle,
          child: Text(
            expanded ? 'Show less' : 'Show more',
            style: const TextStyle(
                fontSize: 12, color: kPrimary, fontWeight: FontWeight.w600),
          ),
        ),
      ],
    );
  }
}

// ── Chapter row ───────────────────────────────────────────────────────────────

class _ChapterRow extends ConsumerStatefulWidget {
  final Chapter chapter;
  final ReadProgress? progress;
  final bool isTV;
  final String mangaId;
  final QueueItem? queueItem;

  const _ChapterRow({
    required this.chapter,
    required this.progress,
    required this.isTV,
    required this.mangaId,
    this.queueItem,
  });

  @override
  ConsumerState<_ChapterRow> createState() => _ChapterRowState();
}

class _ChapterRowState extends ConsumerState<_ChapterRow> {
  bool _locallyAvailable = false;
  bool _downloading = false;
  int _downloadProgress = 0;
  bool _waitingForDownload = false;

  @override
  void initState() {
    super.initState();
    _checkLocal();
  }

  @override
  void didUpdateWidget(_ChapterRow oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (_waitingForDownload) {
      final qi = widget.queueItem;
      final ready =
          widget.chapter.downloaded || _locallyAvailable || qi?.status == 'done';
      if (ready) {
        _waitingForDownload = false;
        WidgetsBinding.instance.addPostFrameCallback((_) {
          if (mounted) context.push('/reader/${widget.chapter.id}');
        });
      } else if (qi?.status == 'error') {
        _waitingForDownload = false;
      }
    }
  }

  Future<void> _checkLocal() async {
    if (widget.chapter.pageCount == 0) return;
    final available = await localChapters.isFullyDownloaded(
        widget.chapter.id, widget.chapter.pageCount);
    if (mounted) setState(() => _locallyAvailable = available);
  }

  Future<void> _downloadLocally() async {
    if (_downloading || widget.chapter.pageCount == 0) return;
    setState(() {
      _downloading = true;
      _downloadProgress = 0;
    });
    try {
      await localChapters.downloadChapter(
        widget.chapter.id,
        widget.chapter.pageCount,
        onProgress: (done, total) {
          if (mounted) setState(() => _downloadProgress = done);
        },
      );
      if (mounted) {
        setState(() {
          _locallyAvailable = true;
          _downloading = false;
        });
      }
    } catch (e) {
      if (mounted) {
        setState(() => _downloading = false);
        ScaffoldMessenger.of(context)
            .showSnackBar(SnackBar(content: Text('Download failed: $e')));
      }
    }
  }

  Future<void> _deleteLocal() async {
    await localChapters.deleteChapter(widget.chapter.id);
    if (mounted) setState(() => _locallyAvailable = false);
  }

  void _onTap(BuildContext context, Chapter ch) {
    if (ch.downloaded || _locallyAvailable) {
      context.push('/reader/${ch.id}');
      return;
    }
    if (ch.sourceId == null) return;
    final qi = widget.queueItem;
    if (qi?.status == 'error' || qi == null) {
      svc.downloadChapter(ch.id).then((_) {
        ref.invalidate(_chaptersProvider(widget.mangaId));
      }).catchError((_) {});
    }
    setState(() => _waitingForDownload = true);
  }

  @override
  Widget build(BuildContext context) {
    final ch = widget.chapter;
    final prog = widget.progress;
    final read = prog?.completed == true;
    final inProgress = prog != null && !read;

    return InkWell(
      onTap: () => _onTap(context, ch),
      child: Container(
        padding: EdgeInsets.symmetric(
            horizontal: widget.isTV ? 32 : 20, vertical: 14),
        decoration: const BoxDecoration(
            border: Border(bottom: BorderSide(color: kBorder, width: 0.5))),
        child: Row(
          children: [
            SizedBox(
              width: 24,
              child: Icon(
                read
                    ? Icons.check_circle
                    : inProgress
                        ? Icons.play_circle_outline
                        : Icons.circle_outlined,
                size: 18,
                color: read
                    ? const Color(0xFF34d399)
                    : inProgress
                        ? kPrimary
                        : kBorder,
              ),
            ),
            const SizedBox(width: 12),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    ch.title ?? 'Chapter ${ch.displayNumber}',
                    style: TextStyle(
                        fontSize: widget.isTV ? 15 : 14,
                        color: read ? kMutedFg : kForeground,
                        fontWeight: read ? FontWeight.normal : FontWeight.w500),
                  ),
                  if (inProgress)
                    Text('Page ${prog.currentPage + 1}',
                        style: const TextStyle(fontSize: 11, color: kPrimary)),
                  if (_waitingForDownload)
                    const Text('Opening when ready…',
                        style: TextStyle(fontSize: 11, color: kPrimary)),
                ],
              ),
            ),
            Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                if (ch.downloaded)
                  const Padding(
                    padding: EdgeInsets.symmetric(horizontal: 4),
                    child: Icon(Icons.download_done,
                        size: 16, color: Color(0xFF34d399)),
                  )
                else if (widget.queueItem != null)
                  _QueueStatusBadge(
                    status: widget.queueItem!.status,
                    onCancel: () => svc.removeFromQueue(widget.queueItem!.id),
                  )
                else
                  IconButton(
                    icon: const Icon(Icons.download_outlined,
                        size: 18, color: kMutedFg),
                    tooltip: 'Download on server',
                    onPressed: () async {
                      await svc.downloadChapter(ch.id);
                      ref.invalidate(_chaptersProvider(widget.mangaId));
                    },
                    padding: const EdgeInsets.all(4),
                    constraints: const BoxConstraints(),
                  ),
                if (ch.pageCount > 0) ...[
                  const SizedBox(width: 4),
                  if (_downloading)
                    Padding(
                      padding: const EdgeInsets.symmetric(horizontal: 8),
                      child: Text('$_downloadProgress/${ch.pageCount}',
                          style: const TextStyle(
                              fontSize: 11, color: Colors.white54)),
                    )
                  else if (_locallyAvailable)
                    IconButton(
                      icon: const Icon(Icons.phone_iphone,
                          size: 18, color: Color(0xFF34d399)),
                      tooltip: 'Saved on device — tap to remove',
                      onPressed: _deleteLocal,
                      padding: const EdgeInsets.all(4),
                      constraints: const BoxConstraints(),
                    )
                  else
                    IconButton(
                      icon: const Icon(Icons.phone_android_outlined,
                          size: 18, color: kMutedFg),
                      tooltip: 'Save to device',
                      onPressed: _downloadLocally,
                      padding: const EdgeInsets.all(4),
                      constraints: const BoxConstraints(),
                    ),
                ],
                const SizedBox(width: 4),
                const Icon(Icons.chevron_right, color: kMutedFg, size: 18),
              ],
            ),
          ],
        ),
      ),
    );
  }
}

// ── Small reusable widgets ────────────────────────────────────────────────────

class _FilterPill extends StatelessWidget {
  final String label;
  final bool selected;
  final VoidCallback onTap;
  const _FilterPill(this.label, this.selected, this.onTap);

  @override
  Widget build(BuildContext context) {
    return GestureDetector(
      onTap: onTap,
      child: AnimatedContainer(
        duration: const Duration(milliseconds: 150),
        padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 5),
        decoration: BoxDecoration(
          color: selected ? kPrimary : kMuted,
          borderRadius: BorderRadius.circular(20),
        ),
        child: Text(label,
            style: TextStyle(
                fontSize: 11,
                fontWeight: FontWeight.w600,
                color: selected ? Colors.white : kMutedFg)),
      ),
    );
  }
}

class _Chip extends StatelessWidget {
  final String label;
  final Color? color;
  final bool small;
  final IconData? icon;
  const _Chip(this.label, {this.color, this.small = false, this.icon});

  @override
  Widget build(BuildContext context) {
    final fg = color ?? kMutedFg;
    return Container(
      padding: EdgeInsets.symmetric(
          horizontal: small ? 6 : 10, vertical: small ? 2 : 4),
      decoration: BoxDecoration(color: kMuted, borderRadius: BorderRadius.circular(20)),
      child: icon != null
          ? Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                Icon(icon, size: small ? 10 : 12, color: fg),
                const SizedBox(width: 4),
                Text(label,
                    style: TextStyle(
                        fontSize: small ? 10 : 12,
                        color: fg,
                        fontWeight: FontWeight.w500)),
              ],
            )
          : Text(label,
              style: TextStyle(
                  fontSize: small ? 10 : 12,
                  color: fg,
                  fontWeight: FontWeight.w500)),
    );
  }
}

class _QueueStatusBadge extends StatelessWidget {
  final String status;
  final VoidCallback onCancel;
  const _QueueStatusBadge({required this.status, required this.onCancel});

  @override
  Widget build(BuildContext context) {
    if (status == 'downloading') {
      return const Padding(
        padding: EdgeInsets.symmetric(horizontal: 6),
        child: SizedBox(
          width: 14,
          height: 14,
          child: CircularProgressIndicator(strokeWidth: 2, color: kPrimary),
        ),
      );
    }
    if (status == 'error') {
      return const Padding(
        padding: EdgeInsets.symmetric(horizontal: 4),
        child: Icon(Icons.error_outline, size: 16, color: Colors.redAccent),
      );
    }
    return GestureDetector(
      onTap: onCancel,
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 4),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: const [
            Icon(Icons.schedule, size: 14, color: kMutedFg),
            SizedBox(width: 3),
            Text('Queued', style: TextStyle(fontSize: 10, color: kMutedFg)),
          ],
        ),
      ),
    );
  }
}

class _StatCell extends StatelessWidget {
  final String label;
  final String value;
  final Color? valueColor;
  final bool capitalize;
  final IconData? icon;
  const _StatCell(this.label, this.value,
      {this.valueColor, this.capitalize = false, this.icon});

  @override
  Widget build(BuildContext context) {
    final color = valueColor ?? kForeground;
    final display = capitalize && value.isNotEmpty
        ? value[0].toUpperCase() + value.substring(1)
        : value;
    return Expanded(
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(label,
                style: const TextStyle(
                    fontSize: 9,
                    color: kMutedFg,
                    fontWeight: FontWeight.w700,
                    letterSpacing: 0.8)),
            const SizedBox(height: 3),
            Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                if (icon != null) ...[
                  Icon(icon, size: 12, color: color),
                  const SizedBox(width: 4),
                ],
                Flexible(
                  child: Text(display,
                      style: TextStyle(
                          fontSize: 13,
                          fontWeight: FontWeight.w700,
                          color: color),
                      overflow: TextOverflow.ellipsis),
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }
}

// ── Settings cards ────────────────────────────────────────────────────────────

class _ReaderModeCard extends StatelessWidget {
  final String mangaId;
  final String? value;
  final void Function(String?) onChanged;
  const _ReaderModeCard(
      {required this.mangaId, required this.value, required this.onChanged});

  @override
  Widget build(BuildContext context) {
    final options = <(String, String?)>[
      ('Global', null),
      ('Paged', 'paged'),
      ('Scroll', 'scroll'),
    ];
    return Container(
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: kCard,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: kBorder),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          const Text('READER MODE',
              style: TextStyle(
                  fontSize: 9,
                  color: kMutedFg,
                  fontWeight: FontWeight.w700,
                  letterSpacing: 0.8)),
          const SizedBox(height: 10),
          Row(
            children: options.map(((String, String?) opt) {
              final (label, v) = opt;
              final selected = value == v;
              return Expanded(
                child: GestureDetector(
                  onTap: () => onChanged(v),
                  child: AnimatedContainer(
                    duration: const Duration(milliseconds: 150),
                    padding: const EdgeInsets.symmetric(vertical: 7),
                    margin: const EdgeInsets.only(right: 4),
                    decoration: BoxDecoration(
                      color: selected ? kPrimary : kMuted,
                      borderRadius: BorderRadius.circular(8),
                    ),
                    alignment: Alignment.center,
                    child: Text(label,
                        style: TextStyle(
                            fontSize: 12,
                            fontWeight: FontWeight.w600,
                            color: selected ? Colors.white : kMutedFg)),
                  ),
                ),
              );
            }).toList(),
          ),
          const SizedBox(height: 8),
          Text(
            value == null
                ? 'Uses the global reader mode setting.'
                : value == 'paged'
                    ? 'One page at a time, tap to advance.'
                    : 'All pages in a continuous vertical scroll.',
            style: const TextStyle(fontSize: 11, color: kMutedFg),
          ),
        ],
      ),
    );
  }
}

class _AutoDownloadCard extends StatelessWidget {
  final String mangaId;
  final bool? value;
  final void Function(bool?) onChanged;
  const _AutoDownloadCard(
      {required this.mangaId, required this.value, required this.onChanged});

  @override
  Widget build(BuildContext context) {
    final options = <(String, bool?)>[
      ('Global', null),
      ('Always', true),
      ('Never', false),
    ];
    return Container(
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: kCard,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: kBorder),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          const Text('AUTO-DOWNLOAD',
              style: TextStyle(
                  fontSize: 9,
                  color: kMutedFg,
                  fontWeight: FontWeight.w700,
                  letterSpacing: 0.8)),
          const SizedBox(height: 10),
          Row(
            children: options.map(((String, bool?) opt) {
              final (label, v) = opt;
              final selected = value == v;
              return Expanded(
                child: GestureDetector(
                  onTap: () => onChanged(v),
                  child: AnimatedContainer(
                    duration: const Duration(milliseconds: 150),
                    padding: const EdgeInsets.symmetric(vertical: 7),
                    margin: const EdgeInsets.only(right: 4),
                    decoration: BoxDecoration(
                      color: selected ? kPrimary : kMuted,
                      borderRadius: BorderRadius.circular(8),
                    ),
                    alignment: Alignment.center,
                    child: Text(label,
                        style: TextStyle(
                            fontSize: 12,
                            fontWeight: FontWeight.w600,
                            color: selected ? Colors.white : kMutedFg)),
                  ),
                ),
              );
            }).toList(),
          ),
          const SizedBox(height: 8),
          Text(
            value == null
                ? 'Follows the global auto-download setting.'
                : value!
                    ? 'New chapters download automatically.'
                    : 'New chapters are never auto-downloaded.',
            style: const TextStyle(fontSize: 11, color: kMutedFg),
          ),
        ],
      ),
    );
  }
}

class _DownloadDirCard extends StatefulWidget {
  final String mangaId;
  final String? value;
  final Future<void> Function(String?) onChanged;
  const _DownloadDirCard(
      {required this.mangaId, required this.value, required this.onChanged});

  @override
  State<_DownloadDirCard> createState() => _DownloadDirCardState();
}

class _DownloadDirCardState extends State<_DownloadDirCard> {
  late TextEditingController _ctrl;
  bool _saving = false;

  @override
  void initState() {
    super.initState();
    _ctrl = TextEditingController(text: widget.value ?? '');
  }

  @override
  void didUpdateWidget(_DownloadDirCard old) {
    super.didUpdateWidget(old);
    if (old.value != widget.value && !_ctrl.text.startsWith('/')) {
      _ctrl.text = widget.value ?? '';
    }
  }

  @override
  void dispose() {
    _ctrl.dispose();
    super.dispose();
  }

  Future<void> _save() async {
    setState(() => _saving = true);
    final v = _ctrl.text.trim();
    await widget.onChanged(v.isEmpty ? null : v);
    if (mounted) setState(() => _saving = false);
  }

  Future<void> _clear() async {
    _ctrl.clear();
    await widget.onChanged(null);
  }

  @override
  Widget build(BuildContext context) {
    final isDirty = _ctrl.text.trim() != (widget.value ?? '');
    return Container(
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: kCard,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: kBorder),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          const Text('DOWNLOAD PATH',
              style: TextStyle(
                  fontSize: 9,
                  color: kMutedFg,
                  fontWeight: FontWeight.w700,
                  letterSpacing: 0.8)),
          const SizedBox(height: 10),
          Row(
            children: [
              Expanded(
                child: TextField(
                  controller: _ctrl,
                  onChanged: (_) => setState(() {}),
                  onSubmitted: (_) => _save(),
                  style: const TextStyle(fontSize: 12, color: kForeground),
                  decoration: InputDecoration(
                    hintText: '/path/to/manga',
                    hintStyle: const TextStyle(fontSize: 12, color: kMutedFg),
                    filled: true,
                    fillColor: kMuted,
                    contentPadding:
                        const EdgeInsets.symmetric(horizontal: 10, vertical: 8),
                    border: OutlineInputBorder(
                      borderRadius: BorderRadius.circular(8),
                      borderSide: BorderSide.none,
                    ),
                    focusedBorder: OutlineInputBorder(
                      borderRadius: BorderRadius.circular(8),
                      borderSide: const BorderSide(color: kPrimary),
                    ),
                  ),
                ),
              ),
              if (isDirty) ...[
                const SizedBox(width: 8),
                GestureDetector(
                  onTap: _saving ? null : _save,
                  child: Container(
                    padding:
                        const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
                    decoration: BoxDecoration(
                      color: kPrimary,
                      borderRadius: BorderRadius.circular(8),
                    ),
                    child: _saving
                        ? const SizedBox(
                            width: 14,
                            height: 14,
                            child: CircularProgressIndicator(
                                strokeWidth: 2, color: Colors.white))
                        : const Text('Save',
                            style: TextStyle(
                                fontSize: 12,
                                fontWeight: FontWeight.w600,
                                color: Colors.white)),
                  ),
                ),
              ] else if (widget.value != null) ...[
                const SizedBox(width: 8),
                GestureDetector(
                  onTap: _clear,
                  child: Container(
                    padding:
                        const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
                    decoration: BoxDecoration(
                      color: kMuted,
                      borderRadius: BorderRadius.circular(8),
                      border: Border.all(color: kBorder),
                    ),
                    child: const Text('Reset',
                        style: TextStyle(fontSize: 12, color: kMutedFg)),
                  ),
                ),
              ],
            ],
          ),
          const SizedBox(height: 8),
          Text(
            widget.value == null
                ? 'Default: _downloads/{title}/ inside manga dir.'
                : 'Chapters save to ${widget.value}',
            style: const TextStyle(fontSize: 11, color: kMutedFg),
          ),
        ],
      ),
    );
  }
}
