//! Turning database rows into model types.

use turso::{Row, Value};

use crate::{Bookmark, Error, PreviewStatus, Result, TagSource};

/// Column order read by [`bookmark`].
pub const BOOKMARK_COLUMNS: &str = "id, url, original_url, title, note, shared_text, provider, \
  site_name, description, image_url, embed_kind, embed_url, preview_status, preview_fetched_at, \
  rules_revision, created_at, updated_at";

/// A bookmark without its tags (loaded separately).
pub fn bookmark(row: &Row) -> Result<Bookmark> {
  Ok(Bookmark {
    id: text(row, 0, "id")?,
    url: text(row, 1, "url")?,
    original_url: text(row, 2, "original_url")?,
    title: opt_text(row, 3, "title")?,
    note: opt_text(row, 4, "note")?,
    shared_text: text(row, 5, "shared_text")?,
    provider: opt_text(row, 6, "provider")?,
    site_name: opt_text(row, 7, "site_name")?,
    description: opt_text(row, 8, "description")?,
    image_url: opt_text(row, 9, "image_url")?,
    embed_kind: opt_text(row, 10, "embed_kind")?,
    embed_url: opt_text(row, 11, "embed_url")?,
    preview_status: preview_status(&text(row, 12, "preview_status")?)?,
    preview_fetched_at: opt_int(row, 13, "preview_fetched_at")?,
    rules_revision: text(row, 14, "rules_revision")?,
    tags: Vec::new(),
    created_at: int(row, 15, "created_at")?,
    updated_at: int(row, 16, "updated_at")?,
  })
}

pub fn opt_text(row: &Row, idx: usize, column: &'static str) -> Result<Option<String>> {
  match row.get_value(idx)? {
    Value::Null => Ok(None),
    Value::Text(text) => Ok(Some(text)),
    _ => Err(Error::BadColumn { column }),
  }
}

pub fn text(row: &Row, idx: usize, column: &'static str) -> Result<String> {
  opt_text(row, idx, column)?.ok_or(Error::BadColumn { column })
}

pub fn opt_int(row: &Row, idx: usize, column: &'static str) -> Result<Option<i64>> {
  match row.get_value(idx)? {
    Value::Null => Ok(None),
    Value::Integer(n) => Ok(Some(n)),
    _ => Err(Error::BadColumn { column }),
  }
}

pub fn int(row: &Row, idx: usize, column: &'static str) -> Result<i64> {
  opt_int(row, idx, column)?.ok_or(Error::BadColumn { column })
}

fn preview_status(value: &str) -> Result<PreviewStatus> {
  Ok(match value {
    "none" => PreviewStatus::None,
    "pending" => PreviewStatus::Pending,
    "done" => PreviewStatus::Done,
    "failed" => PreviewStatus::Failed,
    _ => return Err(Error::BadColumn { column: "preview_status" }),
  })
}

pub fn tag_source(value: &str) -> TagSource {
  match value {
    "suggested" => TagSource::Suggested,
    "ai" => TagSource::Ai,
    _ => TagSource::User,
  }
}

pub fn tag_source_str(source: TagSource) -> &'static str {
  match source {
    TagSource::User => "user",
    TagSource::Suggested => "suggested",
    TagSource::Ai => "ai",
  }
}
