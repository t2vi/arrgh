import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import '../../core/theme/app_theme.dart';

/// Wraps any widget with TV D-pad focus + violet glow on focus.
class TvFocusable extends StatefulWidget {
  final Widget child;
  final VoidCallback? onSelect;
  final FocusNode? focusNode;
  final bool autofocus;
  final bool showFocusDecoration;

  const TvFocusable({
    super.key,
    required this.child,
    this.onSelect,
    this.focusNode,
    this.autofocus = false,
    this.showFocusDecoration = true,
  });

  @override
  State<TvFocusable> createState() => _TvFocusableState();
}

class _TvFocusableState extends State<TvFocusable> {
  late FocusNode _node;
  bool _focused = false;

  @override
  void initState() {
    super.initState();
    _node = widget.focusNode ?? FocusNode();
    _node.addListener(_onFocusChange);
  }

  void _onFocusChange() {
    setState(() => _focused = _node.hasFocus);
  }

  @override
  void dispose() {
    if (widget.focusNode == null) _node.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return Focus(
      focusNode: _node,
      autofocus: widget.autofocus,
      onKeyEvent: (node, event) {
        if (event is KeyDownEvent &&
            (event.logicalKey == LogicalKeyboardKey.select ||
                event.logicalKey == LogicalKeyboardKey.enter ||
                event.logicalKey == LogicalKeyboardKey.gameButtonA)) {
          widget.onSelect?.call();
          return KeyEventResult.handled;
        }
        return KeyEventResult.ignored;
      },
      child: AnimatedContainer(
        duration: const Duration(milliseconds: 150),
        decoration: widget.showFocusDecoration
            ? BoxDecoration(
                borderRadius: BorderRadius.circular(12),
                boxShadow: _focused
                    ? [
                        BoxShadow(
                          color: kPrimary.withValues(alpha: 0.5),
                          blurRadius: 16,
                          spreadRadius: 2,
                        )
                      ]
                    : null,
                border: Border.all(
                  color: _focused ? kPrimary : Colors.transparent,
                  width: 2,
                ),
              )
            : null,
        child: widget.child,
      ),
    );
  }
}
