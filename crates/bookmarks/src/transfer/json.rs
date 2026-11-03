//! Clean Share's own backup: every bookmark with its title, note, tags (and where each came
//! from), shared text, dates and preview text. Importing it restores them; links are re-cleaned
//! from the original link with the current rules.

use serde::{Deserialize, Serialize};

use super::{ImportItem, ParseError};
use crate::{Bookmark, TagInput, TagSource};

pub const FORMAT: &str = "clean-share/bookmarks";
pub const VERSION: u32 = 1;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Backup {
  pub format: String,
  pub version: u32,
  /// ms since the epoch.
  pub exported_at: i64,
  pub bookmarks: Vec<Entry>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
  /// Cleaned link.
  pub url: String,
  /// As shared, with tracking; re-cleaned on import.
  #[serde(default)]
  pub original_url: Option<String>,
  #[serde(default)]
  pub title: Option<String>,
  #[serde(default)]
  pub note: Option<String>,
  #[serde(default)]
  pub tags: Vec<EntryTag>,
  #[serde(default)]
  pub shared_text: Option<String>,
  #[serde(default)]
  pub description: Option<String>,
  #[serde(default)]
  pub site_name: Option<String>,
  pub created_at: i64,
  pub updated_at: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EntryTag {
  pub name: String,
  #[serde(default)]
  pub source: TagSource,
}

pub fn export(bookmarks: &[Bookmark], now: i64) -> String {
  let backup = Backup {
    format: FORMAT.into(),
    version: VERSION,
    exported_at: now,
    bookmarks: bookmarks
      .iter()
      .map(|b| Entry {
        url: b.url.clone(),
        original_url: Some(b.original_url.clone()),
        title: b.title.clone(),
        note: b.note.clone(),
        tags: b.tags.iter().map(|t| EntryTag { name: t.name.clone(), source: t.source }).collect(),
        shared_text: Some(b.shared_text.clone()),
        description: b.description.clone(),
        site_name: b.site_name.clone(),
        created_at: b.created_at,
        updated_at: b.updated_at,
      })
      .collect(),
  };
  serde_json::to_string_pretty(&backup).expect("serializable")
}

pub fn parse(text: &str) -> Result<Vec<ImportItem>, ParseError> {
  let backup: Backup =
    serde_json::from_str(text).map_err(|e| ParseError::Invalid("a Clean Share backup", e.to_string()))?;
  if backup.format != FORMAT {
    return Err(ParseError::UnknownFormat);
  }
  if backup.version > VERSION {
    return Err(ParseError::Invalid(
      "a Clean Share backup",
      format!("it's from a newer app (version {}); update the app first", backup.version),
    ));
  }
  Ok(
    backup
      .bookmarks
      .into_iter()
      .map(|e| ImportItem {
        // The original link, so today's rules clean it (they may know more than then).
        url: e.original_url.filter(|u| !u.is_empty()).unwrap_or(e.url),
        title: e.title,
        note: e.note,
        tags: e.tags.into_iter().map(|t| TagInput { name: t.name, source: t.source }).collect(),
        created_at: Some(e.created_at),
      })
      .collect(),
  )
}
