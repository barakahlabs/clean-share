use super::*;

fn tags(list: &[(&str, u32)]) -> Vec<TagUse> {
  list.iter().map(|(name, count)| TagUse { name: name.to_string(), count: *count }).collect()
}

fn content(text: &str, provider: Option<&str>) -> Content {
  Content { text: text.into(), provider: provider.map(Into::into) }
}

fn suggest(content: &Content, history: &History, picked: &[&str]) -> Vec<(String, Reason)> {
  let picked: Vec<String> = picked.iter().map(|p| p.to_string()).collect();
  MatchSuggester
    .suggest(content, history, &picked, 5)
    .into_iter()
    .map(|s| (s.name, s.reason))
    .collect()
}

fn names(list: &[(String, Reason)]) -> Vec<&str> {
  list.iter().map(|(name, _)| name.as_str()).collect()
}

#[test]
fn existing_tags_found_in_the_content() {
  let history = History { tags: tags(&[("Rust", 4), ("recipe", 2), ("go", 1)]), ..Default::default() };
  let got = suggest(&content("Async Rust recipes https://example.com/post", None), &history, &[]);
  assert_eq!(got, [("Rust".into(), Reason::Content), ("recipe".into(), Reason::Content)]);
}

#[test]
fn multi_word_tags_need_the_whole_phrase() {
  let history = History { tags: tags(&[("machine learning", 1), ("learning", 1)]), ..Default::default() };
  assert_eq!(
    names(&suggest(&content("Machine learning, explained", None), &history, &[])),
    ["learning", "machine learning"]
  );
  assert_eq!(names(&suggest(&content("Learning to cook", None), &history, &[])), ["learning"]);
}

#[test]
fn tags_from_the_same_site_rank_by_how_often_they_were_used() {
  let history = History {
    tags: tags(&[("music", 3), ("talks", 1), ("rust", 5)]),
    site: Uses { tags: tags(&[("music", 3), ("talks", 1)]), bookmarks: 4 },
    ..Default::default()
  };
  let got = suggest(&content("https://youtu.be/abc", None), &history, &[]);
  assert_eq!(got, [("music".into(), Reason::Site), ("talks".into(), Reason::Site)]);
}

#[test]
fn related_tags_follow_the_picked_ones() {
  let history = History {
    tags: tags(&[("rust", 5), ("async", 2), ("cooking", 2)]),
    related: Uses { tags: tags(&[("rust", 5), ("async", 2)]), bookmarks: 5 },
    ..Default::default()
  };
  // `rust` is picked: never suggested again, but it brings `async` along.
  let got = suggest(&content("A blog post", None), &history, &["Rust"]);
  assert_eq!(got, [("async".into(), Reason::Related)]);
}

#[test]
fn provider_tags_are_suggested_even_when_new() {
  assert_eq!(
    suggest(&content("https://youtu.be/abc", Some("youtube")), &History::default(), &[]),
    [("video".into(), Reason::Provider)]
  );
}

#[test]
fn provider_tags_use_the_existing_spelling() {
  let history = History { tags: tags(&[("Videos", 2)]), ..Default::default() };
  assert_eq!(
    suggest(&content("https://youtu.be/abc", Some("youtube")), &history, &[]),
    [("Videos".into(), Reason::Provider)]
  );
}

#[test]
fn reasons_add_up_and_the_strongest_is_reported() {
  // `music` is in the text (3.0) and on 1 of 4 bookmarks from the site (0.5 + 0.5).
  // `video` is from the provider only (1.0).
  let history = History {
    tags: tags(&[("music", 1)]),
    site: Uses { tags: tags(&[("music", 1)]), bookmarks: 4 },
    ..Default::default()
  };
  let all = MatchSuggester.suggest(
    &content("Music for programming", Some("youtube")),
    &history,
    &[],
    5,
  );
  assert_eq!(all.len(), 2);
  assert_eq!((all[0].name.as_str(), all[0].reason, all[0].score), ("music", Reason::Content, 4.0));
  assert_eq!((all[1].name.as_str(), all[1].reason, all[1].score), ("video", Reason::Provider, 1.0));
}

#[test]
fn ties_go_to_the_more_used_tag_then_by_name() {
  let history = History { tags: tags(&[("beta", 1), ("alpha", 1), ("gamma", 9)]), ..Default::default() };
  assert_eq!(
    names(&suggest(&content("alpha beta gamma", None), &history, &[])),
    ["gamma", "alpha", "beta"]
  );
}

#[test]
fn picked_tags_are_left_out_whatever_their_spelling() {
  let history = History { tags: tags(&[("Recipes", 1), ("baking", 1)]), ..Default::default() };
  assert_eq!(
    names(&suggest(&content("Baking recipes", None), &history, &["recipe"])),
    ["baking"]
  );
}

#[test]
fn at_most_limit_suggestions() {
  let history = History { tags: tags(&[("a1", 1), ("b2", 1), ("c3", 1)]), ..Default::default() };
  let got = MatchSuggester.suggest(&content("a1 b2 c3", None), &history, &[], 2);
  assert_eq!(got.len(), 2);
}

#[test]
fn nothing_to_go_on_means_no_suggestions() {
  let history = History { tags: tags(&[("rust", 3)]), ..Default::default() };
  assert!(suggest(&content("https://example.com/", None), &history, &[]).is_empty());
}
