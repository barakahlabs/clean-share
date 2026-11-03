//! Parsing saved pages: what each source contributes, and which wins.

use link_preview::{Page, merge, oembed, parse_html};
use url::Url;

fn page(fixture: &str, url: &str) -> Page {
  let path = format!("{}/tests/fixtures/{fixture}", env!("CARGO_MANIFEST_DIR"));
  parse_html(&std::fs::read_to_string(path).unwrap(), &Url::parse(url).unwrap())
}

#[test]
fn youtube_watch_page() {
  let p = page("youtube-watch.html", "https://www.youtube.com/watch?v=5C_HPTJg5ek");
  assert_eq!(p.title.as_deref(), Some("Rust in 100 Seconds"));
  assert_eq!(p.site_name.as_deref(), Some("YouTube"));
  assert_eq!(p.image_url.as_deref(), Some("https://i.ytimg.com/vi/5C_HPTJg5ek/maxresdefault.jpg"));
  // The largest declared icon.
  assert_eq!(p.icon_url.as_deref(), Some("https://www.youtube.com/s/desktop/favicon_144x144.png"));
  // JSON-LD says VideoObject; og:type would say video too.
  assert_eq!(p.kind.as_deref(), Some("video"));
  assert_eq!(p.author.as_deref(), Some("Fireship"));
  assert_eq!(p.published.as_deref(), Some("2021-09-20T15:00:12-07:00"));
  assert_eq!(p.language.as_deref(), Some("en"));
  // Keywords from <meta name=keywords> and og:video:tag, deduplicated ignoring case.
  assert_eq!(p.keywords, ["rust", "programming", "systems programming", "fireship", "webassembly"]);
  assert_eq!(
    p.oembed_url.as_deref(),
    Some("https://www.youtube.com/oembed?format=json&url=https%3A%2F%2Fwww.youtube.com%2Fwatch%3Fv%3D5C_HPTJg5ek")
  );
  // Script text isn't part of the excerpt.
  assert_eq!(p.excerpt.as_deref(), Some("Rust in 100 Seconds"));
}

#[test]
fn recipe_with_a_json_ld_graph() {
  let p = page("recipe.html", "https://kitchen.example/recipes/lasagna");
  // The Recipe node, not WebSite or Organization.
  assert_eq!(p.kind.as_deref(), Some("recipe"));
  assert_eq!(p.title.as_deref(), Some("Weeknight Lasagna"));
  assert_eq!(p.description.as_deref(), Some("A quick lasagna with fresh pasta."));
  assert_eq!(p.author.as_deref(), Some("Ann Cook"));
  assert_eq!(p.published.as_deref(), Some("2026-03-01"));
  assert_eq!(p.keywords, ["pasta", "Italian", "weeknight dinner"]);
  assert_eq!(p.language.as_deref(), Some("de-DE"));
  // og:image beats JSON-LD's image, made absolute (tracking is the caller's to clean).
  assert_eq!(p.image_url.as_deref(), Some("https://kitchen.example/images/lasagna-1200.jpg?utm_source=og"));
  assert_eq!(p.icon_url.as_deref(), Some("https://kitchen.example/icons/touch-180.png"));
  // From <article>, without the form, nav or footer.
  assert_eq!(
    p.excerpt.as_deref(),
    Some("Weeknight Lasagna This lasagna takes 40 minutes from start to finish. Layer the sauce, pasta & cheese, then bake.")
  );
}

#[test]
fn news_article_with_tags_and_a_base_href() {
  let p = page("news.html", "https://news.example/2026/10/lanes");
  assert_eq!(p.title.as_deref(), Some("City opens new bike lanes"));
  assert_eq!(p.description.as_deref(), Some("Forty kilometres of protected lanes open this week."));
  assert_eq!(p.kind.as_deref(), Some("article"));
  // article:author is a link, so the plain author name is used.
  assert_eq!(p.author.as_deref(), Some("Bo Writer"));
  assert_eq!(p.published.as_deref(), Some("2026-10-30T08:00:00Z"));
  assert_eq!(p.keywords, ["cycling", "transport", "Urban planning"]);
  // Relative to <base href="/en/">.
  assert_eq!(p.image_url.as_deref(), Some("https://news.example/en/img/lanes.jpg"));
  assert_eq!(p.icon_url.as_deref(), Some("https://news.example/en/favicon.svg"));
  assert_eq!(p.excerpt.as_deref(), Some("The city opened forty kilometres of protected bike lanes."));
}

#[test]
fn a_bare_page_falls_back_to_title_and_favicon() {
  let p = page("bare.html", "http://bare.example/a/b");
  let title = p.title.unwrap();
  assert!(title.starts_with("A page with only a title"));
  assert!(title.ends_with('…'));
  assert!(title.chars().count() <= 300);
  assert_eq!(p.icon_url.as_deref(), Some("http://bare.example/favicon.ico"));
  assert_eq!(p.excerpt.as_deref(), Some("Just some text."));
  assert_eq!((p.description, p.image_url, p.kind, p.oembed_url), (None, None, None, None));
  assert!(p.keywords.is_empty());
}

#[test]
fn oembed_wins_for_title_author_and_thumbnail_page_for_the_rest() {
  let p = page("youtube-watch.html", "https://www.youtube.com/watch?v=5C_HPTJg5ek");
  let o = oembed::parse(include_bytes!("fixtures/youtube-oembed.json")).unwrap();
  let preview = merge("https://www.youtube.com/watch?v=5C_HPTJg5ek".into(), Some(p), Some(o));
  assert_eq!(preview.title.as_deref(), Some("Rust in 100 Seconds"));
  assert_eq!(preview.image_url.as_deref(), Some("https://i.ytimg.com/vi/5C_HPTJg5ek/hqdefault.jpg"));
  assert_eq!(preview.site_name.as_deref(), Some("YouTube"));
  assert!(preview.description.unwrap().starts_with("Rust is a memory-safe"));
  assert_eq!(preview.embed.unwrap().url.as_deref(), Some("https://www.youtube.com/embed/5C_HPTJg5ek?feature=oembed"));
}

#[test]
fn oembed_alone_still_makes_a_preview() {
  let o = oembed::parse(include_bytes!("fixtures/x-oembed.json")).unwrap();
  let preview = merge("https://x.com/rustlang/status/1".into(), None, Some(o));
  assert_eq!(preview.author.as_deref(), Some("Rust Language"));
  assert_eq!(preview.site_name.as_deref(), Some("Twitter"));
  assert_eq!(preview.title, None);
}

#[test]
fn no_excerpt_when_the_text_is_mostly_links() {
  let html = r#"<body><div id="app"></div><footer2><a href="/a">About</a> <a href="/b">Press</a> <a href="/c">Copyright</a> <a href="/d">Contact us</a> Terms</footer2></body>"#;
  let p = parse_html(html, &Url::parse("https://app.example/").unwrap());
  assert_eq!(p.excerpt, None);
}
