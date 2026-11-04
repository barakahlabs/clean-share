use std::sync::{
  Arc,
  atomic::{AtomicI64, Ordering},
};

use super::*;
use crate::{ListQuery, PreviewStatus, RecleanReport, SCHEMA_VERSION, TagCount, TagInput, TagSource};

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
  // The suggested title is the text around the first link, as `save` would use it.
  assert_eq!(fresh.title.as_deref(), Some("see and"));
  assert_eq!(store.prepare("https://youtu.be/a").await.unwrap().title, None);

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
fn titles_come_from_the_text_around_the_links() {
  let title = |text: &str| title_from(text, &BundledRules.links(text));
  assert_eq!(title("Great talk - https://a.com/x").as_deref(), Some("Great talk"));
  assert_eq!(title("https://a.com/x"), None);
  assert_eq!(title("Look:\nhttps://a.com/x\n  so good").as_deref(), Some("Look: so good"));
  // Every link is left out, not just the first.
  assert_eq!(title("see https://a.com/x and https://b.com/y").as_deref(), Some("see and"));
}

fn draft(text: &str, picked: &[&str]) -> crate::TagDraft {
  crate::TagDraft {
    text: text.into(),
    title: None,
    picked: picked.iter().map(|p| p.to_string()).collect(),
    page: None,
  }
}

async fn suggested(store: &Store, draft: &crate::TagDraft) -> Vec<(String, crate::Reason)> {
  store.suggest_tags(draft).await.unwrap().into_iter().map(|s| (s.name, s.reason)).collect()
}

#[tokio::test]
async fn suggests_tags_used_on_the_same_site() {
  use crate::Reason::*;
  let mut store = store().await;
  store.save(new("https://blog.example.com/one", &["reading", "rust"])).await.unwrap();
  store.save(new("https://blog.example.com/two", &["reading"])).await.unwrap();
  store.save(new("https://other.example.com/x", &["cooking"])).await.unwrap();

  let got = suggested(&store, &draft("Look https://blog.example.com/three?utm_source=x", &[])).await;
  assert_eq!(got, [("reading".into(), Site), ("rust".into(), Site)]);
}

#[tokio::test]
async fn provider_counts_as_one_site_across_hosts() {
  use crate::Reason::*;
  let mut store = store().await;
  store.save(new("https://www.youtube.com/watch?v=a1", &["talks"])).await.unwrap();

  let got = suggested(&store, &draft("https://youtu.be/b2?si=TRACK", &[])).await;
  assert_eq!(got, [("talks".into(), Site), ("video".into(), Provider)]);
}

#[tokio::test]
async fn suggests_existing_tags_named_in_the_text_and_title() {
  use crate::Reason::*;
  let mut store = store().await;
  store.save(new("https://a.example/1", &["Rust", "recipes"])).await.unwrap();

  let mut d = draft("Async Rust in practice https://b.example/post", &[]);
  d.title = Some("Weeknight recipe ideas".into());
  // Same score and uses: by name. Then a keyword from the title, after the existing tags ("recipe
  // ideas" would overlap `recipes`, so it isn't offered).
  assert_eq!(
    suggested(&store, &d).await,
    [("recipes".into(), Content), ("Rust".into(), Content), ("weeknight".into(), Text)]
  );
}

#[tokio::test]
async fn suggests_tags_used_together_with_the_picked_ones() {
  use crate::Reason::*;
  let mut store = store().await;
  store.save(new("https://a.example/1", &["rust", "async"])).await.unwrap();
  store.save(new("https://b.example/2", &["rust"])).await.unwrap();
  store.save(new("https://c.example/3", &["cooking"])).await.unwrap();

  let got = suggested(&store, &draft("https://d.example/4", &["Rust"])).await;
  assert_eq!(got, [("async".into(), Related)]);
}

#[tokio::test]
async fn editing_a_bookmark_doesnt_suggest_from_itself() {
  let mut store = store().await;
  store.save(new("https://blog.example.com/one", &["reading"])).await.unwrap();

  // The only bookmark on this site is the one being edited, with its tag removed.
  assert!(suggested(&store, &draft("https://blog.example.com/one", &[])).await.is_empty());
}

#[tokio::test]
async fn hosts_only_match_exactly() {
  let mut store = store().await;
  store.save(new("https://box.com/file", &["storage"])).await.unwrap();

  assert!(suggested(&store, &draft("https://ox.com/page", &[])).await.is_empty());
}

fn preview(title: &str, keywords: &[&str], kind: &str) -> crate::PreviewData {
  crate::PreviewData {
    title: Some(title.into()),
    description: Some("A quick weeknight dinner.".into()),
    site_name: Some("Example Kitchen".into()),
    image_url: Some("https://kitchen.example/lasagna.jpg".into()),
    keywords: keywords.iter().map(|k| k.to_string()).collect(),
    page_kind: Some(kind.into()),
    excerpt: Some("Layer the sauce and pasta, then bake.".into()),
    ..Default::default()
  }
}

#[tokio::test]
async fn stores_a_preview_without_replacing_the_users_title() {
  let mut store = store().await;
  let saved = store.save(new("My lasagna https://kitchen.example/lasagna", &[])).await.unwrap().bookmark;
  let untitled = store.save(new("https://kitchen.example/soup", &[])).await.unwrap().bookmark;
  assert_eq!(saved.preview_status, PreviewStatus::None);

  let b = store.set_preview(&saved.id, &preview("Weeknight Lasagna", &["pasta", "Italian"], "recipe")).await.unwrap();
  assert_eq!(b.title.as_deref(), Some("My lasagna"));
  assert_eq!(b.description.as_deref(), Some("A quick weeknight dinner."));
  assert_eq!(b.keywords, ["pasta", "Italian"]);
  assert_eq!(b.page_kind.as_deref(), Some("recipe"));
  assert_eq!(b.preview_status, PreviewStatus::Done);
  assert!(b.preview_fetched_at.is_some());

  // No title yet: the preview's is used.
  let b = store.set_preview(&untitled.id, &preview("Tomato Soup", &[], "recipe")).await.unwrap();
  assert_eq!(b.title.as_deref(), Some("Tomato Soup"));
}

#[tokio::test]
async fn the_preview_queue_holds_pending_bookmarks_newest_first() {
  let mut store = store().await;
  let a = store.save(new("https://a.example/1", &[])).await.unwrap().bookmark;
  let b = store.save(new("https://b.example/2", &[])).await.unwrap().bookmark;
  let c = store.save(new("https://c.example/3", &[])).await.unwrap().bookmark;
  store.set_preview_status(&c.id, PreviewStatus::Failed).await.unwrap();
  assert!(store.preview_queue(10).await.unwrap().is_empty());

  // Switching previews on queues those that never had one; failed ones stay failed.
  assert_eq!(store.queue_missing_previews().await.unwrap(), 2);
  let queue = store.preview_queue(10).await.unwrap();
  assert_eq!(queue, [(b.id.clone(), b.url.clone()), (a.id.clone(), a.url.clone())]);

  store.set_preview(&b.id, &crate::PreviewData::default()).await.unwrap();
  assert_eq!(store.preview_queue(10).await.unwrap(), [(a.id.clone(), a.url.clone())]);
  assert!(matches!(store.set_preview_status("missing", PreviewStatus::Pending).await, Err(Error::NotFound)));
}

#[tokio::test]
async fn suggestions_use_the_page_keywords_and_kind() {
  use crate::Reason::*;
  let mut store = store().await;
  store.save(new("https://other.example/1", &["Pasta"])).await.unwrap();

  // A draft with a preview fetched for the save sheet.
  let mut d = draft("https://kitchen.example/lasagna", &[]);
  d.page = Some(crate::PageInfo { keywords: vec!["pasta".into()], kind: Some("recipe".into()), ..Default::default() });
  assert_eq!(suggested(&store, &d).await, [("Pasta".into(), Page), ("recipe".into(), Page)]);

  // Editing a saved bookmark: its stored preview is used. Its excerpt names pasta, which
  // counts as content (stronger than the keyword).
  let saved = store.save(new("https://kitchen.example/lasagna", &[])).await.unwrap().bookmark;
  store.set_preview(&saved.id, &preview("Lasagna", &["pasta"], "recipe")).await.unwrap();
  let got = suggested(&store, &draft("https://kitchen.example/lasagna", &[])).await;
  assert_eq!(got, [("Pasta".into(), Content), ("recipe".into(), Page)]);
}

#[tokio::test]
async fn a_version_1_database_keeps_its_bookmarks_when_migrated() {
  let conn = db::open(&Location::Memory).await.unwrap();
  conn.execute_batch(include_str!("../../migrations/0001_init.sql")).await.unwrap();
  conn.execute_batch("PRAGMA user_version = 1").await.unwrap();
  conn
    .execute(
      "INSERT INTO bookmark (id, url, original_url, title, shared_text, rules_revision, created_at, updated_at)
       VALUES ('b1', 'https://old.example/', 'https://old.example/?utm_source=x', 'Old one', 'Old one https://old.example/?utm_source=x', 'r1', 1, 1)",
      (),
    )
    .await
    .unwrap();

  let store = Store::new(conn, Arc::new(BundledRules), Arc::new(TestClock(AtomicI64::new(1_000)))).await.unwrap();
  let b = store.get("b1").await.unwrap().unwrap();
  assert_eq!(b.title.as_deref(), Some("Old one"));
  assert!(b.keywords.is_empty());
  assert_eq!((b.page_kind, b.excerpt), (None, None));
  assert_eq!(search(&store, "old").await, ["https://old.example/"]);
}

#[tokio::test]
async fn a_failed_retry_keeps_the_preview_it_had() {
  let mut store = store().await;
  let fresh = store.save(new("https://a.example/1", &[])).await.unwrap().bookmark;
  let had = store.save(new("https://b.example/2", &[])).await.unwrap().bookmark;
  store.set_preview(&had.id, &preview("Old preview", &[], "article")).await.unwrap();

  store.preview_failed(&fresh.id).await.unwrap();
  store.set_preview_status(&had.id, PreviewStatus::Pending).await.unwrap();
  store.preview_failed(&had.id).await.unwrap();

  assert_eq!(store.get(&fresh.id).await.unwrap().unwrap().preview_status, PreviewStatus::Failed);
  let kept = store.get(&had.id).await.unwrap().unwrap();
  assert_eq!(kept.preview_status, PreviewStatus::Done);
  assert_eq!(kept.title.as_deref(), Some("Old preview"));
}

const BROWSER_EXPORT: &str = r#"<!DOCTYPE NETSCAPE-Bookmark-file-1>
<DL><p>
  <DT><H3>Bookmarks bar</H3>
  <DL><p>
    <DT><A HREF="https://youtu.be/abc?si=TRACK" ADD_DATE="1700000000">A talk</A>
    <DT><H3>Recipes</H3>
    <DL><p>
      <DT><A HREF="https://kitchen.example/lasagna?utm_source=newsletter" ADD_DATE="1700000100" TAGS="pasta">Lasagna</A>
    </DL><p>
  </DL><p>
  <DT><A HREF="javascript:void(0)">Bookmarklet</A>
</DL>"#;

#[tokio::test]
async fn imports_cleaned_links_with_their_dates_and_folders() {
  let mut store = store().await;
  let report = store.import(BROWSER_EXPORT).await.unwrap();
  assert_eq!((report.added, report.merged, report.skipped), (2, 0, 0));

  let talk = store.find_by_url("https://youtu.be/abc").await.unwrap().unwrap();
  assert_eq!(talk.title.as_deref(), Some("A talk"));
  assert_eq!(talk.original_url, "https://youtu.be/abc?si=TRACK");
  assert_eq!(talk.created_at, 1_700_000_000_000);
  let lasagna = store.find_by_url("https://kitchen.example/lasagna").await.unwrap().unwrap();
  assert_eq!(tag_names(&lasagna), ["pasta", "Recipes"]);
  // Imported bookmarks are found by search like any other.
  assert_eq!(search(&store, "lasagna").await, ["https://kitchen.example/lasagna"]);
}

#[tokio::test]
async fn importing_twice_merges_instead_of_duplicating() {
  let mut store = store().await;
  store.save(new("My own title https://youtu.be/abc", &["mine"])).await.unwrap();
  store.import(BROWSER_EXPORT).await.unwrap();
  let report = store.import(BROWSER_EXPORT).await.unwrap();
  assert_eq!((report.added, report.merged), (0, 2));

  let talk = store.find_by_url("https://youtu.be/abc").await.unwrap().unwrap();
  // The title the user had wins; tags add up.
  assert_eq!(talk.title.as_deref(), Some("My own title"));
  assert_eq!(tag_names(&talk), ["mine"]);
  assert_eq!(store.list(&ListQuery::default()).await.unwrap().items.len(), 2);
}

#[tokio::test]
async fn a_json_backup_restores_into_an_empty_store() {
  let mut old = store().await;
  let mut b = new("Great talk https://youtu.be/abc?si=TRACK", &["video"]);
  b.note = Some("watch again".into());
  b.tags = vec![TagInput { name: "video".into(), source: TagSource::Suggested }];
  old.save(b).await.unwrap();
  old.save(new("https://kitchen.example/soup", &["food", "quick"])).await.unwrap();
  let backup = old.export_json().await.unwrap();

  let mut fresh = store().await;
  let report = fresh.import(&backup).await.unwrap();
  assert_eq!((report.format, report.added), (crate::transfer::Format::CleanShare, 2));
  let talk = fresh.find_by_url("https://youtu.be/abc").await.unwrap().unwrap();
  let before = old.find_by_url("https://youtu.be/abc").await.unwrap().unwrap();
  assert_eq!(talk.title.as_deref(), Some("Great talk"));
  assert_eq!(talk.note.as_deref(), Some("watch again"));
  assert_eq!(talk.tags, before.tags);
  assert_eq!(talk.created_at, before.created_at);
  assert_eq!(talk.id, before.id);
  let soup = fresh.find_by_url("https://kitchen.example/soup").await.unwrap().unwrap();
  assert_eq!(tag_names(&soup), ["food", "quick"]);
}

#[tokio::test]
async fn html_export_lists_every_bookmark_with_tags() {
  let mut store = store().await;
  store.save(new("Fish & chips https://kitchen.example/fish", &["food"])).await.unwrap();
  let html = store.export_html().await.unwrap();
  assert!(html.starts_with("<!DOCTYPE NETSCAPE-Bookmark-file-1>"));
  assert!(html.contains(r#"<A HREF="https://kitchen.example/fish""#));
  assert!(html.contains(r#"TAGS="food""#));
  assert!(html.contains(">Fish &amp; chips</A>"));
}

#[tokio::test]
async fn files_that_arent_exports_are_refused() {
  let mut store = store().await;
  assert!(matches!(store.import("hello").await, Err(crate::ImportError::Parse(_))));
}

#[tokio::test]
async fn a_big_import_adds_everything() {
  let mut store = store().await;
  let rows: String = (0..300)
    .map(|i| format!("Item {i},https://site{}.example/page/{i},1700000000,tag{}|common,unread\n", i % 50, i % 20))
    .collect();
  let report = store.import(&format!("title,url,time_added,tags,status\n{rows}")).await.unwrap();
  assert_eq!((report.added, report.merged, report.skipped), (300, 0, 0));
  let common = store.tags().await.unwrap().into_iter().find(|t| t.name == "common").unwrap();
  assert_eq!(common.count, 300);
}

/// Newer rules: the bundled ones, plus removing `ref` from every link.
struct StricterRules;

impl Cleaner for StricterRules {
  fn links(&self, text: &str) -> Vec<link_cleaner::Link> {
    BundledRules
      .links(text)
      .into_iter()
      .map(|mut link| {
        if let Some((base, query)) = link.cleaned.split_once('?') {
          let kept: Vec<&str> = query.split('&').filter(|p| !p.starts_with("ref=")).collect();
          link.cleaned = if kept.is_empty() { base.to_string() } else { format!("{base}?{}", kept.join("&")) };
        }
        link
      })
      .collect()
  }

  fn revision(&self) -> &str {
    "stricter"
  }
}

#[tokio::test]
async fn newer_rules_clean_saved_links_again() {
  let mut store = store().await;
  let b = store.save(new("https://shop.example/item?id=7&ref=newsletter", &["shop"])).await.unwrap().bookmark;
  let plain = store.save(new("https://plain.example/page", &[])).await.unwrap().bookmark;
  assert_eq!(b.url, "https://shop.example/item?id=7&ref=newsletter");

  store.set_cleaner(Arc::new(StricterRules));
  let report = store.reclean(false).await.unwrap();
  assert_eq!((report.checked, report.changed, report.merged), (2, 1, 0));

  let after = store.get(&b.id).await.unwrap().unwrap();
  assert_eq!(after.url, "https://shop.example/item?id=7");
  assert_eq!(after.original_url, "https://shop.example/item?id=7&ref=newsletter");
  assert_eq!(after.rules_revision, "stricter");
  assert_eq!(tag_names(&after), ["shop"]);
  assert_eq!(store.get(&plain.id).await.unwrap().unwrap().rules_revision, "stricter");

  // Nothing left with the old rules: a second run looks at nothing; `all` looks at everything.
  assert_eq!(store.reclean(false).await.unwrap().checked, 0);
  assert_eq!(store.reclean(true).await.unwrap(), RecleanReport { checked: 2, ..Default::default() });
}

#[tokio::test]
async fn a_link_that_becomes_a_saved_link_is_merged_into_the_older_one() {
  let mut store = store().await;
  let mut older = new("https://shop.example/item?id=7", &["shop"]);
  older.note = Some("the good one".into());
  let older = store.save(older).await.unwrap().bookmark;
  let newer = store.save(new("Item seven https://shop.example/item?id=7&ref=ad", &["deal"])).await.unwrap().bookmark;

  store.set_cleaner(Arc::new(StricterRules));
  let report = store.reclean(false).await.unwrap();
  assert_eq!((report.changed, report.merged), (1, 1));
  assert_eq!(report.removed, std::slice::from_ref(&newer.id));

  assert_eq!(store.get(&newer.id).await.unwrap(), None);
  let kept = store.get(&older.id).await.unwrap().unwrap();
  assert_eq!(kept.url, "https://shop.example/item?id=7");
  assert_eq!(tag_names(&kept), ["shop", "deal"]);
  // The older one had no title: it takes the newer one's; its own note stays.
  assert_eq!(kept.title.as_deref(), Some("Item seven"));
  assert_eq!(kept.note.as_deref(), Some("the good one"));
  assert_eq!(kept.created_at, older.created_at);
  // The merged tags are searchable.
  assert_eq!(search(&store, "deal").await, ["https://shop.example/item?id=7"]);
}
