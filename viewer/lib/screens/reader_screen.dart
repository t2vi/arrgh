import 'package:cached_network_image/cached_network_image.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../api/client.dart';

class ReaderScreen extends ConsumerStatefulWidget {
  final String chapterId;
  const ReaderScreen({super.key, required this.chapterId});

  @override
  ConsumerState<ReaderScreen> createState() => _ReaderScreenState();
}

class _ReaderScreenState extends ConsumerState<ReaderScreen> {
  late PageController _pageController;
  int _currentPage = 0;
  int _totalPages = 0;
  bool _showUi = false;

  @override
  void initState() {
    super.initState();
    _pageController = PageController();
    _loadChapterInfo();
  }

  Future<void> _loadChapterInfo() async {
    final client = ref.read(apiClientProvider);
    final data = await client.get('/api/chapters/${widget.chapterId}');
    setState(() => _totalPages = data['page_count'] as int);
  }

  @override
  void dispose() {
    _pageController.dispose();
    super.dispose();
  }

  void _toggleUi() => setState(() => _showUi = !_showUi);

  @override
  Widget build(BuildContext context) {
    final client = ref.read(apiClientProvider);

    SystemChrome.setEnabledSystemUIMode(
      _showUi ? SystemUiMode.edgeToEdge : SystemUiMode.immersiveSticky,
    );

    return Scaffold(
      backgroundColor: Colors.black,
      body: Stack(
        children: [
          GestureDetector(
            onTap: _toggleUi,
            child: _totalPages == 0
                ? const Center(child: CircularProgressIndicator())
                : PageView.builder(
                    controller: _pageController,
                    itemCount: _totalPages,
                    onPageChanged: (page) {
                      setState(() => _currentPage = page);
                      _saveProgress(page);
                    },
                    itemBuilder: (context, index) {
                      return InteractiveViewer(
                        minScale: 0.5,
                        maxScale: 4.0,
                        child: CachedNetworkImage(
                          imageUrl: client.pageUrl(widget.chapterId, index),
                          fit: BoxFit.contain,
                          placeholder: (_, __) =>
                              const Center(child: CircularProgressIndicator()),
                          errorWidget: (_, __, ___) =>
                              const Center(child: Icon(Icons.broken_image, color: Colors.white30)),
                        ),
                      );
                    },
                  ),
          ),
          if (_showUi) ...[
            Positioned(
              top: 0,
              left: 0,
              right: 0,
              child: AppBar(
                backgroundColor: Colors.black54,
                title: _totalPages > 0
                    ? Text('${_currentPage + 1} / $_totalPages')
                    : const SizedBox(),
              ),
            ),
            Positioned(
              bottom: 0,
              left: 0,
              right: 0,
              child: Container(
                color: Colors.black54,
                padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
                child: _totalPages > 1
                    ? Slider(
                        value: _currentPage.toDouble(),
                        min: 0,
                        max: (_totalPages - 1).toDouble(),
                        divisions: _totalPages - 1,
                        onChanged: (v) {
                          _pageController.jumpToPage(v.round());
                        },
                      )
                    : const SizedBox(height: 40),
              ),
            ),
          ],
        ],
      ),
    );
  }

  Future<void> _saveProgress(int page) async {
    final client = ref.read(apiClientProvider);
    await client.put('/api/progress/${widget.chapterId}', {
      'current_page': page,
      'completed': page >= _totalPages - 1,
    });
  }
}
