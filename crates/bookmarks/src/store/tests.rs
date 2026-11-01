use std::sync::{
  Arc,
  atomic::{AtomicI64, Ordering},
};

use super::*;
use crate::{ListQuery, PreviewStatus, SCHEMA_VERSION, TagCount, TagInput, TagSource};

/// Each call is one millisecond later, so ordering by time is predictable.
struct TestClock(AtomicI64);

impl Clock for TestClock {
  fn now_ms(&self) -> i64 {
    self.0.fetch_add(1, Ordering::SeqCst)
  }
}

async fn store() -> Store {
  let conn = db::open(&Location::Memory).await.unwrap();
  Store::new(conn, Arc::new(BundledRules), Arc::new(TestClock(AtomicI64::new(1_000))))
    .await
    .unwrap()
}

fn new(text: &str, tags: &[&str]) -> NewBookmark {
  NewBookmark {
    shared_text: text.into(),
    tags: tags.iter().map(|t| TagInput::user(*t)).collect(),
    ..Default::default()
  }
}

fn tag_names(b: &Bookmark) -> Vec<&str> {
  b.tags.iter().map(|t| t.name.as_str()).collect()
}

async fn search(store: &Store, text: &str) -> Vec<String> {
  let query = ListQuery { search: Some(text.into()), ..Default::default() };
  store.list(&query).await.unwrap().items.into_iter().map(|b| b.url).collect()
}

#[tokio::test]
async fn saves_the_cleaned_link_and_keeps_the_original() {
  let mut store = store().await;
  let saved = store.save(new("Great talk https://youtu.be/abc?si=TRACK", &["rust"])).await.unwrap();

  assert!(saved.created);
  let b = saved.bookmark;
  assert_eq!(b.url, "https://youtu.be/abc");
  assert_eq!(b.original_url, "https://youtu.be/abc?si=TRACK");
  assert_eq!(b.title.as_deref(), Some("Great talk"));
  assert_eq!(b.provider.as_deref(), Some("youtube"));
  assert_eq!(b.preview_status, PreviewStatus::None);
  assert_eq!(b.rules_revision, link_cleaner::bundled_rules().revision);
  assert_eq!(tag_names(&b), ["rust"]);
  assert_eq!(store.get(&b.id).await.unwrap(), Some(b));
}

#[tokio::test]
async fn shared_text_without_a_link_is_refused() {
  let mut store = store().await;
  assert!(matches!(store.save(new("just words", &[])).await, Err(Error::NoLink)));
}

#[tokio::test]
async fn saving_the_same_link_again_merges() {
  let mut store = store().await;
  let first = store.save(new("https://x.com/a/status/1?s=20", &["news"])).await.unwrap().bookmark;

  // Same post, different tracking, a title this time, and more tags.
  let mut again = new("https://x.com/a/status/1?t=XYZ", &["News", "politics"]);
  again.title = Some("A thread".into());
  let second = store.save(again).await.unwrap();

  assert!(!second.created);
  assert_eq!(second.bookmark.id, first.id);
  assert_eq!(second.bookmark.created_at, first.created_at);
  assert_eq!(second.bookmark.title.as_deref(), Some("A thread"));
  assert_eq!(tag_names(&second.bookmark), ["news", "politics"]);
  assert_eq!(store.list(&ListQuery::default()).await.unwrap().items.len(), 1);
}

#[tokio::test]
async fn merging_keeps_an_existing_title() {
  let mut store = store().await;
  store.save(new("Mine https://example.com/a", &[])).await.unwrap();
  let saved = store.save(new("Theirs https://example.com/a", &[])).await.unwrap();
  assert_eq!(saved.bookmark.title.as_deref(), Some("Mine"));
}

#[tokio::test]
async fn ids_are_the_same_on_every_device() {
  let (mut phone_a, mut phone_b) = (store().await, store().await);
  let a = phone_a.save(new("https://youtu.be/abc?si=1", &["Rust"])).await.unwrap().bookmark;
  let b = phone_b.save(new("https://youtu.be/abc?si=2", &["rust"])).await.unwrap().bookmark;
  assert_eq!(a.id, b.id);
  assert_eq!(ids::tag_id("Rust"), ids::tag_id("rust"));
}

#[tokio::test]
async fn prepare_reports_links_and_an_existing_bookmark() {
  let mut store = store().await;
  let fresh = store.prepare("see https://youtu.be/a?si=1 and https://example.com/b").await.unwrap();
  assert_eq!(fresh.links.len(), 2);
  assert_eq!(fresh.links[0].cleaned, "https://youtu.be/a");
  assert!(fresh.existing.is_none());

  store.save(new("https://youtu.be/a", &[])).await.unwrap();
  let again = store.prepare("https://youtu.be/a?si=2").await.unwrap();
  assert_eq!(again.existing.unwrap().url, "https://youtu.be/a");
}

#[tokio::test]
async fn update_changes_and_clears_fields() {
  let mut store = store().await;
  let b = store.save(new("Title https://example.com/a", &[])).await.unwrap().bookmark;

  let patch = BookmarkPatch { note: Some("  remember this ".into()), ..Default::default() };
  let updated = store.update(&b.id, patch).await.unwrap();
  assert_eq!(updated.note.as_deref(), Some("remember this"));
  assert_eq!(updated.title.as_deref(), Some("Title"));
  assert!(updated.updated_at > b.updated_at);

  let cleared = store
    .update(&b.id, BookmarkPatch { title: Some(String::new()), ..Default::default() })
    .await
    .unwrap();
  assert_eq!(cleared.title, None);
  assert_eq!(cleared.note.as_deref(), Some("remember this"));

  assert!(matches!(store.update("nope", BookmarkPatch::default()).await, Err(Error::NotFound)));
}

#[tokio::test]
async fn delete_removes_the_bookmark_but_keeps_its_tags() {
  let mut store = store().await;
  let a = store.save(new("https://example.com/a", &["keep", "only-a"])).await.unwrap().bookmark;
  store.save(new("https://example.com/b", &["keep"])).await.unwrap();

  store.delete(&a.id).await.unwrap();
  assert_eq!(store.get(&a.id).await.unwrap(), None);
  // "only-a" has no bookmarks left but stays, with a count of 0, until the user deletes it.
  assert_eq!(
    store.tags().await.unwrap(),
    [TagCount { name: "keep".into(), count: 1 }, TagCount { name: "only-a".into(), count: 0 }]
  );
  assert!(matches!(store.delete(&a.id).await, Err(Error::NotFound)));
}

#[tokio::test]
async fn tags_are_normalized_and_case_insensitive() {
  let mut store = store().await;
  let b = store
    .save(new("https://example.com/a", &["  #Rust  lang ", "rust LANG", "", "  "]))
    .await
    .unwrap();
  assert_eq!(tag_names(&b.bookmark), ["Rust lang"]);
}

#[tokio::test]
async fn set_tags_replaces_and_keeps_the_source() {
  let mut store = store().await;
  let b = store.save(new("https://example.com/a", &["old"])).await.unwrap().bookmark;

  let tags = store
    .set_tags(
      &b.id,
      &[TagInput::user("new"), TagInput { name: "ai-pick".into(), source: TagSource::Ai }],
    )
    .await
    .unwrap();
  assert_eq!(
    tags.iter().map(|t| (t.name.as_str(), t.source)).collect::<Vec<_>>(),
    [("ai-pick", TagSource::Ai), ("new", TagSource::User)]
  );
  // "old" is no longer used, but stays (count 0) so it can still be picked or deleted.
  let counts: Vec<_> = store.tags().await.unwrap().into_iter().map(|t| (t.name, t.count)).collect();
  assert_eq!(counts, [("ai-pick".into(), 1), ("new".into(), 1), ("old".to_string(), 0)]);
}

#[tokio::test]
async fn tag_counts_are_sorted_by_use() {
  let mut store = store().await;
  store.save(new("https://example.com/a", &["video", "rust"])).await.unwrap();
  store.save(new("https://example.com/b", &["video"])).await.unwrap();
  assert_eq!(
    store.tags().await.unwrap(),
    [TagCount { name: "video".into(), count: 2 }, TagCount { name: "rust".into(), count: 1 },]
  );
}

#[tokio::test]
async fn rename_tag_merges_into_an_existing_one() {
  let mut store = store().await;
  let a = store.save(new("https://example.com/a", &["film"])).await.unwrap().bookmark;
  let b = store.save(new("https://example.com/b", &["film", "movies"])).await.unwrap().bookmark;
  store.save(new("https://example.com/c", &["movies"])).await.unwrap();

  store.rename_tag("film", "movies").await.unwrap();
  assert_eq!(store.tags().await.unwrap(), [TagCount { name: "movies".into(), count: 3 }]);
  assert_eq!(tag_names(&store.get(&a.id).await.unwrap().unwrap()), ["movies"]);
  assert_eq!(tag_names(&store.get(&b.id).await.unwrap().unwrap()), ["movies"]);
  // The search text follows the rename.
  assert_eq!(search(&store, "film").await, Vec::<String>::new());
  assert_eq!(search(&store, "movies").await.len(), 3);

  store.rename_tag("MOVIES", "Movies").await.unwrap();
  assert_eq!(store.tags().await.unwrap()[0].name, "Movies");
  assert!(matches!(store.rename_tag("nothing", "x").await, Err(Error::TagNotFound(_))));
}

#[tokio::test]
/// Deleting a tag that's in use removes it from every bookmark that has it.
async fn delete_tag_removes_it_everywhere() {
  let mut store = store().await;
  let a = store.save(new("https://example.com/a", &["temp", "keep"])).await.unwrap().bookmark;
  let b = store.save(new("https://example.com/b", &["temp"])).await.unwrap().bookmark;
  store.delete_tag("TEMP").await.unwrap();
  assert_eq!(tag_names(&store.get(&a.id).await.unwrap().unwrap()), ["keep"]);
  assert!(tag_names(&store.get(&b.id).await.unwrap().unwrap()).is_empty());
  assert_eq!(store.tags().await.unwrap(), [TagCount { name: "keep".into(), count: 1 }]);
  assert!(search(&store, "temp").await.is_empty());
}

#[tokio::test]
async fn lists_newest_first_in_pages() {
  let mut store = store().await;
  for i in 0..5 {
    store.save(new(&format!("https://example.com/{i}"), &[])).await.unwrap();
  }
  let mut query = ListQuery { limit: 2, ..Default::default() };
  let mut urls = Vec::new();
  loop {
    let page = store.list(&query).await.unwrap();
    urls.extend(page.items.into_iter().map(|b| b.url));
    match page.next {
      Some(next) => query.cursor = Some(next),
      None => break,
    }
  }
  let expected: Vec<_> = (0..5).rev().map(|i| format!("https://example.com/{i}")).collect();
  assert_eq!(urls, expected);
}

#[tokio::test]
async fn filters_by_all_given_tags() {
  let mut store = store().await;
  store.save(new("https://example.com/a", &["rust", "video"])).await.unwrap();
  store.save(new("https://example.com/b", &["rust"])).await.unwrap();
  store.save(new("https://example.com/c", &["video"])).await.unwrap();

  let query = ListQuery { tags: vec!["Rust".into(), "video".into()], ..Default::default() };
  let urls: Vec<_> = store.list(&query).await.unwrap().items.into_iter().map(|b| b.url).collect();
  assert_eq!(urls, ["https://example.com/a"]);
}

#[tokio::test]
async fn search_matches_parts_of_words_in_every_field() {
  let mut store = store().await;
  let mut talk = new("Rust async talk https://youtu.be/a", &["programming"]);
  talk.note = Some("watch with the team".into());
  store.save(talk).await.unwrap();
  store.save(new("Cooking pasta at home https://example.com/pasta", &["food"])).await.unwrap();

  assert_eq!(search(&store, "ru").await, ["https://youtu.be/a"]); // title, as you type
  assert_eq!(search(&store, "team").await, ["https://youtu.be/a"]); // note
  assert_eq!(search(&store, "progr").await, ["https://youtu.be/a"]); // tag
  assert_eq!(search(&store, "PASTA").await, ["https://example.com/pasta"]);
  // Every word must match.
  assert_eq!(search(&store, "rust talk").await, ["https://youtu.be/a"]);
  assert!(search(&store, "rust pasta").await.is_empty());
  // Query syntax in the input is just text, and one-letter input doesn't filter.
  assert!(search(&store, "title:rust NOT* (").await.is_empty());
  assert_eq!(search(&store, "rust*").await, ["https://youtu.be/a"]);
  assert_eq!(search(&store, "a").await.len(), 2);
}

/// Guards the score direction: the docs once said "lower is better" while 0.8.3 returns higher
/// for better matches. If an upgrade flips it, this fails.
#[tokio::test]
async fn search_puts_the_best_match_first() {
  let mut store = store().await;
  // Saved first, so ordering by time alone would put it last.
  let mut strong = new("Rust tips https://example.com/strong", &["rust"]);
  strong.note = Some("rust rust rust".into());
  store.save(strong).await.unwrap();
  store
    .save(new(
      "A long post about cooking, gardening, travel, music and, once, rust https://example.com/weak",
      &[],
    ))
    .await
    .unwrap();

  assert_eq!(
    search(&store, "rust").await,
    ["https://example.com/strong", "https://example.com/weak"]
  );
}

#[tokio::test]
async fn optimize_keeps_search_working() {
  let mut store = store().await;
  store.optimize().await.unwrap(); // empty index

  let mut ids = Vec::new();
  for i in 0..20 {
    ids.push(
      store
        .save(new(&format!("Recipe {i} https://example.com/{i}"), &[]))
        .await
        .unwrap()
        .bookmark
        .id,
    );
  }
  for id in &ids[..15] {
    store.delete(id).await.unwrap();
  }
  store
    .update(&ids[15], BookmarkPatch { title: Some("Renamed".into()), ..Default::default() })
    .await
    .unwrap();
  store.optimize().await.unwrap();

  // 5 left; the renamed one still says "Recipe 15" in its shared text.
  assert_eq!(search(&store, "recipe").await.len(), 5);
  assert_eq!(search(&store, "renamed").await, ["https://example.com/15"]);
}

#[tokio::test]
async fn search_sees_edits() {
  let mut store = store().await;
  let b = store.save(new("https://example.com/a", &[])).await.unwrap().bookmark;
  assert!(search(&store, "weekend").await.is_empty());
  store
    .update(&b.id, BookmarkPatch { note: Some("for the weekend".into()), ..Default::default() })
    .await
    .unwrap();
  assert_eq!(search(&store, "weekend").await, ["https://example.com/a"]);
}

#[tokio::test]
async fn search_combines_with_tags_and_pages() {
  let mut store = store().await;
  for i in 0..3 {
    store.save(new(&format!("Recipe {i} https://example.com/{i}"), &["food"])).await.unwrap();
  }
  store.save(new("Recipe app https://example.com/app", &["tech"])).await.unwrap();

  let mut query = ListQuery {
    search: Some("recipe".into()),
    tags: vec!["food".into()],
    limit: 2,
    ..Default::default()
  };
  let first = store.list(&query).await.unwrap();
  assert_eq!(first.items.len(), 2);
  query.cursor = first.next;
  let second = store.list(&query).await.unwrap();
  assert_eq!(second.items.len(), 1);
  assert_eq!(second.next, None);
}

#[tokio::test]
async fn bad_cursors_are_rejected() {
  let store = store().await;
  for cursor in ["garbage", "t:notanumber:x", "o:x"] {
    let query = ListQuery { cursor: Some(cursor.into()), ..Default::default() };
    assert!(matches!(store.list(&query).await, Err(Error::InvalidCursor)), "{cursor}");
  }
  // A search cursor can't continue a plain list.
  let query = ListQuery { cursor: Some("o:2".into()), ..Default::default() };
  assert!(matches!(store.list(&query).await, Err(Error::InvalidCursor)));
}

#[tokio::test]
async fn file_database_keeps_data_and_schema_across_opens() {
  let dir = std::env::temp_dir().join(format!("bookmarks-test-{}", std::process::id()));
  std::fs::create_dir_all(&dir).unwrap();
  let path = dir.join("bookmarks.db");
  let _ = std::fs::remove_file(&path);
  let location = Location::File(path.clone());

  let id = {
    let mut store = Store::open(&location).await.unwrap();
    store.save(new("Kept https://example.com/kept", &["saved"])).await.unwrap().bookmark.id
  };
  let store = Store::open(&location).await.unwrap();
  assert_eq!(migrate::user_version(&store.conn).await.unwrap(), SCHEMA_VERSION);
  assert_eq!(tag_names(&store.get(&id).await.unwrap().unwrap()), ["saved"]);
  assert_eq!(search(&store, "kept").await, ["https://example.com/kept"]);
  drop(store);
  let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn refuses_a_database_from_a_newer_app() {
  let conn = db::open(&Location::Memory).await.unwrap();
  conn.execute_batch(&format!("PRAGMA user_version = {}", SCHEMA_VERSION + 1)).await.unwrap();
  let result = Store::new(conn, Arc::new(BundledRules), Arc::new(SystemClock)).await;
  assert!(matches!(result, Err(Error::NewerSchema { .. })));
}

#[test]
fn titles_come_from_the_text_around_the_link() {
  assert_eq!(
    title_from("Great talk - https://a.com/x", "https://a.com/x").as_deref(),
    Some("Great talk")
  );
  assert_eq!(title_from("https://a.com/x", "https://a.com/x"), None);
  assert_eq!(
    title_from("Look:\nhttps://a.com/x\n  so good", "https://a.com/x").as_deref(),
    Some("Look: so good")
  );
}
