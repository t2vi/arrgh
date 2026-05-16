import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import '../../core/api/api_client.dart';
import '../../core/api/api_service.dart';
import '../../core/providers/auth_provider.dart';
import '../../core/theme/app_theme.dart';
import '../utils/platform_utils.dart';
import 'tv_focusable.dart';

final _activeDownloadsProvider = StreamProvider.autoDispose<int>((ref) async* {
  while (true) {
    try {
      final items = await svc.getQueue();
      yield items
          .where((i) => i.status == 'downloading' || i.status == 'pending')
          .length;
    } catch (_) {
      yield 0;
    }
    await Future.delayed(const Duration(seconds: 3));
  }
});

class AppShell extends ConsumerWidget {
  final Widget child;
  final String location;

  const AppShell({super.key, required this.child, required this.location});

  static const _destinations = [
    _NavDest(icon: Icons.home_outlined, activeIcon: Icons.home, label: 'Home', path: '/'),
    _NavDest(icon: Icons.library_books_outlined, activeIcon: Icons.library_books, label: 'Library', path: '/library'),
    _NavDest(icon: Icons.explore_outlined, activeIcon: Icons.explore, label: 'Discover', path: '/discover'),
    _NavDest(icon: Icons.download_outlined, activeIcon: Icons.download, label: 'Downloads', path: '/downloads'),
  ];

  int get _selectedIndex {
    for (var i = 0; i < _destinations.length; i++) {
      final path = _destinations[i].path;
      if (path == '/' ? location == '/' : location.startsWith(path)) return i;
    }
    return 0;
  }

  void _navigate(BuildContext ctx, int index) {
    final path = _destinations[index].path;
    if (!GoRouterState.of(ctx).uri.toString().startsWith(path) || path == '/') {
      ctx.go(path);
    }
  }

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final layout = layoutType(context);
    final username = api.getUsernameSync() ?? 'User';
    final activeDownloads = ref.watch(_activeDownloadsProvider).valueOrNull ?? 0;
    void logout() => ref.read(authProvider.notifier).logout();

    return switch (layout) {
      LayoutType.phone => _PhoneShell(
          child: child,
          selectedIndex: _selectedIndex,
          destinations: _destinations,
          onTap: (i) => _navigate(context, i),
          username: username,
          onLogout: logout,
          activeDownloads: activeDownloads,
        ),
      LayoutType.tablet || LayoutType.tv => _SidebarShell(
          child: child,
          selectedIndex: _selectedIndex,
          destinations: _destinations,
          onTap: (i) => _navigate(context, i),
          expanded: layout == LayoutType.tv,
          username: username,
          onLogout: logout,
          activeDownloads: activeDownloads,
        ),
    };
  }
}

class _NavDest {
  final IconData icon;
  final IconData activeIcon;
  final String label;
  final String path;
  const _NavDest({required this.icon, required this.activeIcon, required this.label, required this.path});
}

class _PhoneShell extends StatelessWidget {
  final Widget child;
  final int selectedIndex;
  final List<_NavDest> destinations;
  final ValueChanged<int> onTap;
  final String username;
  final VoidCallback onLogout;
  final int activeDownloads;

  const _PhoneShell({
    required this.child,
    required this.selectedIndex,
    required this.destinations,
    required this.onTap,
    required this.username,
    required this.onLogout,
    required this.activeDownloads,
  });

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      body: child,
      bottomNavigationBar: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          Container(
            decoration: const BoxDecoration(
              color: kCard,
              border: Border(top: BorderSide(color: kBorder)),
            ),
            padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 6),
            child: Row(
              children: [
                const Icon(Icons.person_outline, size: 15, color: kMutedFg),
                const SizedBox(width: 8),
                Expanded(
                  child: Text(username,
                      style: const TextStyle(fontSize: 12, color: kMutedFg),
                      overflow: TextOverflow.ellipsis),
                ),
                GestureDetector(
                  onTap: onLogout,
                  child: const Padding(
                    padding: EdgeInsets.all(4),
                    child: Icon(Icons.logout, size: 16, color: kMutedFg),
                  ),
                ),
              ],
            ),
          ),
          NavigationBar(
            backgroundColor: kCard,
            selectedIndex: selectedIndex,
            onDestinationSelected: onTap,
            indicatorColor: kPrimary.withValues(alpha: 0.2),
            destinations: destinations.map((d) {
              final isDownloads = d.path == '/downloads';
              final showBadge = isDownloads && activeDownloads > 0;
              Widget icon = Icon(d.icon, color: kMutedFg);
              Widget activeIcon = Icon(d.activeIcon, color: kPrimary);
              if (showBadge) {
                final label = activeDownloads > 99 ? '99+' : '$activeDownloads';
                icon = Badge(label: Text(label, style: const TextStyle(fontSize: 9)), backgroundColor: kPrimary, child: icon);
                activeIcon = Badge(label: Text(label, style: const TextStyle(fontSize: 9)), backgroundColor: kPrimary, child: activeIcon);
              }
              return NavigationDestination(icon: icon, selectedIcon: activeIcon, label: d.label);
            }).toList(),
          ),
        ],
      ),
    );
  }
}

class _SidebarShell extends StatelessWidget {
  final Widget child;
  final int selectedIndex;
  final List<_NavDest> destinations;
  final ValueChanged<int> onTap;
  final bool expanded;
  final String username;
  final VoidCallback onLogout;
  final int activeDownloads;

  const _SidebarShell({
    required this.child,
    required this.selectedIndex,
    required this.destinations,
    required this.onTap,
    required this.expanded,
    required this.username,
    required this.onLogout,
    required this.activeDownloads,
  });

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      body: Row(
        children: [
          Container(
            width: expanded ? 220 : 72,
            decoration: const BoxDecoration(
              color: kCard,
              border: Border(right: BorderSide(color: kBorder)),
            ),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                const SizedBox(height: 24),
                Padding(
                  padding: EdgeInsets.symmetric(
                      horizontal: expanded ? 20 : 16, vertical: 8),
                  child: expanded
                      ? const Text('*ARRgh',
                          style: TextStyle(
                              fontSize: 22,
                              fontWeight: FontWeight.w800,
                              color: kPrimary))
                      : const Icon(Icons.auto_stories, color: kPrimary, size: 28),
                ),
                const SizedBox(height: 16),
                ...destinations.asMap().entries.map((e) {
                  final i = e.key;
                  final d = e.value;
                  final selected = selectedIndex == i;
                  final badge = d.path == '/downloads' && activeDownloads > 0
                      ? activeDownloads
                      : 0;
                  return _SideNavItem(
                    icon: selected ? d.activeIcon : d.icon,
                    label: d.label,
                    selected: selected,
                    expanded: expanded,
                    badge: badge,
                    onTap: () => onTap(i),
                  );
                }),
                const Spacer(),
                Container(
                  decoration: const BoxDecoration(
                    border: Border(top: BorderSide(color: kBorder)),
                  ),
                  padding: EdgeInsets.symmetric(
                      horizontal: expanded ? 16 : 12, vertical: 12),
                  child: expanded
                      ? Row(
                          children: [
                            const Icon(Icons.person_outline, size: 18, color: kMutedFg),
                            const SizedBox(width: 8),
                            Expanded(
                              child: Text(username,
                                  style: const TextStyle(
                                      fontSize: 13, color: kMutedFg,
                                      fontWeight: FontWeight.w500),
                                  overflow: TextOverflow.ellipsis),
                            ),
                            GestureDetector(
                              onTap: onLogout,
                              child: const Padding(
                                padding: EdgeInsets.all(4),
                                child: Icon(Icons.logout, size: 16, color: kMutedFg),
                              ),
                            ),
                          ],
                        )
                      : GestureDetector(
                          onTap: onLogout,
                          child: const Center(
                            child: Icon(Icons.logout, size: 20, color: kMutedFg),
                          ),
                        ),
                ),
              ],
            ),
          ),
          Expanded(child: child),
        ],
      ),
    );
  }
}

class _SideNavItem extends StatelessWidget {
  final IconData icon;
  final String label;
  final bool selected;
  final bool expanded;
  final int badge;
  final VoidCallback onTap;

  const _SideNavItem({
    required this.icon,
    required this.label,
    required this.selected,
    required this.expanded,
    required this.onTap,
    this.badge = 0,
  });

  @override
  Widget build(BuildContext context) {
    final iconColor = selected ? kPrimary : kMutedFg;
    // collapsed sidebar: badge on icon; expanded: badge shown as right-side pill instead
    final iconWidget = badge > 0 && !expanded
        ? Badge(
            label: Text(badge > 99 ? '99+' : '$badge',
                style: const TextStyle(fontSize: 9)),
            backgroundColor: kPrimary,
            child: Icon(icon, color: iconColor, size: 22),
          )
        : Icon(icon, color: iconColor, size: 22);

    return TvFocusable(
      onSelect: onTap,
      child: InkWell(
      onTap: onTap,
      canRequestFocus: false,
      borderRadius: BorderRadius.circular(10),
      child: Container(
        margin: const EdgeInsets.symmetric(horizontal: 10, vertical: 2),
        padding: EdgeInsets.symmetric(
            horizontal: expanded ? 14 : 12, vertical: 10),
        decoration: BoxDecoration(
          borderRadius: BorderRadius.circular(10),
          color: selected ? kPrimary.withValues(alpha: 0.15) : null,
        ),
        child: Row(
          children: [
            iconWidget,
            if (expanded) ...[
              const SizedBox(width: 12),
              Expanded(
                child: Text(label,
                    style: TextStyle(
                        color: selected ? kPrimary : kMutedFg,
                        fontWeight: selected ? FontWeight.w600 : FontWeight.normal,
                        fontSize: 15)),
              ),
              if (badge > 0)
                Container(
                  padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
                  decoration: BoxDecoration(
                    color: kPrimary,
                    borderRadius: BorderRadius.circular(10),
                  ),
                  child: Text(badge > 99 ? '99+' : '$badge',
                      style: const TextStyle(
                          fontSize: 9,
                          fontWeight: FontWeight.w700,
                          color: Colors.white)),
                ),
            ],
          ],
        ),
      ),
      ),
    );
  }
}
