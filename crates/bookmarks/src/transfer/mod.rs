//! Export and import files. Pure: these functions turn text into [`ImportItem`]s and bookmarks
//! into text; [`crate::Store::import`] and the `export_*` methods do the database part.
//!
//! Import reads, by content (not by file name):
//! - **Clean Share JSON**, this app's own backup ([`json`]): everything, including tag sources.
//! - **Bookmark HTML** ([`netscape`]): what Chrome, Firefox, Safari, Edge, Pocket (`ril_export.html`)
//!   and Raindrop export. Folders become tags (except the browsers' own top folders).
//! - **CSV** ([`table`]): Pocket (`title,url,time_added,tags,status`), Raindrop
//!   (`id,title,note,excerpt,url,folder,tags,created,…`), or any CSV with a `url` column.
//!
//! Every link goes through the cleaning rules on import, like a share does.

mod date;
pub mod json;
pub mod netscape;
pub mod table;

use serde::Serialize;

use crate::TagInput;

/// One bookmark read from a file, not yet cleaned or saved.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ImportItem {
  pub url: String,
  pub title: Option<String>,
  pub note: Option<String>,
  pub tags: Vec<TagInput>,
  /// When it was saved there, ms since the epoch.
  pub created_at: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Format {
  /// This app's own JSON backup.
  CleanShare,
  /// Netscape bookmark HTML (browsers, Pocket, Raindrop).
  Html,
  Pocket,
  Raindrop,
  /// Some other CSV with a `url` column.
  Csv,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ParseError {
  #[error("this file isn't a bookmarks export Clean Share can read (JSON, bookmark HTML or CSV)")]
  UnknownFormat,
  #[error("the file looks like {0} but can't be read: {1}")]
  Invalid(&'static str, String),
}

/// Reads an export file, whatever it is.
pub fn parse(text: &str) -> Result<(Format, Vec<ImportItem>), ParseError> {
  let text = text.trim_start_matches('\u{feff}');
  let start = text.trim_start();
  if start.starts_with('{') {
    return json::parse(start).map(|items| (Format::CleanShare, items));
  }
  let head: String = start.chars().take(2048).collect::<String>().to_ascii_lowercase();
  if head.contains("<!doctype netscape-bookmark-file") || head.contains("<a href=") || head.contains("<dt>") {
    return Ok((Format::Html, netscape::parse(start)));
  }
  table::parse(start)
}

/// Splits "a, b|c" into tags, trimmed, without empties.
fn split_tags(list: &str, separators: &[char]) -> Vec<TagInput> {
  list.split(separators).map(str::trim).filter(|t| !t.is_empty()).map(TagInput::user).collect()
}

/// Trimmed, `None` if empty.
fn text(value: &str) -> Option<String> {
  let value = value.trim();
  (!value.is_empty()).then(|| value.to_string())
}

#[cfg(test)]
mod tests;
