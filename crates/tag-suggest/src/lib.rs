//! Tag suggestions for Clean Share bookmarks.
//!
//! A [`TagSuggester`] ranks tags for one bookmark from what it's about ([`Content`]) and how the
//! user has tagged before ([`History`]). It never touches the database: the caller (the
//! bookmarks store) gathers the history, so suggesters stay plain functions that are easy to
//! test, and later ones (on-device or cloud AI, `docs/BOOKMARKS.md` "Tagging") fit the same
//! trait.
//!
//! [`MatchSuggester`] is tier 1: offline, on every phone. It combines five reasons:
//!
//! - [`Reason::Content`]: an existing tag whose name appears in the title, text or link
//!   (`video` also matches "videos").
//! - [`Reason::Site`]: tags the user put on other bookmarks from the same site.
//! - [`Reason::Related`]: tags often used together with the ones already picked.
//! - [`Reason::Provider`]: a small built-in map, e.g. YouTube → `video`.
//! - [`Reason::Page`]: what the page says about itself in its preview: its declared keywords
//!   (an existing tag that matches one ranks high; a few others are offered as new tags) and its
//!   kind (a recipe page suggests `recipe`).

mod words;

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// What the bookmark is about.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Content {
  /// Title, shared text, the cleaned link (its host and path words count too) and, with a
  /// preview, the page's description and excerpt.
  pub text: String,
  /// Provider id from the cleaning rules (e.g. `youtube`), if one matched.
  pub provider: Option<String>,
  /// Keywords the page declares (from its preview).
  pub keywords: Vec<String>,
  /// What the page is, from its preview: `recipe`, `article`, `video`…
  pub kind: Option<String>,
}

/// How the user has tagged so far. `site` and `related` leave out the bookmark being tagged.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct History {
  /// Every existing tag with how many bookmarks use it.
  pub tags: Vec<TagUse>,
  /// Tags on the other bookmarks from the same site, and how many such bookmarks there are.
  pub site: Uses,
  /// Tags on the bookmarks that have any of the picked tags, and how many such bookmarks there
  /// are.
  pub related: Uses,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagUse {
  pub name: String,
  pub count: u32,
}

/// Tag counts over a group of bookmarks.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Uses {
  pub tags: Vec<TagUse>,
  /// Bookmarks in the group, tagged or not.
  pub bookmarks: u32,
}

/// Why a tag was suggested: the reason that added most to its score.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Reason {
  Content,
  Site,
  Related,
  Provider,
  Page,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Suggestion {
  /// An existing tag's name (as the user wrote it), or a new one from the provider map.
  pub name: String,
  pub reason: Reason,
  pub score: f32,
}

pub trait TagSuggester: Send + Sync {
  /// Best first, at most `limit`, never one of `picked` (compared case-insensitively).
  fn suggest(
    &self,
    content: &Content,
    history: &History,
    picked: &[String],
    limit: usize,
  ) -> Vec<Suggestion>;
}

/// Offline suggestions from simple matching. See the crate docs for the reasons it uses.
#[derive(Debug, Clone, Copy, Default)]
pub struct MatchSuggester;

/// Scores per reason. A tag can earn several; they add up.
const CONTENT: f32 = 3.0;
/// For site and related tags, scaled by the share of bookmarks that have the tag, plus a base so
/// that a tag used once on that site still shows.
const SITE: f32 = 2.0;
const SITE_BASE: f32 = 0.5;
const RELATED: f32 = 1.5;
const PROVIDER: f32 = 1.0;
/// A page keyword that matches an existing tag.
const PAGE_MATCH: f32 = 2.5;
/// A page keyword offered as a new tag; only the first few, short ones.
const PAGE_NEW: f32 = 0.75;
const PAGE_NEW_MAX: usize = 3;
const PAGE_KIND: f32 = 1.0;

/// Provider id (from the cleaning rules) → tag. Kept short and generic on purpose.
const PROVIDER_TAGS: &[(&str, &str)] = &[
  ("youtube", "video"),
  ("tiktok", "video"),
  ("spotify", "music"),
  ("instagram", "social"),
  ("x", "social"),
  ("threads", "social"),
  ("facebook", "social"),
];

/// Page kind (from `crates/link-preview`) → tag. Kinds like `article` say too little to be a tag.
const KIND_TAGS: &[(&str, &str)] = &[
  ("recipe", "recipe"),
  ("video", "video"),
  ("music", "music"),
  ("podcast", "podcast"),
  ("book", "book"),
  ("movie", "movie"),
  ("event", "event"),
  ("product", "shopping"),
];

impl TagSuggester for MatchSuggester {
  fn suggest(
    &self,
    content: &Content,
    history: &History,
    picked: &[String],
    limit: usize,
  ) -> Vec<Suggestion> {
    let text = words::stems(&content.text);
    let mut ranking = Ranking::new(history, picked);

    for tag in &history.tags {
      if words::contains(&text, &words::stems(&tag.name)) {
        ranking.add(&tag.name, Reason::Content, CONTENT);
      }
    }
    for (uses, weight, base, reason) in [
      (&history.site, SITE, SITE_BASE, Reason::Site),
      (&history.related, RELATED, 0.0, Reason::Related),
    ] {
      let total = uses.bookmarks.max(1) as f32;
      for tag in &uses.tags {
        ranking.add(&tag.name, reason, base + weight * (tag.count as f32 / total).min(1.0));
      }
    }
    if let Some(provider) = content.provider.as_deref() {
      for (_, tag) in PROVIDER_TAGS.iter().filter(|(id, _)| *id == provider) {
        ranking.add(tag, Reason::Provider, PROVIDER);
      }
    }
    let mut new_keywords = 0;
    for keyword in &content.keywords {
      if ranking.is_known(keyword) {
        ranking.add(keyword, Reason::Page, PAGE_MATCH);
      } else if new_keywords < PAGE_NEW_MAX && usable_as_tag(keyword) {
        ranking.add(keyword, Reason::Page, PAGE_NEW);
        new_keywords += 1;
      }
    }
    if let Some(kind) = content.kind.as_deref() {
      for (_, tag) in KIND_TAGS.iter().filter(|(k, _)| *k == kind) {
        ranking.add(tag, Reason::Page, PAGE_KIND);
      }
    }
    ranking.best(limit)
  }
}

/// Short enough to be a tag, and more than a number: at most 3 words and 25 characters.
fn usable_as_tag(keyword: &str) -> bool {
  let words = keyword.split_whitespace().count();
  (1..=3).contains(&words)
    && keyword.chars().count() <= 25
    && keyword.chars().any(char::is_alphabetic)
}

/// Collects scores per tag, keyed by the tag's stemmed name so `Video` and `videos` are one tag.
struct Ranking<'a> {
  /// Existing tag names and uses by key, to show the user's spelling and break ties.
  known: HashMap<String, &'a TagUse>,
  picked: Vec<String>,
  scores: HashMap<String, Entry>,
}

struct Entry {
  name: String,
  total: f32,
  /// Score per reason, to report the strongest one.
  parts: Vec<(Reason, f32)>,
}

impl<'a> Ranking<'a> {
  fn new(history: &'a History, picked: &[String]) -> Self {
    Self {
      known: history.tags.iter().map(|tag| (words::key(&tag.name), tag)).collect(),
      picked: picked.iter().map(|name| words::key(name)).collect(),
      scores: HashMap::new(),
    }
  }

  fn is_known(&self, name: &str) -> bool {
    self.known.contains_key(&words::key(name))
  }

  fn add(&mut self, name: &str, reason: Reason, score: f32) {
    let key = words::key(name);
    if key.is_empty() || self.picked.contains(&key) {
      return;
    }
    let name = self.known.get(&key).map_or(name, |tag| tag.name.as_str()).to_string();
    let entry =
      self.scores.entry(key).or_insert_with(|| Entry { name, total: 0.0, parts: Vec::new() });
    entry.total += score;
    match entry.parts.iter_mut().find(|(r, _)| *r == reason) {
      Some((_, part)) => *part += score,
      None => entry.parts.push((reason, score)),
    }
  }

  fn best(self, limit: usize) -> Vec<Suggestion> {
    let uses = |key: &str| self.known.get(key).map_or(0, |tag| tag.count);
    let mut ranked: Vec<_> = self.scores.iter().collect();
    ranked.sort_by(|(ka, a), (kb, b)| {
      b.total
        .total_cmp(&a.total)
        .then_with(|| uses(kb).cmp(&uses(ka)))
        .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    ranked
      .into_iter()
      .take(limit)
      .map(|(_, entry)| Suggestion {
        name: entry.name.clone(),
        reason: entry.parts.iter().max_by(|a, b| a.1.total_cmp(&b.1)).map(|p| p.0).unwrap(),
        score: entry.total,
      })
      .collect()
  }
}

#[cfg(test)]
mod tests;
