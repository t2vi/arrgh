//! Metadata authority clients for Discover fan-out (ADR 0031, S6 #128).
//!
//! ponytail: the ADR floats splitting this into an `arrgh-metadata` crate
//! once there's "a second crate's worth of code to move" — six thin HTTP
//! clients with no reuse target outside `arrgh-server` don't clear that bar
//! (workspace ceremony with zero functional benefit for a single-binary
//! deployment), so this stays a module. Revisit if a second consumer shows up.
//!
//! E-Hentai (`Services/EHentaiService.cs`) is dead code in .NET — zero
//! references outside its own file, superseded by nhentai (queried directly
//! via plugin-host in `crate::discover`, not a metadata-authority client).
//! Not ported.

pub mod anilist;
pub mod mangadex;
pub mod mangaupdates;
pub mod novelupdates;
pub mod wuxiaworld;
