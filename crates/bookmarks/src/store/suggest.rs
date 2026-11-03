//! Tag suggestions for the save sheet. The store gathers the history (tags, same-site bookmarks,
//! tags used together with the picked ones) and `tag-suggest` ranks them.

use std::collections::HashMap;

use tag_suggest::{Content, History, MatchSuggester, Suggestion, TagSuggester, TagUse, Uses};
use turso::Value;

use super::{Store, row, tags, title_from};
use crate::{PageInfo, Result, TagDraft, ids};

/// Chips shown on the save sheet.
const LIMIT: usize = 5;
/// History looks at this many bookmarks per group at most (newest first), so it stays fast.
const SAMPLE: u32 = 500;

impl Store {
  /// Tags to suggest for a bookmark being saved or edited, best first, never one already picked.
  pub async fn suggest_tags(&self, draft: &TagDraft) -> Result<Vec<Suggestion>> {
    let links = self.cleaner.links(&draft.text);
    let link = links.first();

    // What the page says about itself: from the draft (a preview fetched for the save sheet),
    // else from the stored preview when the link is already saved.
    let page = match (&draft.page, link) {
      (Some(page), _) => page.clone(),
      (None, Some(link)) => match self.find_by_url(&link.cleaned).await? {
        Some(saved) => PageInfo {
          description: saved.description,
          excerpt: saved.excerpt,
          keywords: saved.keywords,
          kind: saved.page_kind,
        },
        None => PageInfo::default(),
      },
      (None, None) => PageInfo::default(),
    };

    // The text without its links (they carry tracking), the title, the cleaned link, and the
    // page's description and excerpt.
    let shared = title_from(&draft.text, &links).unwrap_or_default();
    let mut text = shared.clone();
    let parts = [
      draft.title.as_deref(),
      link.map(|l| l.cleaned.as_str()),
      page.description.as_deref(),
      page.excerpt.as_deref(),
    ];
    for part in parts.into_iter().flatten() {
      text.push(' ');
      text.push_str(part);
    }
    let summary =
      [page.description.as_deref(), page.excerpt.as_deref()].into_iter().flatten().collect::<Vec<_>>().join(" ");
    let content = Content {
      text,
      provider: link.and_then(|l| l.provider.clone()),
      keywords: page.keywords,
      kind: page.kind,
      title: draft.title.clone().unwrap_or_default(),
      summary,
      shared,
      host: link.and_then(|l| host_of(&l.cleaned)),
    };

    // Counts leave out the bookmark itself (when editing), so it doesn't suggest its own tags.
    let this = link.map_or("", |l| l.cleaned.as_str());
    let site = match link.and_then(|l| Site::of(&l.cleaned, l.provider.as_deref())) {
      Some(site) => self.site_uses(&site, this).await?,
      None => Uses::default(),
    };
    let history = History {
      tags: self
        .tags()
        .await?
        .into_iter()
        .map(|t| TagUse { name: t.name, count: t.count as u32 })
        .collect(),
      site,
      related: self.related_uses(&draft.picked, this).await?,
    };
    Ok(MatchSuggester.suggest(&content, &history, &draft.picked, LIMIT))
  }

  /// Tags on the other bookmarks from the same site.
  async fn site_uses(&self, site: &Site, this: &str) -> Result<Uses> {
    let ids = match site {
      Site::Provider(provider) => {
        self
          .ids(
            "SELECT id, url FROM bookmark WHERE provider = ?1 AND url != ?2
             ORDER BY created_at DESC",
            vec![Value::from(provider.as_str()), Value::from(this)],
            |_| true,
          )
          .await?
      }
      // `instr` narrows it down; the host check makes it exact (`x.com` isn't `box.com`).
      Site::Host(host) => {
        self
          .ids(
            "SELECT id, url FROM bookmark WHERE instr(url, ?1) > 0 AND url != ?2
             ORDER BY created_at DESC",
            vec![Value::from(host.as_str()), Value::from(this)],
            |url| host_of(url).as_deref() == Some(host.as_str()),
          )
          .await?
      }
    };
    self.uses(ids).await
  }

  /// Tags on the bookmarks that have any of the picked tags.
  async fn related_uses(&self, picked: &[String], this: &str) -> Result<Uses> {
    let tag_ids: Vec<String> =
      picked.iter().filter_map(|name| tags::normalize(name)).map(|name| ids::tag_id(&name)).collect();
    if tag_ids.is_empty() {
      return Ok(Uses::default());
    }
    let placeholders = vec!["?"; tag_ids.len()].join(", ");
    let sql = format!(
      "SELECT DISTINCT b.id, b.url, b.created_at
       FROM bookmark b JOIN bookmark_tag bt ON bt.bookmark_id = b.id
       WHERE bt.tag_id IN ({placeholders}) AND b.url != ?
       ORDER BY b.created_at DESC"
    );
    let mut params: Vec<Value> = tag_ids.iter().map(|id| Value::from(id.as_str())).collect();
    params.push(Value::from(this));
    let ids = self.ids(&sql, params, |_| true).await?;
    self.uses(ids).await
  }

  /// Runs `sql` (selecting id and url first, newest first) and keeps the ids whose url passes
  /// `keep`, at most [`SAMPLE`].
  async fn ids(
    &self,
    sql: &str,
    params: Vec<Value>,
    keep: impl Fn(&str) -> bool,
  ) -> Result<Vec<String>> {
    let mut rows = self.conn.query(sql, params).await?;
    let mut ids = Vec::new();
    while let Some(r) = rows.next().await? {
      if keep(&row::text(&r, 1, "url")?) {
        ids.push(row::text(&r, 0, "id")?);
        if ids.len() as u32 == SAMPLE {
          break;
        }
      }
    }
    Ok(ids)
  }

  /// How often each tag appears on these bookmarks.
  async fn uses(&self, bookmark_ids: Vec<String>) -> Result<Uses> {
    let mut counts: HashMap<String, u32> = HashMap::new();
    for tag in tags::of(&self.conn, &bookmark_ids).await?.into_values().flatten() {
      *counts.entry(tag.name).or_default() += 1;
    }
    Ok(Uses {
      tags: counts.into_iter().map(|(name, count)| TagUse { name, count }).collect(),
      bookmarks: bookmark_ids.len() as u32,
    })
  }
}

/// What counts as "the same site": the rules' provider when one matched (so `youtu.be` and
/// `youtube.com` are one site), otherwise the host without `www.`.
enum Site {
  Provider(String),
  Host(String),
}

impl Site {
  fn of(url: &str, provider: Option<&str>) -> Option<Self> {
    match provider {
      Some(provider) => Some(Self::Provider(provider.to_string())),
      None => host_of(url).map(Self::Host),
    }
  }
}

/// `https://www.Example.com:8080/a` → `example.com`.
fn host_of(url: &str) -> Option<String> {
  let rest = url.split_once("://")?.1;
  let authority = rest.split(['/', '?', '#']).next()?;
  let host = authority.rsplit('@').next()?.split(':').next()?.to_lowercase();
  let host = host.strip_prefix("www.").unwrap_or(&host);
  (!host.is_empty()).then(|| host.to_string())
}

#[cfg(test)]
mod tests {
  use super::host_of;

  #[test]
  fn hosts_drop_www_port_and_user() {
    assert_eq!(host_of("https://www.Example.com:8080/a?b#c").as_deref(), Some("example.com"));
    assert_eq!(host_of("https://user@blog.example.com").as_deref(), Some("blog.example.com"));
    assert_eq!(host_of("not a url"), None);
  }
}
