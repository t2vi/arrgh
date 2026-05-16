import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import 'screens/library_screen.dart';
import 'screens/manga_detail_screen.dart';
import 'screens/reader_screen.dart';
import 'screens/settings_screen.dart';
import 'theme/app_theme.dart';

void main() {
  runApp(const ProviderScope(child: ArrghApp()));
}

final _router = GoRouter(
  routes: [
    GoRoute(
      path: '/',
      builder: (context, state) => const LibraryScreen(),
    ),
    GoRoute(
      path: '/manga/:id',
      builder: (context, state) => MangaDetailScreen(
        mangaId: state.pathParameters['id']!,
      ),
    ),
    GoRoute(
      path: '/reader/:chapterId',
      builder: (context, state) => ReaderScreen(
        chapterId: state.pathParameters['chapterId']!,
      ),
    ),
    GoRoute(
      path: '/settings',
      builder: (context, state) => const SettingsScreen(),
    ),
  ],
);

class ArrghApp extends StatelessWidget {
  const ArrghApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp.router(
      title: 'Arrgh',
      theme: AppTheme.dark(),
      routerConfig: _router,
      debugShowCheckedModeBanner: false,
    );
  }
}
