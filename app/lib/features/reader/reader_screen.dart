import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:cached_network_image/cached_network_image.dart';
import '../../core/api/api_client.dart';
import '../../core/api/api_service.dart';
import '../../core/models/models.dart';
import '../../core/storage/local_chapter_storage.dart';
import '../../core/theme/app_theme.dart';

class ReaderScreen extends ConsumerStatefulWidget {
  final String chapterId;
  const ReaderScreen({super.key, required this.chapterId});

  @override
  ConsumerState<ReaderScreen> createState() => _ReaderScreenState();
}

class _ReaderScreenState extends ConsumerState<ReaderScreen> {
  final _pageCtrl = PageController();
  final _scrollCtrl = ScrollController();
  late final FocusNode _focusNode;
  int _currentPage = 0;
  bool _uiVisible = true;
  Chapter? _chapter;
  String? _mangaReaderMode;  // from manga (null = use global)
  String? _globalReaderMode; // from settings
  String? _sessionOverride;  // in-reader toggle
  int _pageCount = 0;
  int _knownPageCount = 0;

  // Scroll mode: grow rendered pages as user scrolls
  int _renderedPages = 0;
  final Set<int> _failedPages = {};

  // Local cache state
  bool _checkingLocal = false;
  bool _downloading = false;
  int _downloadProgress = 0;
  bool _locallyAvailable = false;

  String get _effectiveMode =>
    _sessionOverride ?? _mangaReaderMode ?? _globalReaderMode ?? 'paged';

  @override
  void initState() {
    super.initState();
    _focusNode = FocusNode(debugLabel: 'reader');
    SystemChrome.setEnabledSystemUIMode(SystemUiMode.immersiveSticky);
    _loadChapter();
    _scrollCtrl.addListener(_onScrollUpdate);
  }

  @override
  void dispose() {
    _focusNode.dispose();
    _scrollCtrl.removeListener(_onScrollUpdate);
    _scrollCtrl.dispose();
    SystemChrome.setEnabledSystemUIMode(SystemUiMode.edgeToEdge);
    _saveProgress();
    _pageCtrl.dispose();
    super.dispose();
  }

  void _onScrollUpdate() {
    if (!_scrollCtrl.hasClients) return;
    final pos = _scrollCtrl.position;
    // Grow rendered pages as user approaches bottom
    if (pos.extentAfter < 600 && _pageCount > 0) {
      setState(() => _renderedPages = (_renderedPages + 5).clamp(0, _pageCount));
    }
    // Approximate current page
    final approxH = pos.maxScrollExtent / _pageCount;
    if (approxH > 0) {
      final p = (pos.pixels / approxH).floor().clamp(0, _pageCount - 1);
      if (p != _currentPage) {
        _currentPage = p;
        svc.updateProgress(widget.chapterId, p, p >= _pageCount - 1).catchError((_) {});
      }
    }
  }

  Future<void> _loadChapter() async {
    try {
      final results = await Future.wait([
        svc.getChapter(widget.chapterId),
        svc.getProgress(widget.chapterId).catchError((_) => null),
        svc.getSettings(),
      ]);
      final ch = results[0] as Chapter;
      final prog = results[1] as ReadProgress?;
      final settings = results[2] as Map<String, dynamic>;

      // Load manga for per-manga reader_mode
      Map<String, dynamic>? mangaData;
      try {
        final res = await svc.getManga(ch.mangaId);
        mangaData = {'reader_mode': res.readerMode};
      } catch (_) {}

      if (!mounted) return;
      setState(() {
        _chapter = ch;
        _knownPageCount = ch.pageCount;
        _pageCount = ch.pageCount > 0 ? ch.pageCount : 20;
        _renderedPages = _pageCount.clamp(0, 20);
        _globalReaderMode = settings['reader_mode'] as String?;
        _mangaReaderMode = mangaData?['reader_mode'] as String?;
        if (prog != null && !prog.completed) {
          _currentPage = prog.currentPage.clamp(0, _pageCount - 1);
          WidgetsBinding.instance.addPostFrameCallback((_) {
            if (_effectiveMode == 'paged') {
              _pageCtrl.jumpToPage(_currentPage);
            }
          });
        }
      });
      _checkLocalAvailability();
    } catch (_) {}
  }

  Future<void> _checkLocalAvailability() async {
    if (_chapter == null || _pageCount == 0) return;
    setState(() => _checkingLocal = true);
    final available = await localChapters.isFullyDownloaded(
        widget.chapterId, _pageCount);
    if (mounted) setState(() { _locallyAvailable = available; _checkingLocal = false; });
  }

  Future<void> _downloadLocally() async {
    if (_chapter == null || _downloading) return;
    setState(() { _downloading = true; _downloadProgress = 0; });
    try {
      await localChapters.downloadChapter(
        widget.chapterId,
        _pageCount,
        onProgress: (done, total) {
          if (mounted) setState(() => _downloadProgress = done);
        },
      );
      if (mounted) setState(() { _locallyAvailable = true; _downloading = false; });
    } catch (e) {
      if (mounted) {
        setState(() => _downloading = false);
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('Download failed: $e')));
      }
    }
  }

  Future<void> _deleteLocal() async {
    await localChapters.deleteChapter(widget.chapterId);
    if (mounted) setState(() => _locallyAvailable = false);
  }

  Future<void> _saveProgress() async {
    if (_chapter == null || !_chapter!.downloaded) return;
    try {
      await svc.updateProgress(
          _chapter!.id, _currentPage,
          _currentPage >= _pageCount - 1);
    } catch (_) {}
  }

  void _onPageChanged(int page) {
    setState(() => _currentPage = page);
    if (_chapter?.downloaded != true) return;
    svc.updateProgress(widget.chapterId, page, page >= _pageCount - 1)
        .catchError((_) {});
  }

  void _toggleUI() => setState(() => _uiVisible = !_uiVisible);

  void _prevPage() {
    if (_currentPage > 0) _pageCtrl.previousPage(
        duration: const Duration(milliseconds: 200), curve: Curves.easeOut);
  }

  void _nextPage() {
    if (_currentPage < _pageCount - 1) {
      _pageCtrl.nextPage(
          duration: const Duration(milliseconds: 200), curve: Curves.easeOut);
    } else {
      _saveProgress().then((_) {
        if (!mounted) return;
        if (context.canPop()) context.pop(); else context.go('/library');
      });
    }
  }

  void _toggleMode() {
    setState(() {
      _sessionOverride = _effectiveMode == 'paged' ? 'scroll' : 'paged';
    });
  }

  @override
  Widget build(BuildContext context) {
    if (_chapter == null) {
      return const Scaffold(
          body: Center(child: CircularProgressIndicator(color: kPrimary)));
    }

    final isScroll = _effectiveMode == 'scroll';

    return Scaffold(
      backgroundColor: Colors.black,
      body: Focus(
        focusNode: _focusNode,
        autofocus: true,
        onKeyEvent: (node, event) {
          if (event is KeyDownEvent && !isScroll) {
            if (event.logicalKey == LogicalKeyboardKey.arrowRight ||
                event.logicalKey == LogicalKeyboardKey.arrowDown) _nextPage();
            if (event.logicalKey == LogicalKeyboardKey.arrowLeft ||
                event.logicalKey == LogicalKeyboardKey.arrowUp) _prevPage();
          }
          if (event is KeyDownEvent &&
              event.logicalKey == LogicalKeyboardKey.escape) {
            if (context.canPop()) context.pop(); else context.go('/library');
          }
          return KeyEventResult.ignored;
        },
        child: Stack(
          children: [
            // ── Main content ──
            if (isScroll)
              ListView.builder(
                controller: _scrollCtrl,
                itemCount: _renderedPages,
                itemBuilder: (ctx, page) => _failedPages.contains(page)
                    ? const SizedBox.shrink()
                    : _PageView(
                        chapterId: widget.chapterId,
                        page: page,
                        locallyAvailable: _locallyAvailable,
                        onError: () {
                          if (page > 0 && _knownPageCount == 0) {
                            setState(() { _pageCount = page; });
                          }
                          setState(() => _failedPages.add(page));
                        },
                      ),
              )
            else
              PageView.builder(
                controller: _pageCtrl,
                onPageChanged: _onPageChanged,
                itemCount: _pageCount,
                itemBuilder: (ctx, page) => _PageView(
                  chapterId: widget.chapterId,
                  page: page,
                  locallyAvailable: _locallyAvailable,
                  onError: () {
                    if (page > 0 && _knownPageCount == 0) {
                      setState(() => _pageCount = page);
                    }
                  },
                ),
              ),

            // ── Top-left: back + title ──
            Positioned(
              top: MediaQuery.of(context).padding.top + 8,
              left: 12,
              child: _Pill(
                child: Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    GestureDetector(
                      onTap: () {
                        if (context.canPop()) context.pop();
                        else context.go('/library');
                      },
                      child: const Icon(Icons.chevron_left, color: Colors.white, size: 20),
                    ),
                    const SizedBox(width: 4),
                    ConstrainedBox(
                      constraints: const BoxConstraints(maxWidth: 160),
                      child: Text(
                        _chapter!.title ?? 'Chapter ${_chapter!.displayNumber}',
                        style: const TextStyle(color: Colors.white, fontSize: 13,
                            fontWeight: FontWeight.w500),
                        overflow: TextOverflow.ellipsis,
                      ),
                    ),
                  ],
                ),
              ),
            ),

            // ── Top-right: mode toggle + download ──
            Positioned(
              top: MediaQuery.of(context).padding.top + 8,
              right: 12,
              child: _Pill(
                child: Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    GestureDetector(
                      onTap: _toggleMode,
                      child: Icon(
                        isScroll ? Icons.menu_book : Icons.view_day_outlined,
                        color: Colors.white70, size: 18),
                    ),
                    const SizedBox(width: 10),
                    if (_checkingLocal)
                      const SizedBox(width: 16, height: 16,
                          child: CircularProgressIndicator(strokeWidth: 2, color: Colors.white54))
                    else if (_downloading)
                      Text('$_downloadProgress/$_pageCount',
                          style: const TextStyle(color: Colors.white70, fontSize: 12))
                    else if (_locallyAvailable)
                      GestureDetector(
                          onTap: _deleteLocal,
                          child: const Icon(Icons.phone_iphone, color: Color(0xFF34d399), size: 18))
                    else
                      GestureDetector(
                          onTap: _downloadLocally,
                          child: const Icon(Icons.download_for_offline_outlined,
                              color: Colors.white70, size: 18)),
                  ],
                ),
              ),
            ),

            // ── Bottom: page counter (paged only) ──
            if (!isScroll)
              Positioned(
                bottom: MediaQuery.of(context).padding.bottom + 12,
                left: 16,
                child: _Pill(
                  child: Row(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      GestureDetector(
                        onTap: _currentPage > 0 ? _prevPage : null,
                        child: Icon(Icons.navigate_before, size: 22,
                            color: _currentPage > 0 ? Colors.white70 : Colors.white24),
                      ),
                      const SizedBox(width: 8),
                      Text('${_currentPage + 1} / $_pageCount',
                          style: const TextStyle(color: Colors.white, fontSize: 13,
                              fontWeight: FontWeight.w600)),
                      const SizedBox(width: 8),
                      GestureDetector(
                        onTap: _nextPage,
                        child: const Icon(Icons.navigate_next, size: 22, color: Colors.white70),
                      ),
                    ],
                  ),
                ),
              ),
          ],
        ),
      ),
    );
  }
}

class _PageView extends StatefulWidget {
  final String chapterId;
  final int page;
  final bool locallyAvailable;
  final VoidCallback? onError;

  const _PageView({
    required this.chapterId,
    required this.page,
    required this.locallyAvailable,
    this.onError,
  });

  @override
  State<_PageView> createState() => _PageViewState();
}

class _PageViewState extends State<_PageView> {
  String? _localPath;
  bool _checked = false;

  @override
  void initState() {
    super.initState();
    _resolve();
  }

  @override
  void didUpdateWidget(_PageView old) {
    super.didUpdateWidget(old);
    if (old.locallyAvailable != widget.locallyAvailable) _resolve();
  }

  Future<void> _resolve() async {
    if (widget.locallyAvailable) {
      final path = await localChapters.getLocalPage(widget.chapterId, widget.page);
      if (mounted) setState(() { _localPath = path; _checked = true; });
    } else {
      if (mounted) setState(() { _localPath = null; _checked = true; });
    }
  }

  @override
  Widget build(BuildContext context) {
    if (!_checked) {
      return const Center(child: CircularProgressIndicator(color: kPrimary));
    }

    final token = api.getTokenSync();
    final ImageProvider provider = _localPath != null
        ? FileImage(File(_localPath!))
        : CachedNetworkImageProvider(
            api.pageUrl(widget.chapterId, widget.page),
            headers: token != null ? {'Authorization': 'Bearer $token'} : null,
          );

    return InteractiveViewer(
      minScale: 0.5,
      maxScale: 4.0,
      child: Center(
        child: Image(
          image: provider,
          fit: BoxFit.contain,
          loadingBuilder: (context, child, progress) {
            if (progress == null) return child;
            return SizedBox(
              height: 200,
              child: Center(
                child: CircularProgressIndicator(
                  value: progress.expectedTotalBytes != null
                      ? progress.cumulativeBytesLoaded / progress.expectedTotalBytes!
                      : null,
                  color: kPrimary,
                ),
              ),
            );
          },
          errorBuilder: (context, error, stackTrace) {
            WidgetsBinding.instance
                .addPostFrameCallback((_) => widget.onError?.call());
            return const SizedBox.shrink();
          },
        ),
      ),
    );
  }
}

class _Pill extends StatelessWidget {
  final Widget child;
  const _Pill({required this.child});

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
      decoration: BoxDecoration(
        color: const Color(0xCC121415),
        borderRadius: BorderRadius.circular(24),
      ),
      child: child,
    );
  }
}
