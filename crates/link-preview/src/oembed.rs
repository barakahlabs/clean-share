//! oEmbed (<https://oembed.com>): the provider describes a link in JSON. Used for the providers
//! below, which answer without an API key, and for pages that link to their own oEmbed JSON.
//!
//! Meta's providers (Instagram, Facebook, Threads) need an app token, so they're not listed;
//! their pages' Open Graph tags are used instead.
//!
//! Besides the standard fields, some providers add their own: the New York Times sends
//! `summary`, `publication_date` and `asset_type`.

use scraper::{Html, Selector};
use serde::Deserialize;
use url::Url;

use crate::{Embed, Error, Result};

/// Host suffix → (provider, endpoint). The link goes in the `url` query parameter.
const PROVIDERS: &[(&str, &str, &str)] = &[
  ("youtube.com", "youtube", "https://www.youtube.com/oembed?format=json"),
  ("youtu.be", "youtube", "https://www.youtube.com/oembed?format=json"),
  ("x.com", "x", "https://publish.twitter.com/oembed?omit_script=true"),
  ("twitter.com", "x", "https://publish.twitter.com/oembed?omit_script=true"),
  ("tiktok.com", "tiktok", "https://www.tiktok.com/oembed"),
  ("open.spotify.com", "spotify", "https://open.spotify.com/oembed"),
  ("vimeo.com", "vimeo", "https://vimeo.com/api/oembed.json"),
  ("reddit.com", "reddit", "https://www.reddit.com/oembed"),
  ("soundcloud.com", "soundcloud", "https://soundcloud.com/oembed?format=json"),
  // Its pages refuse fetches that aren't from a browser; its oEmbed answers.
  ("nytimes.com", "nytimes", "https://www.nytimes.com/svc/oembed/json/"),
];

/// The oEmbed endpoint for a link, if its host is a known provider.
pub fn endpoint(link: &Url) -> Option<Url> {
  let host = link.host_str()?.to_ascii_lowercase();
  let (_, _, endpoint) = PROVIDERS
    .iter()
    .find(|(suffix, ..)| host == *suffix || host.ends_with(&format!(".{suffix}")))?;
  let mut endpoint = Url::parse(endpoint).ok()?;
  endpoint.query_pairs_mut().append_pair("url", link.as_str());
  Some(endpoint)
}

/// The fields of an oEmbed response that previews use. Providers fill different subsets.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct OEmbed {
  /// `video`, `photo`, `rich` or `link`.
  #[serde(rename = "type")]
  pub kind_raw: Option<String>,
  pub title: Option<String>,
  pub author_name: Option<String>,
  pub provider_name: Option<String>,
  pub thumbnail_url: Option<String>,
  pub html: Option<String>,
  /// Not standard oEmbed: a short description (New York Times).
  pub summary: Option<String>,
  /// Not standard: e.g. "November 3, 2026" (New York Times).
  pub publication_date: Option<String>,
  /// Not standard: e.g. "article" (New York Times).
  pub asset_type: Option<String>,
}

pub fn parse(json: &[u8]) -> Result<OEmbed> {
  let mut parsed: OEmbed =
    serde_json::from_slice(json).map_err(|e| Error::OEmbed(e.to_string()))?;
  for field in [
    &mut parsed.title,
    &mut parsed.author_name,
    &mut parsed.provider_name,
    &mut parsed.thumbnail_url,
    &mut parsed.summary,
    &mut parsed.publication_date,
  ] {
    *field = field.take().map(|v| v.trim().to_string()).filter(|v| !v.is_empty());
  }
  // "By Emmy Martin" → "Emmy Martin".
  parsed.author_name = parsed.author_name.map(|a| a.strip_prefix("By ").unwrap_or(&a).to_string());
  // Reddit calls itself "reddit"; show it as a name.
  parsed.provider_name = parsed.provider_name.map(|name| {
    if name.chars().all(|c| !c.is_uppercase()) {
      let mut chars = name.chars();
      chars.next().map(|c| c.to_uppercase().chain(chars).collect()).unwrap_or_default()
    } else {
      name
    }
  });
  // Only absolute web links count as thumbnails.
  parsed.thumbnail_url = parsed.thumbnail_url.filter(|u| u.starts_with("https://") || u.starts_with("http://"));
  Ok(parsed)
}

impl OEmbed {
  /// `video` for video embeds, `article` when the provider says so; other oEmbed types don't
  /// say what the page is about.
  pub fn kind(&self) -> Option<&str> {
    match (self.kind_raw.as_deref(), self.asset_type.as_deref()) {
      (Some("video"), _) => Some("video"),
      (_, Some("article")) => Some("article"),
      _ => None,
    }
  }

  /// The player, if the response has one: the provider name, and the iframe's address when the
  /// embed is an iframe.
  pub fn embed(&self) -> Option<Embed> {
    let html = self.html.as_deref()?;
    let kind = self.provider_name.as_deref()?.to_lowercase();
    // X calls itself "Twitter" in oEmbed.
    let kind = match kind.as_str() {
      "twitter" => "x".to_string(),
      "the new york times" => "nytimes".to_string(),
      _ => kind,
    };
    let iframe = Selector::parse("iframe[src]").unwrap();
    let url = Html::parse_fragment(html)
      .select(&iframe)
      .next()
      .and_then(|el| el.value().attr("src"))
      .filter(|src| src.starts_with("https://"))
      .map(str::to_string);
    Some(Embed { kind, url })
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn known_providers_get_their_endpoint() {
    let link = Url::parse("https://youtu.be/dQw4w9WgXcQ").unwrap();
    assert_eq!(
      endpoint(&link).unwrap().as_str(),
      "https://www.youtube.com/oembed?format=json&url=https%3A%2F%2Fyoutu.be%2FdQw4w9WgXcQ"
    );
    let link = Url::parse("https://m.youtube.com/watch?v=abc").unwrap();
    assert!(endpoint(&link).unwrap().as_str().starts_with("https://www.youtube.com/oembed"));
    let link = Url::parse("https://x.com/user/status/1").unwrap();
    assert!(endpoint(&link).unwrap().as_str().starts_with("https://publish.twitter.com/oembed"));
  }

  #[test]
  fn other_hosts_have_none() {
    for link in ["https://example.com/", "https://notyoutube.com/x", "https://www.instagram.com/p/1/"] {
      assert_eq!(endpoint(&Url::parse(link).unwrap()), None, "{link}");
    }
  }

  #[test]
  fn parses_a_youtube_response() {
    let json = include_bytes!("../tests/fixtures/youtube-oembed.json");
    let o = parse(json).unwrap();
    assert_eq!(o.title.as_deref(), Some("Rust in 100 Seconds"));
    assert_eq!(o.author_name.as_deref(), Some("Fireship"));
    assert_eq!(o.thumbnail_url.as_deref(), Some("https://i.ytimg.com/vi/5C_HPTJg5ek/hqdefault.jpg"));
    assert_eq!(o.kind(), Some("video"));
    assert_eq!(
      o.embed(),
      Some(Embed {
        kind: "youtube".into(),
        url: Some("https://www.youtube.com/embed/5C_HPTJg5ek?feature=oembed".into())
      })
    );
  }

  #[test]
  fn an_x_embed_has_no_iframe() {
    let o = parse(include_bytes!("../tests/fixtures/x-oembed.json")).unwrap();
    assert_eq!(o.author_name.as_deref(), Some("Rust Language"));
    assert_eq!(o.kind(), None);
    assert_eq!(o.embed(), Some(Embed { kind: "x".into(), url: None }));
  }

  #[test]
  fn a_new_york_times_response_has_summary_date_and_kind() {
    let o = parse(include_bytes!("../tests/fixtures/nytimes-oembed.json")).unwrap();
    assert_eq!(o.title.as_deref(), Some("Binance Says Hackers Plotted $6.3 Billion Heist for Months"));
    assert_eq!(o.author_name.as_deref(), Some("Emmy Martin"));
    assert!(o.summary.as_deref().unwrap().starts_with("A report from the cryptocurrency exchange"));
    assert_eq!(o.publication_date.as_deref(), Some("November 3, 2026"));
    assert_eq!(o.kind(), Some("article"));
    assert!(o.thumbnail_url.as_deref().unwrap().starts_with("https://static01.nyt.com/"));
  }

  #[test]
  fn a_reddit_response_gets_a_capitalised_site_name() {
    let o = parse(include_bytes!("../tests/fixtures/reddit-oembed.json")).unwrap();
    assert_eq!(o.title.as_deref(), Some("Test whether an Option contains a certain value"));
    assert_eq!(o.provider_name.as_deref(), Some("Reddit"));
    assert_eq!(o.author_name.as_deref(), Some("Thick-Pineapple666"));
  }

  #[test]
  fn bad_json_is_an_error() {
    assert!(matches!(parse(b"<html>"), Err(Error::OEmbed(_))));
  }
}
