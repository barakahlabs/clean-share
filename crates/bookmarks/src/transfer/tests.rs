use super::*;
use crate::TagSource;

fn names(item: &ImportItem) -> Vec<&str> {
  item.tags.iter().map(|t| t.name.as_str()).collect()
}

const CHROME: &str = r#"<!DOCTYPE NETSCAPE-Bookmark-file-1>
<!-- This is an automatically generated file. -->
<META HTTP-EQUIV="Content-Type" CONTENT="text/html; charset=UTF-8">
<TITLE>Bookmarks</TITLE>
<H1>Bookmarks</H1>
<DL><p>
    <DT><H3 ADD_DATE="1700000000" PERSONAL_TOOLBAR_FOLDER="true">Bookmarks bar</H3>
    <DL><p>
        <DT><A HREF="https://www.rust-lang.org/" ADD_DATE="1700000000" ICON="data:image/png;base64,AAA">Rust &amp; Cargo</A>
        <DT><H3 ADD_DATE="1700000100">Recipes</H3>
        <DL><p>
            <DT><A HREF="https://kitchen.example/lasagna?utm_source=newsletter" ADD_DATE="1700000200">Lasagna</A>
        </DL><p>
    </DL><p>
    <DT><A HREF="javascript:alert(1)">A bookmarklet</A>
    <DT><A HREF="https://example.com/top">Top level</A>
</DL><p>
"#;

#[test]
fn chrome_bookmarks_with_folders_as_tags() {
  let (format, items) = parse(CHROME).unwrap();
  assert_eq!(format, Format::Html);
  // The bookmarklet isn't a web link.
  assert_eq!(items.len(), 3);
  assert_eq!(items[0].url, "https://www.rust-lang.org/");
  assert_eq!(items[0].title.as_deref(), Some("Rust & Cargo"));
  assert_eq!(items[0].created_at, Some(1_700_000_000_000));
  // "Bookmarks bar" is the browser's, not a tag.
  assert!(items[0].tags.is_empty());
  assert_eq!(names(&items[1]), ["Recipes"]);
  assert_eq!(items[1].url, "https://kitchen.example/lasagna?utm_source=newsletter");
  assert!(items[2].tags.is_empty());
}

#[test]
fn firefox_tags_and_descriptions() {
  let html = r#"<!DOCTYPE NETSCAPE-Bookmark-file-1>
<DL><p>
  <DT><H3>Reading</H3>
  <DL><p>
    <DT><A HREF="https://blog.example/post" ADD_DATE="1700000000123456" LAST_MODIFIED="1" TAGS="rust,async">A post</A>
    <DD>Worth a second read
  </DL><p>
</DL>"#;
  let (_, items) = parse(html).unwrap();
  assert_eq!(items.len(), 1);
  assert_eq!(names(&items[0]), ["rust", "async", "Reading"]);
  assert_eq!(items[0].note.as_deref(), Some("Worth a second read"));
  assert_eq!(items[0].created_at, Some(1_700_000_000_123));
}

#[test]
fn pocket_html_export() {
  let html = r#"<!DOCTYPE html><html><head><title>Pocket Export</title></head><body>
<h1>Unread</h1>
<ul>
<li><a href="https://news.example/a" time_added="1700000000" tags="news,tech">Article A</a></li>
</ul>
<h1>Read Archive</h1>
<ul>
<li><a href="https://news.example/b" time_added="1700000500" tags="">Article B</a></li>
</ul></body></html>"#;
  let (format, items) = parse(html).unwrap();
  assert_eq!(format, Format::Html);
  assert_eq!(items.len(), 2);
  assert_eq!(names(&items[0]), ["news", "tech"]);
  assert_eq!(items[1].created_at, Some(1_700_000_500_000));
}

#[test]
fn pocket_csv_export() {
  let csv = "title,url,time_added,tags,status\n\
             Article A,https://news.example/a,1700000000,news|tech,unread\n\
             https://news.example/b,https://news.example/b,1700000500,,archive\n";
  let (format, items) = parse(csv).unwrap();
  assert_eq!(format, Format::Pocket);
  assert_eq!(items.len(), 2);
  assert_eq!(items[0].title.as_deref(), Some("Article A"));
  assert_eq!(names(&items[0]), ["news", "tech"]);
  // Pocket writes the link as the title when there's none.
  assert_eq!(items[1].title, None);
}

#[test]
fn raindrop_csv_export() {
  let csv = "id,title,note,excerpt,url,folder,tags,created,cover,highlights,favorite\n\
             1,Lasagna,My favourite,A quick dinner,https://kitchen.example/lasagna,Food/Italian,\"pasta, dinner\",2023-11-14T22:13:20.000Z,,,false\n\
             2,Inbox thing,,,https://example.com/x,Unsorted,,2023-11-14T22:13:20Z,,,false\n";
  let (format, items) = parse(csv).unwrap();
  assert_eq!(format, Format::Raindrop);
  assert_eq!(names(&items[0]), ["pasta", "dinner", "Italian"]);
  assert_eq!(items[0].note.as_deref(), Some("My favourite"));
  assert_eq!(items[0].created_at, Some(1_700_000_000_000));
  // Raindrop's default collection isn't a tag.
  assert!(items[1].tags.is_empty());
}

#[test]
fn any_csv_with_a_url_column() {
  let csv = "Name,Link,Labels\nRust,https://www.rust-lang.org/,lang\nnot a link,ftp://x,\n";
  let (format, items) = parse(csv).unwrap();
  assert_eq!(format, Format::Csv);
  assert_eq!(items.len(), 1);
  assert_eq!(items[0].title.as_deref(), Some("Rust"));
  assert_eq!(names(&items[0]), ["lang"]);
}

#[test]
fn clean_share_backup_round_trip_keeps_tag_sources() {
  let json = r#"{"format":"clean-share/bookmarks","version":1,"exportedAt":1,
    "bookmarks":[{"url":"https://youtu.be/abc","originalUrl":"https://youtu.be/abc?si=TRACK",
      "title":"Talk","note":"Watch later","tags":[{"name":"video","source":"suggested"}],
      "createdAt":1700000000000,"updatedAt":1700000000001}]}"#;
  let (format, items) = parse(json).unwrap();
  assert_eq!(format, Format::CleanShare);
  // The original link, so today's rules clean it.
  assert_eq!(items[0].url, "https://youtu.be/abc?si=TRACK");
  assert_eq!(items[0].tags[0].source, TagSource::Suggested);
  assert_eq!(items[0].created_at, Some(1_700_000_000_000));
}

#[test]
fn a_backup_from_a_newer_app_is_refused() {
  let json = r#"{"format":"clean-share/bookmarks","version":99,"exportedAt":1,"bookmarks":[]}"#;
  assert!(matches!(parse(json), Err(ParseError::Invalid(_, reason)) if reason.contains("newer")));
}

#[test]
fn other_files_are_refused() {
  assert_eq!(parse("just some notes\nnothing here"), Err(ParseError::UnknownFormat));
  // JSON, but not ours.
  assert!(parse(r#"{"some":"json"}"#).unwrap_err().to_string().contains("can't be read"));
}

#[test]
fn html_export_reads_back() {
  use crate::{Bookmark, BookmarkTag, PreviewStatus};
  let b = Bookmark {
    id: "1".into(),
    url: "https://example.com/a?b=1&c=2".into(),
    original_url: String::new(),
    title: Some("Fish & \"chips\"".into()),
    note: Some("Crispy <3".into()),
    shared_text: String::new(),
    provider: None,
    site_name: None,
    description: None,
    image_url: None,
    embed_kind: None,
    embed_url: None,
    page_kind: None,
    keywords: vec![],
    author: None,
    published: None,
    icon_url: None,
    excerpt: None,
    preview_status: PreviewStatus::None,
    preview_fetched_at: None,
    rules_revision: String::new(),
    tags: vec![BookmarkTag { name: "food".into(), source: TagSource::User }],
    created_at: 1_700_000_000_000,
    updated_at: 1_700_000_000_000,
  };
  let html = netscape::export(&[b]);
  let (_, items) = parse(&html).unwrap();
  assert_eq!(items.len(), 1);
  assert_eq!(items[0].url, "https://example.com/a?b=1&c=2");
  assert_eq!(items[0].title.as_deref(), Some("Fish & \"chips\""));
  assert_eq!(items[0].note.as_deref(), Some("Crispy <3"));
  // The "Clean Share" folder isn't a tag; the bookmark's own tags are kept.
  assert_eq!(names(&items[0]), ["food"]);
  assert_eq!(items[0].created_at, Some(1_700_000_000_000));
}
