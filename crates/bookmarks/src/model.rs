use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Bookmark {
  pub id: String,
  /// Cleaned link: what the bookmark *is*. Unique.
  pub url: String,
  /// As shared, with tracking. Kept for re-cleaning; never shown by default.
  pub original_url: String,
  pub title: Option<String>,
  pub note: Option<String>,
  /// The full text that came with the share.
  pub shared_text: String,
  /// Provider id from the rules (e.g. `youtube`), if one matched.
  pub provider: Option<String>,
  pub site_name: Option<String>,
  pub description: Option<String>,
  pub image_url: Option<String>,
  pub embed_kind: Option<String>,
  pub embed_url: Option<String>,
  pub preview_status: PreviewStatus,
  pub preview_fetched_at: Option<i64>,
  /// Rules revision used to clean `url`.
  pub rules_revision: String,
  pub tags: Vec<BookmarkTag>,
  pub created_at: i64,
  pub updated_at: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PreviewStatus {
  #[default]
  None,
  Pending,
  Done,
  Failed,
}

/// Where a tag on a bookmark came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TagSource {
  #[default]
  User,
  Suggested,
  Ai,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookmarkTag {
  pub name: String,
  pub source: TagSource,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagInput {
  pub name: String,
  #[serde(default)]
  pub source: TagSource,
}

impl TagInput {
  pub fn user(name: impl Into<String>) -> Self {
    Self { name: name.into(), source: TagSource::User }
  }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagCount {
  pub name: String,
  pub count: u64,
}

/// What to save. The store finds and cleans the link itself.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewBookmark {
  pub shared_text: String,
  pub title: Option<String>,
  pub note: Option<String>,
  #[serde(default)]
  pub tags: Vec<TagInput>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Saved {
  pub bookmark: Bookmark,
  /// False if the link was already saved and the new tags were merged into it.
  pub created: bool,
}

/// What the save screen shows before saving.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Prepared {
  /// Every link in the shared text; the first one becomes the bookmark.
  pub links: Vec<link_cleaner::Link>,
  /// The bookmark for the first link, if it's already saved.
  pub existing: Option<Bookmark>,
  /// Title suggested from the shared text around the first link (what `save` would use).
  pub title: Option<String>,
}

/// A bookmark being tagged, for [`crate::Store::suggest_tags`]: the save sheet's current state.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagDraft {
  /// The shared text (or a saved bookmark's link); its first link is the one being tagged.
  pub text: String,
  pub title: Option<String>,
  /// Tags already on it. They're never suggested, and they bring related tags along.
  #[serde(default)]
  pub picked: Vec<String>,
}

/// Fields to change. `None` leaves a field as is; `Some("")` clears it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookmarkPatch {
  pub title: Option<String>,
  pub note: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListQuery {
  /// Words to search for in title, note, shared text, description and tags.
  pub search: Option<String>,
  /// Only bookmarks with all of these tags.
  #[serde(default)]
  pub tags: Vec<String>,
  /// `next` from the previous page.
  pub cursor: Option<String>,
  #[serde(default = "default_limit")]
  pub limit: u32,
}

impl Default for ListQuery {
  fn default() -> Self {
    Self { search: None, tags: Vec::new(), cursor: None, limit: default_limit() }
  }
}

fn default_limit() -> u32 {
  50
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Page<T> {
  pub items: Vec<T>,
  /// Pass as `cursor` to get the next page; `None` on the last page.
  pub next: Option<String>,
}
