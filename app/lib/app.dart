import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'core/providers/auth_provider.dart';
import 'core/theme/app_theme.dart';
import 'features/auth/login_screen.dart';
import 'features/auth/setup_screen.dart';
import 'features/downloads/downloads_screen.dart';
import 'features/home/home_screen.dart';
import 'features/library/library_screen.dart';
import 'features/discover/discover_screen.dart';
import 'features/manga_detail/manga_detail_screen.dart';
import 'features/reader/reader_screen.dart';
import 'shared/widgets/app_shell.dart';

final _routerProvider = Provider<GoRouter>((ref) {
  final auth = ref.watch(authProvider);

  return GoRouter(
    initialLocation: '/',
    redirect: (ctx, state) {
      final loc = state.uri.toString();
      switch (auth) {
        case AuthState.loading:
          return null;
        case AuthState.needsSetup:
          return loc == '/setup' ? null : '/setup';
        case AuthState.needsLogin:
          return loc == '/login' ? null : '/login';
        case AuthState.authenticated:
          if (loc == '/login' || loc == '/setup') return '/';
          return null;
      }
    },
    routes: [
      GoRoute(path: '/login', builder: (_, __) => const LoginScreen()),
      GoRoute(path: '/setup', builder: (_, __) => const SetupScreen()),
      GoRoute(
        path: '/reader/:chapterId',
        builder: (_, s) => ReaderScreen(chapterId: s.pathParameters['chapterId']!),
      ),
      GoRoute(
        path: '/manga/:id',
        builder: (_, s) => MangaDetailScreen(mangaId: s.pathParameters['id']!),
      ),
      ShellRoute(
        builder: (ctx, state, child) =>
            AppShell(child: child, location: state.uri.toString()),
        routes: [
          GoRoute(path: '/', builder: (_, __) => const HomeScreen()),
          GoRoute(path: '/library', builder: (_, __) => const LibraryScreen()),
          GoRoute(path: '/discover', builder: (_, __) => const DiscoverScreen()),
          GoRoute(path: '/downloads', builder: (_, __) => const DownloadsScreen()),
        ],
      ),
    ],
  );
});

class ArrghApp extends ConsumerWidget {
  const ArrghApp({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final router = ref.watch(_routerProvider);
    return MaterialApp.router(
      title: '*ARRgh',
      theme: buildTheme(),
      routerConfig: router,
      debugShowCheckedModeBanner: false,
    );
  }
}
