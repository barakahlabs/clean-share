//! Link previews for Clean Share bookmarks.
//!
//! [`Fetcher::preview`] builds a [`Preview`] for one (already cleaned) link from up to three
//! sources, best first:
//!
//! 1. **oEmbed**, for providers that offer it without an API key (YouTube, X, TikTok, Spotify,
//!    Vimeo, Reddit, SoundCloud) or that link to it from their page. It works even where the page
//!    itself blocks fetches, and gives the title, author, thumbnail and embed player.
//! 2. **Page metadata**: Open Graph and Twitter card tags, `<meta>` description and keywords,
//!    `article:tag`, icons, `<html lang>`.
//! 3. **JSON-LD** (schema.org) in the page: the kind of page (recipe, article, product, video…),
//!    author, publish date and keywords.
//!
//! It also keeps a short text excerpt of the page, for tag suggestions and on-device AI later.
//!
//! Every request has limits ([`Config`]): a time limit, a size limit (pages are cut off, images
//! and JSON refused), a redirect limit, http(s) only, and no private or local addresses, since a
//! shared link shouldn't make the phone talk to the home router.
//!
//! [`parse_html`] and [`oembed`] are plain functions, so most of the crate is tested without
//! network.

mod fetch;
mod guard;
mod html;
pub mod oembed;
mod thumbnail;

use std::time::Duration;

use serde::{Deserialize, Serialize};

pub use fetch::{Download, Fetcher, Image};
pub use html::{Page, parse_html};
pub use thumbnail::{ICON_SIZE, THUMBNAIL_SIZE, icon, thumbnail};

/// What a link points to, as far as the preview sources say. Every field is optional: sites
/// differ a lot in what they provide.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
  /// The page's address after redirects.
  pub url: String,
  pub title: Option<String>,
  pub description: Option<String>,
  pub site_name: Option<String>,
  /// The main image (thumbnail). Absolute URL; it may still carry tracking, so clean it before
  /// storing or fetching.
  pub image_url: Option<String>,
  /// The site's icon: the largest declared one, or `/favicon.ico`.
  pub icon_url: Option<String>,
  pub author: Option<String>,
  /// What kind of page this is, lowercase: `article`, `recipe`, `video`, `music`, `product`,
  /// `event`, `book`, `movie`, `place`, `person`, `course`, `podcast` or `software`.
  pub kind: Option<String>,
  /// Keywords and article tags the page declares, deduplicated, at most [`MAX_KEYWORDS`].
  pub keywords: Vec<String>,
  /// Publish date as the page gives it (usually ISO 8601).
  pub published: Option<String>,
  /// Language from `<html lang>`, e.g. `en` or `de-DE`.
  pub language: Option<String>,
  /// The start of the page's main text, at most [`MAX_EXCERPT`] characters.
  pub excerpt: Option<String>,
  pub embed: Option<Embed>,
}

/// A player the provider offers (from oEmbed).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Embed {
  /// Provider, lowercase, e.g. `youtube`.
  pub kind: String,
  /// The iframe's address, if the embed is an iframe (YouTube, Vimeo, Spotify do this; X uses a
  /// script instead, so this is `None`).
  pub url: Option<String>,
}

pub const MAX_KEYWORDS: usize = 20;
pub const MAX_EXCERPT: usize = 1000;

/// Request limits. The defaults suit a phone on a mobile connection.
#[derive(Debug, Clone)]
pub struct Config {
  /// For the whole request, including redirects and reading the body.
  pub timeout: Duration,
  /// Pages are read up to this size and parsed as far as they got (metadata is near the top).
  pub max_page_bytes: usize,
  /// oEmbed responses over this size are refused.
  pub max_json_bytes: usize,
  /// Images over this size are refused.
  pub max_image_bytes: usize,
  pub max_redirects: usize,
  /// Allows private and local addresses. Off except in tests.
  pub allow_private: bool,
  pub user_agent: String,
  /// Sent as `Accept-Language`, e.g. the phone's languages (`de-DE,de;q=0.9,en;q=0.8`), so sites
  /// that pick a language by location answer in the user's instead.
  pub accept_language: Option<String>,
}

impl Default for Config {
  fn default() -> Self {
    Self {
      timeout: Duration::from_secs(10),
      max_page_bytes: 2 * 1024 * 1024,
      max_json_bytes: 256 * 1024,
      max_image_bytes: 5 * 1024 * 1024,
      max_redirects: 5,
      allow_private: false,
      // Honest: no pretending to be a browser or a social network's crawler.
      user_agent: format!(
        "CleanShare/{} (link preview; +https://github.com/barakahlabs/clean-share)",
        env!("CARGO_PKG_VERSION")
      ),
      accept_language: None,
    }
  }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
  #[error("not a web link: {0}")]
  InvalidUrl(String),
  #[error("{0} is a private or local address")]
  Blocked(String),
  #[error("the site answered {0}")]
  Status(u16),
  #[error("not a web page ({0})")]
  NotHtml(String),
  #[error("not an image ({0})")]
  NotImage(String),
  #[error("larger than {0} bytes")]
  TooLarge(usize),
  #[error("timed out")]
  Timeout,
  #[error("network error: {0}")]
  Network(String),
  #[error("invalid oEmbed response: {0}")]
  OEmbed(String),
  #[error("can't read the image: {0}")]
  BadImage(String),
}

pub type Result<T> = std::result::Result<T, Error>;

/// Combines the page and oEmbed results. oEmbed is the provider's own answer, so it wins for
/// title, author, thumbnail and site name; the page wins for everything oEmbed doesn't have.
pub fn merge(url: String, page: Option<Page>, oembed: Option<oembed::OEmbed>) -> Preview {
  let mut page = page.unwrap_or_default();
  let oembed = oembed.unwrap_or_default();
  let oembed_kind = oembed.kind().map(str::to_string);
  let embed = oembed.embed();
  // A page that refuses us (or a removed video) often answers with just the site's name as its
  // title ("Reddit", "- YouTube") and the site's stock description and keywords. None of that is
  // about this link, so without oEmbed to say otherwise, it's dropped.
  if page.title.as_deref().is_none_or(|t| is_brand_only(t, &url)) {
    page.title = None;
    if oembed.title.is_none() {
      page.description = None;
      page.keywords.clear();
      page.excerpt = None;
    }
  }
  // "Announcing rustup 1.29.3 | Rust Blog" → "Announcing rustup 1.29.3".
  page.title = page.title.map(|t| without_site_suffix(&t, page.site_name.as_deref(), &url));
  Preview {
    title: oembed.title.or(page.title),
    description: page.description.or(oembed.summary),
    site_name: page.site_name.or(oembed.provider_name),
    image_url: oembed.thumbnail_url.or(page.image_url),
    icon_url: page.icon_url,
    author: oembed.author_name.or(page.author),
    kind: page.kind.or(oembed_kind),
    keywords: page.keywords,
    published: page.published.or(oembed.publication_date),
    language: page.language,
    excerpt: page.excerpt,
    embed,
    url,
  }
}

/// Title separators around a site's name: "Page | Site", "Page - Site", "Page — Site".
const SEPARATORS: &[&str] = &[" | ", " - ", " – ", " — ", " · ", " :: "];

/// `title` without a trailing " | Site" when that part names the site: its `og:site_name`, or a
/// word of the host's name ("Rust Blog" on blog.rust-lang.org). Kept if nothing would be left.
fn without_site_suffix(title: &str, site_name: Option<&str>, url: &str) -> String {
  let Some((head, tail)) = SEPARATORS.iter().filter_map(|sep| title.rsplit_once(sep)).max_by_key(|(head, _)| head.len()) else {
    return title.to_string();
  };
  let tail_lower = tail.trim().to_lowercase();
  let names_site = site_name.is_some_and(|s| s.trim().eq_ignore_ascii_case(&tail_lower))
    || brand(url).is_some_and(|brand| {
      brand.split('-').filter(|w| w.len() >= 3).any(|w| tail_lower.split_whitespace().any(|t| t == w))
    });
  if names_site && tail.split_whitespace().count() <= 4 && !head.trim().is_empty() {
    head.trim().to_string()
  } else {
    title.to_string()
  }
}

/// True if `title` is only the site's name, e.g. "Reddit" for `www.reddit.com/r/rust/`.
fn is_brand_only(title: &str, url: &str) -> bool {
  let Some(brand) = brand(url) else { return false };
  // "- YouTube", "Reddit |": a site's title with the page's part left empty.
  let title = title.trim_matches(|c: char| c.is_whitespace() || matches!(c, '-' | '–' | '—' | '|' | '·' | ':'));
  title.eq_ignore_ascii_case(&brand)
}

/// The site's name from its host: "reddit" for www.reddit.com, "bbc" for www.bbc.co.uk,
/// "rust-lang" for blog.rust-lang.org.
fn brand(url: &str) -> Option<String> {
  let host = url::Url::parse(url).ok()?.host_str()?.to_ascii_lowercase();
  // The name before the public suffix: "reddit" in www.reddit.com, "bbc" in www.bbc.co.uk.
  let labels: Vec<&str> = host.split('.').collect();
  let n = labels.len();
  let two_part_suffix = n >= 3
    && labels[n - 1].len() == 2
    && matches!(labels[n - 2], "co" | "com" | "org" | "net" | "ac" | "gov");
  match (n, two_part_suffix) {
    (_, true) => Some(labels[n - 3].to_string()),
    (2.., false) => Some(labels[n - 2].to_string()),
    _ => None,
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn titles_that_are_just_the_site_name() {
    assert!(is_brand_only("Reddit", "https://www.reddit.com/r/rust/"));
    assert!(is_brand_only(" bbc ", "https://www.bbc.co.uk/news"));
    assert!(is_brand_only("- YouTube", "https://www.youtube.com/watch?v=abc123"));
    assert!(!is_brand_only("Reddit - Dive into anything", "https://www.reddit.com/"));
    assert!(!is_brand_only("Rust", "https://www.reddit.com/r/rust/"));
    assert!(!is_brand_only("Reddit", "not a url"));
  }

  #[test]
  fn a_brand_only_page_title_gives_way() {
    let page = Page { title: Some("Reddit".into()), ..Default::default() };
    assert_eq!(merge("https://www.reddit.com/r/rust/".into(), Some(page), None).title, None);
  }

  #[test]
  fn a_page_without_its_own_title_has_only_the_sites_stock_text() {
    // A removed YouTube video: empty title, YouTube's own description and keywords.
    let page = Page {
      title: Some("- YouTube".into()),
      description: Some("Enjoy the videos and music you love, upload original content".into()),
      keywords: vec!["video".into(), "upload".into()],
      icon_url: Some("https://www.youtube.com/favicon.ico".into()),
      ..Default::default()
    };
    let p = merge("https://www.youtube.com/watch?v=gone".into(), Some(page), None);
    assert_eq!((p.title, p.description), (None, None));
    assert!(p.keywords.is_empty());
    // The icon is still the site's.
    assert_eq!(p.icon_url.as_deref(), Some("https://www.youtube.com/favicon.ico"));
  }

  #[test]
  fn the_sites_name_is_taken_off_the_end_of_a_title() {
    let url = "https://blog.rust-lang.org/2026/11/03/Rustup-1.29.3/";
    assert_eq!(without_site_suffix("Announcing rustup 1.29.3 | Rust Blog", None, url), "Announcing rustup 1.29.3");
    assert_eq!(
      without_site_suffix("City opens bike lanes - The Guardian", Some("The Guardian"), "https://www.theguardian.com/x"),
      "City opens bike lanes"
    );
    // Not the site's name: kept.
    assert_eq!(without_site_suffix("Cats | Dogs", None, url), "Cats | Dogs");
    assert_eq!(without_site_suffix("Rust Blog", None, url), "Rust Blog");
  }

  #[test]
  fn oembed_summary_and_date_fill_in_for_a_blocked_page() {
    let o = oembed::parse(include_bytes!("../tests/fixtures/nytimes-oembed.json")).unwrap();
    let p = merge("https://www.nytimes.com/2026/11/03/technology/binance-hack-report.html".into(), None, Some(o));
    assert!(p.description.unwrap().starts_with("A report from"));
    assert_eq!(p.published.as_deref(), Some("November 3, 2026"));
    assert_eq!(p.kind.as_deref(), Some("article"));
    assert_eq!(p.site_name.as_deref(), Some("The New York Times"));
  }
}
