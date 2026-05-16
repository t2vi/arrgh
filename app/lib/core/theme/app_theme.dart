import 'package:flutter/material.dart';
import 'package:google_fonts/google_fonts.dart';

const kBackground = Color(0xFF121415);
const kCard = Color(0xFF1e2021);
const kPrimary = Color(0xFF8b5cf6);
const kMuted = Color(0xFF1a1c1d);
const kMutedFg = Color(0xFF71717a);
const kBorder = Color(0xFF333536);
const kForeground = Color(0xFFf4f4f5);

ThemeData buildTheme() {
  final base = ThemeData.dark();
  final textTheme = GoogleFonts.interTextTheme(base.textTheme).apply(
    bodyColor: kForeground,
    displayColor: kForeground,
  );

  return base.copyWith(
    scaffoldBackgroundColor: kBackground,
    colorScheme: const ColorScheme.dark(
      surface: kBackground,
      primary: kPrimary,
      onPrimary: Colors.white,
      onSurface: kForeground,
    ),
    cardColor: kCard,
    dividerColor: kBorder,
    textTheme: textTheme,
    inputDecorationTheme: InputDecorationTheme(
      filled: true,
      fillColor: kMuted,
      border: OutlineInputBorder(
        borderRadius: BorderRadius.circular(10),
        borderSide: const BorderSide(color: kBorder),
      ),
      enabledBorder: OutlineInputBorder(
        borderRadius: BorderRadius.circular(10),
        borderSide: const BorderSide(color: kBorder),
      ),
      focusedBorder: OutlineInputBorder(
        borderRadius: BorderRadius.circular(10),
        borderSide: const BorderSide(color: kPrimary, width: 2),
      ),
      labelStyle: const TextStyle(color: kMutedFg),
      hintStyle: const TextStyle(color: kMutedFg),
    ),
    elevatedButtonTheme: ElevatedButtonThemeData(
      style: ElevatedButton.styleFrom(
        backgroundColor: kPrimary,
        foregroundColor: Colors.white,
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(10)),
        padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 12),
      ),
    ),
    appBarTheme: const AppBarTheme(
      backgroundColor: kCard,
      foregroundColor: kForeground,
      elevation: 0,
    ),
  );
}
