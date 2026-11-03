//! oEmbed (<https://oembed.com>): the provider describes a link in JSON. Used for the providers
//! below, which answer without an API key, and for pages that link to their own oEmbed JSON.
//!
//! Meta's providers (Instagram, Facebook, Threads) need an app token, so they're not listed;
//! their pages' Open Graph tags are used instead.

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
}

pub fn parse(json: &[u8]) -> Result<OEmbed> {
  let mut parsed: OEmbed =
    serde_json::from_slice(json).map_err(|e| Error::OEmbed(e.to_string()))?;
  for field in [
    &mut parsed.title,
    &mut parsed.author_name,
    &mut parsed.provider_name,
    &mut parsed.thumbnail_url,
  ] {
    *field = field.take().map(|v| v.trim().to_string()).filter(|v| !v.is_empty());
  }
  // Only absolute web links count as thumbnails.
  parsed.thumbnail_url = parsed.thumbnail_url.filter(|u| u.starts_with("https://") || u.starts_with("http://"));
  Ok(parsed)
}

impl OEmbed {
  /// `video` for video embeds; other oEmbed types don't say what the page is about.
  pub fn kind(&self) -> Option<&str> {
    (self.kind_raw.as_deref() == Some("video")).then_some("video")
  }

  /// The player, if the response has one: the provider name, and the iframe's address when the
  /// embed is an iframe.
  pub fn embed(&self) -> Option<Embed> {
    let html = self.html.as_deref()?;
    let kind = self.provider_name.as_deref()?.to_lowercase();
    // X calls itself "Twitter" in oEmbed.
    let kind = if kind == "twitter" { "x".to_string() } else { kind };
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
  fn bad_json_is_an_error() {
    assert!(matches!(parse(b"<html>"), Err(Error::OEmbed(_))));
  }
}
