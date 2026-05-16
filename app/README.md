# arrgh — app

Flutter app for Android phones, tablets, and Firestick (TV D-pad navigation).

## Dev setup

```bash
cd app
flutter pub get
flutter run
```

Requires the server running. By default the app points to `http://10.0.2.2:3000` (Android emulator localhost). Change the server URL from the settings screen on first launch.

## Build (Android APK)

```bash
flutter build apk --release
# APK at build/app/outputs/flutter-apk/app-release.apk
```

Sideload to Firestick via `adb install app-release.apk`.

## Tests

```bash
flutter test
```

## Platform notes

- **Phone / tablet**: touch navigation, standard layouts
- **TV / Firestick**: D-pad navigation via `TvFocusable`, layouts switch at width ≥ 1100px
- Width ≥ 600px = tablet layout, ≥ 1100px = TV layout

## Project structure

```
lib/
├── core/
│   ├── api/       # API client + service
│   ├── models/    # Shared data models
│   ├── storage/   # Local chapter cache
│   └── theme/     # Design tokens
├── features/      # Screens (library, discover, reader, manga detail…)
└── shared/
    ├── utils/     # Platform detection, helpers
    └── widgets/   # Shared widgets (TvFocusable, MangaCard…)
```
