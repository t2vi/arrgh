import 'package:flutter/material.dart';

enum LayoutType { phone, tablet, tv }

LayoutType layoutType(BuildContext context) {
  final w = MediaQuery.of(context).size.width;
  if (w >= 1100) return LayoutType.tv;
  if (w >= 600) return LayoutType.tablet;
  return LayoutType.phone;
}

bool isTV(BuildContext context) => layoutType(context) == LayoutType.tv;
bool isTablet(BuildContext context) => layoutType(context) != LayoutType.phone;

String timeAgo(String iso) {
  try {
    final dt = DateTime.parse(iso);
    final diff = DateTime.now().difference(dt);
    if (diff.inMinutes < 1) return 'Just now';
    if (diff.inHours < 1) return '${diff.inMinutes}m ago';
    if (diff.inDays < 1) return '${diff.inHours}h ago';
    if (diff.inDays == 1) return 'Yesterday';
    return '${diff.inDays}d ago';
  } catch (_) {
    return '';
  }
}
