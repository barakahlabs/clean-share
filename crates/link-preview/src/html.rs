//! Reads a page's metadata: Open Graph and Twitter card tags, `<meta>` tags, JSON-LD, icons, and
//! the start of its main text.

use scraper::{ElementRef, Html, Node, Selector};
use serde_json::Value;
use url::Url;

use crate::{MAX_EXCERPT, MAX_KEYWORDS};

/// What one page says about itself. Relative links are made absolute.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Page {
  pub title: Option<String>,
  pub description: Option<String>,
  pub site_name: Option<String>,
  pub image_url: Option<String>,
  pub icon_url: Option<String>,
  pub author: Option<String>,
  pub kind: Option<String>,
  pub keywords: Vec<String>,
  pub published: Option<String>,
  pub language: Option<String>,
  pub excerpt: Option<String>,
  /// The page's own oEmbed JSON, if it links to one.
  pub oembed_url: Option<String>,
}

const MAX_TITLE: usize = 300;
const MAX_DESCRIPTION: usize = 1000;
const MAX_KEYWORD: usize = 40;
/// Text under these elements isn't part of the main text.
const NOT_TEXT: &[&str] = &[
  "script", "style", "noscript", "template", "nav", "header", "footer", "aside", "form", "button",
  "svg", "iframe", "select",
];

pub fn parse_html(html: &str, page_url: &Url) -> Page {
  let doc = Html::parse_document(html);
  let base = select(&doc, "base[href]")
    .next()
    .and_then(|el| page_url.join(el.value().attr("href")?).ok())
    .unwrap_or_else(|| page_url.clone());
  let meta = Meta::read(&doc);
  let ld = JsonLd::read(&doc);
  let absolute = |link: &str| absolute(&base, link);

  Page {
    title: meta
      .first(&["og:title", "twitter:title"])
      .or_else(|| ld.text(&["headline", "name"]))
      .or_else(|| select(&doc, "title").next().map(|el| el.text().collect()))
      .and_then(|t| clean(&t, MAX_TITLE)),
    description: meta
      .first(&["og:description", "twitter:description", "description"])
      .or_else(|| ld.text(&["description"]))
      .and_then(|t| clean(&t, MAX_DESCRIPTION)),
    site_name: meta
      .first(&["og:site_name", "application-name"])
      .or_else(|| ld.name_of("publisher"))
      .and_then(|t| clean(&t, MAX_TITLE)),
    image_url: meta
      .all(&["og:image:secure_url", "og:image", "og:image:url", "twitter:image", "twitter:image:src"])
      .into_iter()
      .chain(ld.image())
      .find_map(|link| absolute(&link)),
    icon_url: icon(&doc, &base),
    author: meta
      .all(&["author", "article:author"])
      .into_iter()
      // `article:author` is often a profile link, not a name.
      .find(|a| !a.starts_with("http"))
      .or_else(|| ld.name_of("author"))
      .and_then(|t| clean(&t, MAX_TITLE)),
    kind: ld.kind().or_else(|| meta.first(&["og:type"]).and_then(|t| og_kind(&t))).map(str::to_string),
    keywords: keywords(&meta, &ld),
    published: meta
      .first(&["article:published_time", "og:video:release_date"])
      .or_else(|| ld.text(&["datePublished", "uploadDate"]))
      .and_then(|t| clean(&t, 64)),
    language: select(&doc, "html[lang]")
      .next()
      .and_then(|el| clean(el.value().attr("lang")?, 35)),
    excerpt: excerpt(&doc),
    oembed_url: select(&doc, r#"link[rel~="alternate"][type="application/json+oembed"][href]"#)
      .next()
      .and_then(|el| absolute(el.value().attr("href")?)),
  }
}

fn select<'a>(doc: &'a Html, css: &str) -> impl Iterator<Item = ElementRef<'a>> {
  let selector = Selector::parse(css).expect("valid selector");
  doc.select(&selector).collect::<Vec<_>>().into_iter()
}

/// Whitespace collapsed, at most `max` characters (cut at a word), `None` if empty.
fn clean(text: &str, max: usize) -> Option<String> {
  let text = unescape(text);
  let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
  if text.is_empty() {
    return None;
  }
  if text.chars().count() <= max {
    return Some(text);
  }
  // One character is left for the ellipsis.
  let cut: String = text.chars().take(max - 1).collect();
  let cut = cut.rsplit_once(' ').map_or(cut.as_str(), |(head, _)| head).trim_end();
  Some(format!("{cut}…"))
}

/// Character references left in text after HTML parsing: some sites escape twice, so
/// `content="Clippy&amp;#39;s"` reads as `Clippy&#39;s`. Decodes numeric references and the
/// common named ones, once.
fn unescape(text: &str) -> String {
  if !text.contains('&') {
    return text.to_string();
  }
  let mut out = String::with_capacity(text.len());
  let mut rest = text;
  while let Some(at) = rest.find('&') {
    out.push_str(&rest[..at]);
    rest = &rest[at..];
    let decoded = rest.find(';').filter(|&end| end <= 10).and_then(|end| {
      let name = &rest[1..end];
      let c = match name {
        "amp" => Some('&'),
        "lt" => Some('<'),
        "gt" => Some('>'),
        "quot" => Some('"'),
        "apos" => Some('\''),
        "nbsp" => Some(' '),
        _ => name
          .strip_prefix("#x")
          .or_else(|| name.strip_prefix("#X"))
          .map(|hex| u32::from_str_radix(hex, 16))
          .or_else(|| name.strip_prefix('#').map(str::parse::<u32>))
          .and_then(|n| n.ok())
          .and_then(char::from_u32),
      }?;
      Some((c, end + 1))
    });
    match decoded {
      Some((c, len)) => {
        out.push(c);
        rest = &rest[len..];
      }
      None => {
        out.push('&');
        rest = &rest[1..];
      }
    }
  }
  out.push_str(rest);
  out
}

/// An absolute http(s) link, or `None`.
fn absolute(base: &Url, link: &str) -> Option<String> {
  let url = base.join(link.trim()).ok()?;
  matches!(url.scheme(), "http" | "https").then(|| url.to_string())
}

/// `<meta property|name=… content=…>`, in document order, keys lowercased.
struct Meta(Vec<(String, String)>);

impl Meta {
  fn read(doc: &Html) -> Self {
    let tags = select(doc, "meta[content]")
      .filter_map(|el| {
        let attrs = el.value();
        let key = attrs.attr("property").or_else(|| attrs.attr("name"))?;
        Some((key.trim().to_ascii_lowercase(), attrs.attr("content")?.trim().to_string()))
      })
      .filter(|(_, content)| !content.is_empty())
      .collect();
    Self(tags)
  }

  /// The first value for the first key that has one (keys in priority order).
  fn first(&self, keys: &[&str]) -> Option<String> {
    self.all(keys).into_iter().next()
  }

  /// Every value, grouped by key in priority order.
  fn all(&self, keys: &[&str]) -> Vec<String> {
    keys
      .iter()
      .flat_map(|key| self.0.iter().filter(move |(k, _)| k == key).map(|(_, v)| v.clone()))
      .collect()
  }
}

/// JSON-LD blocks, flattened (`@graph`, arrays), with the node that describes the page itself.
struct JsonLd {
  main: Option<Value>,
}

/// schema.org type → kind. Types not listed (WebSite, Organization, BreadcrumbList…) describe the
/// site, not the page.
const LD_KINDS: &[(&str, &str)] = &[
  ("Article", "article"),
  ("NewsArticle", "article"),
  ("BlogPosting", "article"),
  ("TechArticle", "article"),
  ("ScholarlyArticle", "article"),
  ("Report", "article"),
  ("Recipe", "recipe"),
  ("VideoObject", "video"),
  ("TVEpisode", "video"),
  ("TVSeries", "video"),
  ("Movie", "movie"),
  ("MusicRecording", "music"),
  ("MusicAlbum", "music"),
  ("MusicPlaylist", "music"),
  ("PodcastEpisode", "podcast"),
  ("PodcastSeries", "podcast"),
  ("Product", "product"),
  ("ProductGroup", "product"),
  ("Event", "event"),
  ("Book", "book"),
  ("Course", "course"),
  ("SoftwareApplication", "software"),
  ("MobileApplication", "software"),
  ("WebApplication", "software"),
  ("Place", "place"),
  ("LocalBusiness", "place"),
  ("Restaurant", "place"),
  ("Hotel", "place"),
  ("Person", "person"),
  ("ProfilePage", "person"),
];

impl JsonLd {
  fn read(doc: &Html) -> Self {
    let mut nodes = Vec::new();
    for script in select(doc, r#"script[type="application/ld+json"]"#) {
      let text: String = script.text().collect();
      if let Ok(value) = serde_json::from_str::<Value>(&text) {
        flatten(value, &mut nodes);
      }
    }
    let main = nodes.iter().find(|n| ld_kind(n).is_some()).cloned();
    Self { main }
  }

  fn kind(&self) -> Option<&'static str> {
    self.main.as_ref().and_then(ld_kind)
  }

  fn text(&self, keys: &[&str]) -> Option<String> {
    let main = self.main.as_ref()?;
    keys.iter().find_map(|key| main.get(key)?.as_str().map(str::to_string))
  }

  /// `"author": "Ann"`, `{ "name": "Ann" }` or `[{ "name": "Ann" }, …]` → `Ann`.
  fn name_of(&self, key: &str) -> Option<String> {
    let value = self.main.as_ref()?.get(key)?;
    let value = value.as_array().and_then(|a| a.first()).unwrap_or(value);
    value.as_str().or_else(|| value.get("name")?.as_str()).map(str::to_string)
  }

  /// `"image"` as a link, `{ "url": … }`, or a list of either.
  fn image(&self) -> Option<String> {
    let value = self.main.as_ref()?.get("image")?;
    let value = value.as_array().and_then(|a| a.first()).unwrap_or(value);
    value.as_str().or_else(|| value.get("url")?.as_str()).map(str::to_string)
  }

  /// `"keywords"`: a comma-separated string or a list.
  fn keywords(&self) -> Vec<String> {
    match self.main.as_ref().and_then(|m| m.get("keywords")) {
      Some(Value::String(s)) => s.split(',').map(str::to_string).collect(),
      Some(Value::Array(items)) => items.iter().filter_map(|v| v.as_str().map(str::to_string)).collect(),
      _ => Vec::new(),
    }
  }
}

fn flatten(value: Value, out: &mut Vec<Value>) {
  match value {
    Value::Array(items) => items.into_iter().for_each(|v| flatten(v, out)),
    Value::Object(mut map) => {
      if let Some(graph) = map.remove("@graph") {
        flatten(graph, out);
      }
      if map.contains_key("@type") {
        out.push(Value::Object(map));
      }
    }
    _ => {}
  }
}

/// The node's kind, from `@type` (a string or a list). Any `…Event` counts as an event.
fn ld_kind(node: &Value) -> Option<&'static str> {
  let types: Vec<&str> = match node.get("@type")? {
    Value::String(t) => vec![t.as_str()],
    Value::Array(ts) => ts.iter().filter_map(Value::as_str).collect(),
    _ => return None,
  };
  types.iter().find_map(|t| {
    let t = t.rsplit('/').next().unwrap_or(t);
    LD_KINDS.iter().find(|(name, _)| *name == t).map(|(_, kind)| *kind).or_else(|| t.ends_with("Event").then_some("event"))
  })
}

fn og_kind(og_type: &str) -> Option<&'static str> {
  let t = og_type.to_ascii_lowercase();
  Some(match t.split('.').next()? {
    "article" => "article",
    "video" => "video",
    "music" => "music",
    "book" | "books" => "book",
    "product" | "og:product" => "product",
    "profile" => "person",
    _ => return None,
  })
}

fn keywords(meta: &Meta, ld: &JsonLd) -> Vec<String> {
  let declared = meta
    .all(&["keywords", "news_keywords"])
    .into_iter()
    .flat_map(|list| list.split(',').map(str::to_string).collect::<Vec<_>>())
    .chain(meta.all(&["article:tag", "og:video:tag", "video:tag", "book:tag"]))
    .chain(ld.keywords());
  let mut out: Vec<String> = Vec::new();
  for word in declared {
    let Some(word) = clean(&word, usize::MAX).filter(|w| w.chars().count() <= MAX_KEYWORD) else { continue };
    if !out.iter().any(|w| w.eq_ignore_ascii_case(&word)) {
      out.push(word);
    }
    if out.len() == MAX_KEYWORDS {
      break;
    }
  }
  out
}

/// The largest declared icon (`apple-touch-icon` first, as it's meant to be shown large), or
/// `/favicon.ico` on the site.
fn icon(doc: &Html, base: &Url) -> Option<String> {
  let size = |el: &ElementRef| -> u32 {
    el.value()
      .attr("sizes")
      .and_then(|s| s.split_whitespace().filter_map(|s| s.split_once(['x', 'X'])?.0.parse().ok()).max())
      .unwrap_or(0)
  };
  let best = |css: &str| select(doc, css).max_by_key(size).and_then(|el| absolute(base, el.value().attr("href")?));
  best(r#"link[rel~="apple-touch-icon"][href]"#)
    .or_else(|| best(r#"link[rel~="icon"][href]"#))
    .or_else(|| absolute(base, "/favicon.ico"))
}

/// The start of the main text: from `<article>`, `<main>` or the body, leaving out navigation,
/// headers, footers, scripts and forms. `None` when most of the text is link text: that's a menu
/// or a list of links (e.g. a page whose content is drawn by JavaScript), not something to read.
fn excerpt(doc: &Html) -> Option<String> {
  let root = ["article", "main", r#"[role="main"]"#, "body"].iter().find_map(|css| select(doc, css).next())?;
  let mut text = String::new();
  let (mut chars, mut link_chars) = (0, 0);
  for node in root.descendants() {
    let Node::Text(part) = node.value() else { continue };
    let inside: Vec<&str> = node
      .ancestors()
      .take_while(|a| a.id() != root.id())
      .filter_map(|a| a.value().as_element().map(|e| e.name()))
      .collect();
    if inside.iter().any(|name| NOT_TEXT.contains(name)) {
      continue;
    }
    let len: usize = part.split_whitespace().map(str::len).sum();
    chars += len;
    if inside.contains(&"a") {
      link_chars += len;
    }
    text.push_str(part);
    text.push(' ');
    if text.len() > MAX_EXCERPT * 8 {
      break;
    }
  }
  if link_chars * 2 > chars {
    return None;
  }
  clean(&text, MAX_EXCERPT)
}
