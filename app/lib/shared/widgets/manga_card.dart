import 'package:cached_network_image/cached_network_image.dart';
import 'package:flutter/material.dart';
import '../../core/api/api_client.dart';
import '../../core/models/models.dart';
import '../../core/theme/app_theme.dart';
import 'tv_focusable.dart';

class MangaCoverCard extends StatelessWidget {
  final String title;
  final String? coverUrl;
  final String? mangaId;
  final String? subtitle;
  final bool syncing;
  final bool isTv;
  final VoidCallback? onTap;
  final VoidCallback? onLongPress;
  final Widget? badge;

  const MangaCoverCard({
    super.key,
    required this.title,
    this.coverUrl,
    this.mangaId,
    this.subtitle,
    this.syncing = false,
    this.isTv = false,
    this.onTap,
    this.onLongPress,
    this.badge,
  });

  String? get _imageUrl {
    if (coverUrl != null && coverUrl!.startsWith('http')) return coverUrl;
    if (mangaId != null) return api.coverUrl(mangaId!);
    return null;
  }

  @override
  Widget build(BuildContext context) {
    final width = isTv ? 180.0 : 140.0;
    final card = GestureDetector(
      onTap: onTap,
      onLongPress: onLongPress,
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
                    _CoverImage(url: _imageUrl),
                    if (syncing)
                      Container(
                        color: Colors.black54,
                        child: const Center(
                          child: CircularProgressIndicator(color: kPrimary),
                        ),
                      ),
                    if (badge != null)
                      Positioned(top: 6, left: 6, child: badge!),
                  ],
                ),
              ),
            ),
            const SizedBox(height: 6),
            Text(
              title,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: TextStyle(
                fontSize: isTv ? 14 : 13,
                fontWeight: FontWeight.w600,
                color: kForeground,
              ),
            ),
            if (subtitle != null)
              Text(
                subtitle!,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: const TextStyle(fontSize: 11, color: kMutedFg),
              ),
          ],
        ),
      ),
    );

    return TvFocusable(onSelect: onTap, child: card);
  }
}

class _CoverImage extends StatelessWidget {
  final String? url;
  const _CoverImage({this.url});

  @override
  Widget build(BuildContext context) {
    if (url == null) {
      return Container(
        color: kMuted,
        child: const Icon(Icons.book, color: kMutedFg, size: 40),
      );
    }
    return CachedNetworkImage(
      imageUrl: url!,
      fit: BoxFit.cover,
      placeholder: (_, __) => Container(color: kMuted),
      errorWidget: (_, __, ___) => Container(
        color: kMuted,
        child: const Icon(Icons.broken_image, color: kMutedFg, size: 40),
      ),
    );
  }
}

class SectionHeader extends StatelessWidget {
  final String title;
  final String? action;
  final VoidCallback? onAction;

  const SectionHeader({
    super.key,
    required this.title,
    this.action,
    this.onAction,
  });

  @override
  Widget build(BuildContext context) {
    return Row(
      children: [
        Text(title,
            style: const TextStyle(
                fontSize: 20, fontWeight: FontWeight.w800, color: kForeground)),
        const Spacer(),
        if (action != null)
          GestureDetector(
            onTap: onAction,
            child: Text(action!,
                style: const TextStyle(
                    fontSize: 13, color: kPrimary, fontWeight: FontWeight.w600)),
          ),
      ],
    );
  }
}

const _contentTypeColors = {
  'manga':  (Color(0xFF8b5cf6), Color(0x338b5cf6)),
  'manhwa': (Color(0xFF0ea5e9), Color(0x330ea5e9)),
  'manhua': (Color(0xFFf59e0b), Color(0x33f59e0b)),
  'novel':  (Color(0xFF10b981), Color(0x3310b981)),
};

class ContentTypePill extends StatelessWidget {
  final String type;
  const ContentTypePill(this.type, {super.key});

  @override
  Widget build(BuildContext context) {
    final colors = _contentTypeColors[type] ?? _contentTypeColors['manga']!;
    final textColor = colors.$1;
    final bgColor = colors.$2;
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
      decoration: BoxDecoration(
        color: bgColor,
        borderRadius: BorderRadius.circular(20),
        border: Border.all(color: textColor.withValues(alpha: 0.4)),
      ),
      child: Text(
        type[0].toUpperCase() + type.substring(1),
        style: TextStyle(fontSize: 11, fontWeight: FontWeight.w700, color: textColor),
      ),
    );
  }
}

/// Proxied cover for Mangapill trending cards
class ProxiedCoverCard extends StatelessWidget {
  final SearchResult result;
  final bool isTv;
  final VoidCallback? onTap;
  final Widget? badge;

  const ProxiedCoverCard({
    super.key,
    required this.result,
    this.isTv = false,
    this.onTap,
    this.badge,
  });

  String? get _imageUrl {
    final url = result.coverUrl;
    if (url == null) return null;
    if (result.source == 'mangapill') return api.proxyUrl(url);
    return url;
  }

  @override
  Widget build(BuildContext context) {
    final width = isTv ? 180.0 : 140.0;
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
                    _CoverImage(url: _imageUrl),
                    if (badge != null)
                      Positioned(top: 6, left: 6, child: badge!),
                    if (result.inLibrary)
                      Positioned(
                        top: 6, right: 6,
                        child: Container(
                          width: 8, height: 8,
                          decoration: const BoxDecoration(
                            color: Color(0xFF34d399),
                            shape: BoxShape.circle,
                          ),
                        ),
                      ),
                  ],
                ),
              ),
            ),
            const SizedBox(height: 6),
            Text(
              result.title,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: TextStyle(
                fontSize: isTv ? 14 : 13,
                fontWeight: FontWeight.w600,
                color: kForeground,
              ),
            ),
          ],
        ),
      ),
    );

    return TvFocusable(onSelect: onTap, child: card);
  }
}
