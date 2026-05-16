# *ARRgh — Domain Glossary

## User
An authenticated account. Has a `role` (admin or member) and an `allow_explicit` flag.

## Admin
A User with `role = admin`. Can create and delete Users, change server settings, flag Manga as explicit, and grant Explicit Permission to Members. The first registered User is always an Admin.

## Member
A User with `role = member`. Can add Manga to the Library, trigger syncs, queue downloads, and track their own Read Progress. Cannot manage Users or change server settings.

## Explicit Permission
The `allow_explicit` flag on a User. Granted by an Admin. When false (default), Explicit Manga are hidden from that User everywhere — Library, Discover, and Download Queue.

## Library
The shared collection of Manga. Not owned by any User. All Users with access see the same Manga (subject to Explicit Permission).

## Manga
A title in the Library. Has an `is_explicit` flag (default false). Explicit Manga are hidden from Users without Explicit Permission.

## Explicit Manga
A Manga with `is_explicit = true`. Hidden from — not locked for — Users without Explicit Permission. Applies to Library browsing, Discover results, and the Download Queue.

## Source
An external scraper (e.g. Mangapill, nhentai, manga18fx). Has a `default_explicit` flag that pre-fills `is_explicit` when Manga is added from that Source. Admin can override `is_explicit` on any Manga regardless of Source.

## Read Progress
Per-User, per-Chapter reading state (current page, completed flag). Not shared between Users.

## User Manga Settings
Per-User, per-Manga preferences (e.g. `reader_mode`). Distinct from global Manga settings (`auto_download`, `download_dir`) which are shared and Admin-controlled.
