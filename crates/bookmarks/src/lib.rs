//! Bookmarks for Clean Share: save cleaned links, tag them, find them again.
//!
//! # One database = one user
//!
//! A [`Store`] works on a single database and never filters by user: whoever can open the
//! database owns everything in it. Multi-tenancy is decided by *which* database gets opened
//! (a local file, or later the user's own Turso Cloud database), never inside a query. See
//! `docs/BOOKMARKS.md`, "Accounts and sync".
//!
//! # Pieces
//!
//! - [`db`] opens a database and hands back a [`turso::Connection`]. Local files today; a synced
//!   database later returns the same connection type, so nothing else changes.
//! - [`Store`] runs the migrations and all reads and writes on that connection.
//! - [`Cleaner`] (tracking rules) and [`Clock`] are passed in, so tests and downloaded rules can
//!   replace them.
//! - [`Store::suggest_tags`] gathers the user's tagging history and ranks tags with the
//!   `tag-suggest` crate.
//!
//! IDs are derived from content (cleaned URL, tag name), so two devices that save the same link
//! offline create the same row, which helps sync merge them.

mod cleaner;
mod clock;
pub mod db;
mod error;
mod ids;
mod migrate;
mod model;
mod store;

pub use cleaner::{BundledRules, Cleaner, Rules};
pub use clock::{Clock, SystemClock};
pub use error::{Error, Result};
pub use migrate::SCHEMA_VERSION;
pub use model::*;
pub use store::Store;
pub use tag_suggest::{Reason, Suggestion};
