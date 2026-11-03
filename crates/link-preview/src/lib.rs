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

pub use fetch::{Fetcher, Image};
pub use html::{Page, parse_html};
pub use thumbnail::{THUMBNAIL_SIZE, thumbnail};

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
  // A page that refuses us often answers with just the site's name as its title ("Reddit"):
  // that's not this page's title.
  if page.title.as_deref().is_some_and(|t| is_brand_only(t, &url)) {
    page.title = None;
  }
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

/// True if `title` is only the site's name, e.g. "Reddit" for `www.reddit.com/r/rust/`.
fn is_brand_only(title: &str, url: &str) -> bool {
  let Some(host) = url::Url::parse(url).ok().and_then(|u| u.host_str().map(str::to_ascii_lowercase))
  else {
    return false;
  };
  // The name before the public suffix: "reddit" in www.reddit.com, "bbc" in www.bbc.co.uk.
  let labels: Vec<&str> = host.split('.').collect();
  let n = labels.len();
  let two_part_suffix = n >= 3
    && labels[n - 1].len() == 2
    && matches!(labels[n - 2], "co" | "com" | "org" | "net" | "ac" | "gov");
  let brand = match (n, two_part_suffix) {
    (_, true) => labels[n - 3],
    (2.., false) => labels[n - 2],
    _ => return false,
  };
  title.trim().eq_ignore_ascii_case(brand)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn titles_that_are_just_the_site_name() {
    assert!(is_brand_only("Reddit", "https://www.reddit.com/r/rust/"));
    assert!(is_brand_only(" bbc ", "https://www.bbc.co.uk/news"));
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
  fn oembed_summary_and_date_fill_in_for_a_blocked_page() {
    let o = oembed::parse(include_bytes!("../tests/fixtures/nytimes-oembed.json")).unwrap();
    let p = merge("https://www.nytimes.com/2026/11/03/technology/binance-hack-report.html".into(), None, Some(o));
    assert!(p.description.unwrap().starts_with("A report from"));
    assert_eq!(p.published.as_deref(), Some("November 3, 2026"));
    assert_eq!(p.kind.as_deref(), Some("article"));
    assert_eq!(p.site_name.as_deref(), Some("The New York Times"));
  }
}
