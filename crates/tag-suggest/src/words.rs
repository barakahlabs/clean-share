//! Splitting text into comparable words: lowercase, letters and digits only, with a simple
//! singular form so `video` matches "videos" and `recipe` matches "recipes".

/// The stemmed words of `text`, in order.
pub fn stems(text: &str) -> Vec<String> {
  text
    .split(|c: char| !c.is_alphanumeric())
    .filter(|word| !word.is_empty())
    .map(|word| stem(&word.to_lowercase()))
    .collect()
}

/// One key per tag: its stemmed words joined by spaces. `"Recipes"` and `"recipe"` share one.
pub fn key(name: &str) -> String {
  stems(name).join(" ")
}

/// True if `phrase` (stemmed words) appears in `text` (stemmed words) as consecutive words.
pub fn contains(text: &[String], phrase: &[String]) -> bool {
  !phrase.is_empty() && text.windows(phrase.len()).any(|window| window == phrase)
}

/// English plural to singular, only the regular cases: "stories" → "story", "videos" → "video".
/// Short words and "-ss" words are left alone ("bus", "class").
fn stem(word: &str) -> String {
  let chars = word.chars().count();
  if chars > 4 && word.ends_with("ies") {
    format!("{}y", &word[..word.len() - 3])
  } else if chars > 3 && word.ends_with('s') && !word.ends_with("ss") {
    word[..word.len() - 1].to_string()
  } else {
    word.to_string()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn splits_on_anything_but_letters_and_digits() {
    assert_eq!(stems("Rust-lang: Async/await in 2026!"), ["rust", "lang", "async", "await", "in", "2026"]);
    assert_eq!(stems("https://www.youtube.com/watch"), ["http", "www", "youtube", "com", "watch"]);
  }

  #[test]
  fn plurals_become_singular() {
    assert_eq!(stems("videos stories recipes"), ["video", "story", "recipe"]);
    assert_eq!(stems("bus class news"), ["bus", "class", "new"]);
    assert_eq!(key("Cooking Recipes"), "cooking recipe");
  }

  #[test]
  fn phrases_match_consecutive_words_only() {
    let text = stems("A guide to machine learning basics");
    assert!(contains(&text, &stems("machine learning")));
    assert!(!contains(&text, &stems("learning machine")));
    assert!(!contains(&text, &[]));
  }

  #[test]
  fn non_ascii_words_are_kept() {
    assert_eq!(stems("Café crème — 東京"), ["café", "crème", "東京"]);
  }
}
