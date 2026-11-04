//! Keywords from a page's readable text, for pages that declare none: no model, just counting.
//! Words score by where they appear (title 3, description/excerpt 2, shared text 1); common words
//! (English and German), web filler, generic words ("update", "guide"), past tenses and adverbs
//! ("plotted", "quickly"), numbers and the site's own name don't count. Two-word phrases ("machine
//! learning") count when they appear at least twice, and then replace their words.

use std::collections::HashMap;

use crate::words;

/// Where the text is from, and how much a word there counts.
pub struct Field<'a> {
  pub text: &'a str,
  pub weight: f32,
}

/// The best keywords, best first, at most `limit`, each scoring at least [`MIN_SCORE`] (once in the
/// title, or a few times elsewhere). Display form: lowercase, acronyms ("AI", "F1") as written.
pub fn pick(fields: &[Field], site: Option<&str>, limit: usize) -> Vec<(String, f32)> {
  let site_words: Vec<String> = site
    .map(|host| host.split(['.', '-']).filter(|w| w.len() > 2).map(|w| w.to_lowercase()).collect())
    .unwrap_or_default();
  let mut terms: HashMap<String, Term> = HashMap::new();

  for field in fields {
    let tokens: Vec<Token> = field
      .text
      .split(|c: char| !(c.is_alphanumeric() || c == '\''))
      .filter_map(|raw| Token::new(raw.trim_matches('\''), &site_words))
      .collect();
    for (i, token) in tokens.iter().enumerate() {
      let Some(token) = token.word() else { continue };
      terms.entry(token.key.clone()).or_insert_with(|| Term::new(&token.shown)).add(field.weight);
      if let Some(next) = tokens.get(i + 1).and_then(Token::word)
        && !token.acronym
        && !next.acronym
      {
        let key = format!("{} {}", token.key, next.key);
        terms
          .entry(key)
          .or_insert_with(|| Term::phrase(&format!("{} {}", token.shown, next.shown)))
          .add(field.weight);
      }
    }
  }

  // A phrase must appear at least twice: a title alone joins words at random ("hackers plotted").
  terms.retain(|_, t| !t.phrase || t.count >= 2);
  let mut ranked: Vec<(String, Term)> = terms.into_iter().filter(|(_, t)| t.score >= MIN_SCORE).collect();
  // Best score first; then phrases (more specific), then longer, then by name.
  ranked.sort_by(|(ka, a), (kb, b)| {
    b.score
      .total_cmp(&a.score)
      .then(b.phrase.cmp(&a.phrase))
      .then(kb.len().cmp(&ka.len()))
      .then(ka.cmp(kb))
  });

  let mut picked: Vec<(String, String, f32)> = Vec::new();
  for (key, term) in ranked {
    // A phrase covers its words, and a word inside a picked phrase adds nothing.
    let covered = picked.iter().any(|(k, ..)| k.split(' ').any(|w| w == key) || key.split(' ').any(|w| w == k));
    if covered {
      continue;
    }
    picked.push((key, term.shown, term.score));
    if picked.len() == limit {
      break;
    }
  }
  picked.into_iter().map(|(_, shown, score)| (shown, score)).collect()
}

/// Once in a title (3), or e.g. three times in a description.
pub const MIN_SCORE: f32 = 3.0;

struct Term {
  shown: String,
  score: f32,
  count: u32,
  phrase: bool,
}

impl Term {
  fn new(shown: &str) -> Self {
    Self { shown: shown.to_string(), score: 0.0, count: 0, phrase: false }
  }

  fn phrase(shown: &str) -> Self {
    Self { phrase: true, ..Self::new(shown) }
  }

  fn add(&mut self, weight: f32) {
    self.score += weight;
    self.count += 1;
  }
}

/// A word that could be a keyword, or a stop that breaks phrases.
enum Token {
  Word(Word),
  Stop,
}

struct Word {
  key: String,
  shown: String,
  acronym: bool,
}

impl Token {
  fn new(raw: &str, site_words: &[String]) -> Option<Self> {
    if raw.is_empty() {
      return None;
    }
    let lower = raw.to_lowercase();
    let letters = raw.chars().filter(|c| c.is_alphabetic()).count();
    let acronym = raw.chars().all(|c| c.is_uppercase() || c.is_ascii_digit())
      && letters >= 1
      && (2..=5).contains(&raw.chars().count());
    let too_short = raw.chars().count() < 4 && !acronym;
    let not_a_word = letters == 0 || (letters < raw.chars().count() / 2 && !acronym);
    // "plotted", "announced", "quickly": rarely what a page is about. ("-ing" stays: "cooking".)
    let inflected = !acronym && lower.chars().count() >= 5 && (lower.ends_with("ed") || lower.ends_with("ly"));
    if STOP.contains(&lower.as_str())
      || too_short
      || not_a_word
      || inflected
      || site_words.contains(&lower)
    {
      return Some(Token::Stop);
    }
    let shown = if acronym { raw.to_string() } else { lower };
    Some(Token::Word(Word { key: words::key(&shown), shown, acronym }))
  }

  fn word(&self) -> Option<&Word> {
    match self {
      Token::Word(w) => Some(w),
      Token::Stop => None,
    }
  }
}

/// Words that say nothing about what a page is about: common English and German words, and what
/// pages say about themselves (subscribe, cookies, …). Words under 4 letters are dropped anyway.
const STOP: &[&str] = &[
  // English
  "about", "above", "after", "again", "against", "also", "always", "among", "another", "anyone",
  "anything", "around", "away", "back", "based", "because", "been", "before", "being", "below",
  "best", "better", "between", "both", "came", "come", "comes", "could", "days", "does", "doing",
  "done", "down", "during", "each", "even", "ever", "every", "first", "from", "full", "gets",
  "give", "goes", "going", "good", "great", "have", "having", "here", "into", "just", "keep",
  "know", "last", "less", "like", "little", "long", "look", "looks", "made", "make", "makes",
  "many", "might", "more", "most", "much", "must", "need", "never", "next", "night", "nothing",
  "often", "once", "only", "other", "others", "over", "part", "people", "really", "right", "same",
  "says", "said", "should", "show", "since", "some", "something", "soon", "still", "such", "sure",
  "take", "than", "that", "their", "them", "then", "there", "these", "they", "thing", "things",
  "think", "this", "those", "though", "through", "time", "times", "today", "together", "took",
  "under", "until", "upon", "used", "uses", "using", "very", "want", "wants", "ways", "week",
  "well", "were", "what", "when", "where", "which", "while", "whole", "will", "with", "within",
  "without", "work", "works", "would", "year", "years", "your", "yours", "yourself", "it's",
  "don't", "doesn't", "can't", "i'm", "you're", "we're", "they're", "here's", "there's", "that's",
  "what's", "let's", "isn't", "won't", "didn't",
  // German
  "aber", "alle", "allem", "allen", "aller", "alles", "also", "andere", "anderen", "auch",
  "auf", "bereits", "beim", "bist", "bitte", "damit", "dann", "darum", "dass", "dein", "deine",
  "denn", "deren", "derer", "dessen", "diese", "diesem", "diesen", "dieser", "dieses", "doch",
  "dort", "durch", "eine", "einem", "einen", "einer", "eines", "etwa", "etwas", "euch", "euer",
  "für", "gegen", "gibt", "habe", "haben", "hatte", "hier", "hinter", "ihre", "ihrem", "ihren",
  "ihrer", "immer", "jede", "jedem", "jeden", "jeder", "jetzt", "kann", "kein", "keine", "können",
  "machen", "mehr", "mein", "meine", "mich", "mit", "muss", "nach", "nicht", "noch", "nun", "oder",
  "ohne", "schon", "sein", "seine", "sehr", "seit", "sich", "sie", "sind", "soll", "sondern",
  "sowie", "über", "uns", "unser", "unsere", "unter", "viel", "viele", "vom", "von", "vor", "war",
  "waren", "warum", "was", "weil", "weiter", "welche", "wenn", "werden", "wieder", "wird", "wurde",
  "wurden", "zum", "zur", "zwischen",
  // Generic words in titles and descriptions
  "announcing", "body", "breaking", "case", "guide", "how-to", "impact", "introducing",
  "latest", "list", "news", "official", "overview", "part", "release", "releases", "report",
  "review", "story", "tips", "update", "updates", "version", "watch",
  // What pages say about themselves
  "click", "comment", "comments", "cookie", "cookies", "copyright", "email", "follow", "free",
  "home", "html", "http", "https", "login", "menu", "newsletter", "page", "please", "post",
  "posts", "privacy", "read", "reading", "rights", "reserved", "search", "share", "sign",
  "site", "subscribe", "terms", "website", "welcome",
];

#[cfg(test)]
mod tests {
  use super::*;

  fn keywords(title: &str, description: &str, site: Option<&str>) -> Vec<String> {
    let fields = [Field { text: title, weight: 3.0 }, Field { text: description, weight: 2.0 }];
    pick(&fields, site, 3).into_iter().map(|(k, _)| k).collect()
  }

  #[test]
  fn title_words_and_acronyms() {
    let got = keywords(
      "Can Ferrari Win the 2027 F1 Championship?",
      "We look into the chances of Ferrari winning the 2027 F1 World Championship. Subscribe!",
      Some("www.youtube.com"),
    );
    // Equal scores (title + description each); the order among them doesn't matter.
    let mut got = got;
    got.sort();
    assert_eq!(got, ["F1", "championship", "ferrari"]);
  }

  #[test]
  fn phrases_beat_their_words() {
    let got = keywords(
      "A gentle intro to machine learning",
      "Machine learning basics: models, training data and machine learning pitfalls.",
      None,
    );
    assert_eq!(got[0], "machine learning");
    assert!(!got.contains(&"machine".to_string()) && !got.contains(&"learning".to_string()));
  }

  #[test]
  fn the_sites_name_and_filler_dont_count() {
    let got = keywords(
      "Binance Says Hackers Plotted $6.3 Billion Heist for Months | The New York Times",
      "Subscribe to read. Cookies help us. A report from the cryptocurrency exchange.",
      Some("www.nytimes.com"),
    );
    assert!(got.contains(&"binance".to_string()), "{got:?}");
    for junk in ["subscribe", "cookies", "nytimes", "6.3"] {
      assert!(!got.iter().any(|k| k.contains(junk)), "{junk} in {got:?}");
    }
  }

  #[test]
  fn german_text_works_too() {
    let got = keywords("Rezept für einen schnellen Käsekuchen", "Dieser Käsekuchen ist schnell gemacht.", None);
    assert!(got.contains(&"käsekuchen".to_string()), "{got:?}");
    assert!(!got.iter().any(|k| k == "für" || k == "dieser"), "{got:?}");
  }

  #[test]
  fn words_seen_once_outside_the_title_are_not_enough() {
    assert!(keywords("", "Some text mentions zebras once.", None).is_empty());
  }
}
