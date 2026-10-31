//! Finds tracking parameters in links inside shared text and removes them.
//!
//! [`analyze`] returns the original text split into segments (so a UI can
//! highlight the tracking parts) and the cleaned text to share onwards.
//! Everything outside the removed parameters is kept byte-for-byte.
//!
//! Which parameters count as tracking comes from the TOML files in `rules/`
//! (see `rules/README.md`), embedded at build time. [`analyze_with`] takes a
//! different [`RuleSet`], e.g. a newer one downloaded at runtime.

use std::ops::Range;

use serde::Serialize;

pub use link_rules::RuleSet;

/// The rules from `rules/`, validated at build time.
#[cfg(feature = "bundled")]
pub fn bundled_rules() -> &'static RuleSet {
  static RULES: std::sync::OnceLock<RuleSet> = std::sync::OnceLock::new();
  RULES.get_or_init(|| {
    serde_json::from_str(include_str!(concat!(env!("OUT_DIR"), "/rules.json")))
      .expect("embedded rules were validated at build time")
  })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Segment {
  pub text: String,
  /// True if this piece is tracking that the cleaned text drops.
  pub tracking: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Analysis {
  /// The original text, in order, split into tracking and non-tracking pieces.
  pub segments: Vec<Segment>,
  /// The text with all tracking parameters removed.
  pub cleaned: String,
  /// How many tracking parameters were found.
  pub tracking_count: usize,
}

/// Analyzes `text` with the bundled rules.
#[cfg(feature = "bundled")]
pub fn analyze(text: &str) -> Analysis {
  analyze_with(bundled_rules(), text)
}

pub fn analyze_with(rules: &RuleSet, text: &str) -> Analysis {
  let mut tracking_ranges = Vec::new();
  let mut cleaned = String::with_capacity(text.len());
  let mut tracking_count = 0;
  let mut pos = 0;

  for url in find_urls(text) {
    let link = clean_url(rules, &text[url.clone()]);
    cleaned.push_str(&text[pos..url.start]);
    cleaned.push_str(&link.cleaned);
    tracking_count += link.removed.len();
    tracking_ranges.extend(link.removed.iter().map(|r| r.start + url.start..r.end + url.start));
    pos = url.end;
  }
  cleaned.push_str(&text[pos..]);

  Analysis {
    segments: segments(text, &tracking_ranges),
    cleaned,
    tracking_count,
  }
}

struct CleanedUrl {
  cleaned: String,
  /// Ranges (within the URL) of each removed parameter, including its `?`/`&`.
  removed: Vec<Range<usize>>,
}

fn clean_url(rules: &RuleSet, url: &str) -> CleanedUrl {
  let fragment_start = url.find('#').unwrap_or(url.len());
  let Some(query_start) = url[..fragment_start].find('?') else {
    return CleanedUrl { cleaned: url.to_string(), removed: Vec::new() };
  };

  let host = host_of(url);
  let path = path_of(url);
  let mut kept = Vec::new();
  let mut removed = Vec::new();
  let mut start = query_start + 1;

  for param in url[query_start + 1..fragment_start].split('&') {
    let range = start..start + param.len();
    start = range.end + 1;
    if param.is_empty() {
      continue;
    }
    let name = param.split('=').next().unwrap_or(param);
    if is_tracking(rules, &host, path, name) {
      // Include the separator before it, so "?" disappears when nothing is left.
      removed.push(range.start - 1..range.end);
    } else {
      kept.push(param);
    }
  }

  let mut cleaned = url[..query_start].to_string();
  if !kept.is_empty() {
    cleaned.push('?');
    cleaned.push_str(&kept.join("&"));
  }
  cleaned.push_str(&url[fragment_start..]);
  CleanedUrl { cleaned, removed }
}

/// True if query parameter `name` is tracking for a link on `host` at `path`.
pub fn is_tracking(rules: &RuleSet, host: &str, path: &str, name: &str) -> bool {
  let provider = rules
    .providers
    .iter()
    .find(|p| p.domains.iter().any(|d| d.matches(host)));

  if let Some(provider) = provider {
    if provider.keep.iter().any(|k| k.matches(name)) {
      return false;
    }
    let excepted = provider.except.iter().any(|prefix| path.starts_with(prefix.as_str()));
    if !excepted
      && (provider.strip_unknown || provider.remove.iter().any(|r| r.param.matches(name)))
    {
      return true;
    }
  }
  rules.global.remove.iter().any(|r| r.param.matches(name))
}

/// Runs every `[[test]]` in the rules. Returns one message per failing test.
pub fn check_rule_tests(rules: &RuleSet) -> Vec<String> {
  let global = rules.global.tests.iter().map(|t| ("global", t));
  let providers = rules
    .providers
    .iter()
    .flat_map(|p| p.tests.iter().map(move |t| (p.id.as_str(), t)));

  global
    .chain(providers)
    .filter_map(|(source, test)| {
      let actual = analyze_with(rules, &test.input).cleaned;
      (actual != test.output).then(|| {
        format!("{source}: {}\n  expected {}\n  got      {actual}", test.input, test.output)
      })
    })
    .collect()
}

/// Path part of a URL, e.g. "/i/redirect" (empty if none).
fn path_of(url: &str) -> &str {
  let after_scheme = url.split_once("://").map_or(url, |(_, rest)| rest);
  let rest = &after_scheme[after_scheme.find(['/', '?', '#']).unwrap_or(after_scheme.len())..];
  &rest[..rest.find(['?', '#']).unwrap_or(rest.len())]
}

/// Lowercase host without credentials or port, e.g. "www.youtube.com".
fn host_of(url: &str) -> String {
  let after_scheme = url.split_once("://").map_or(url, |(_, rest)| rest);
  let authority = after_scheme
    .split(['/', '?', '#'])
    .next()
    .unwrap_or_default();
  let host = authority.rsplit('@').next().unwrap_or(authority);
  let host = host.split(':').next().unwrap_or(host);
  host.to_ascii_lowercase()
}

/// Byte ranges of http(s) URLs in `text`.
fn find_urls(text: &str) -> Vec<Range<usize>> {
  // ASCII lowercasing keeps byte offsets identical.
  let lower = text.to_ascii_lowercase();
  let mut urls = Vec::new();
  let mut from = 0;

  while let Some(start) = next_url_start(&lower, from) {
    let len = text[start..]
      .find(|c: char| c.is_whitespace() || matches!(c, '<' | '>' | '"' | '\'' | '`'))
      .unwrap_or(text.len() - start);
    let end = start + trim_trailing_punctuation(&text[start..start + len]);
    urls.push(start..end);
    from = start + len;
  }
  urls
}

fn next_url_start(lower: &str, from: usize) -> Option<usize> {
  let https = lower[from..].find("https://");
  let http = lower[from..].find("http://");
  match (https, http) {
    (Some(a), Some(b)) => Some(from + a.min(b)),
    (a, b) => a.or(b).map(|i| from + i),
  }
}

/// Length of `url` without sentence punctuation stuck to its end ("see https://a.com/x.").
fn trim_trailing_punctuation(url: &str) -> usize {
  let mut end = url.len();
  loop {
    let Some(last) = url[..end].chars().last() else { break };
    let unbalanced_paren =
      last == ')' && url[..end].matches('(').count() < url[..end].matches(')').count();
    if matches!(last, '.' | ',' | ';' | ':' | '!' | '?') || unbalanced_paren {
      end -= last.len_utf8();
    } else {
      break;
    }
  }
  end
}

/// Splits `text` at the (sorted, non-overlapping) tracking ranges, merging neighbours.
fn segments(text: &str, tracking: &[Range<usize>]) -> Vec<Segment> {
  let mut out: Vec<Segment> = Vec::new();
  let mut push = |piece: &str, is_tracking: bool| {
    if piece.is_empty() {
      return;
    }
    match out.last_mut() {
      Some(last) if last.tracking == is_tracking => last.text.push_str(piece),
      _ => out.push(Segment { text: piece.to_string(), tracking: is_tracking }),
    }
  };

  let mut pos = 0;
  for range in tracking {
    push(&text[pos..range.start], false);
    push(&text[range.clone()], true);
    pos = range.end;
  }
  push(&text[pos..], false);
  out
}

#[cfg(all(test, feature = "bundled"))]
mod tests {
  use super::*;

  fn cleaned(text: &str) -> String {
    analyze(text).cleaned
  }

  fn highlighted(text: &str) -> Vec<String> {
    analyze(text)
      .segments
      .into_iter()
      .filter(|s| s.tracking)
      .map(|s| s.text)
      .collect()
  }

  /// Every [[test]] in rules/*.toml must pass.
  #[test]
  fn bundled_rule_tests_pass() {
    let failures = check_rule_tests(bundled_rules());
    assert!(failures.is_empty(), "failing rule tests:\n{}", failures.join("\n"));
  }

  #[test]
  fn highlights_tracking_with_its_separator() {
    let link = "https://x.com/someone/status/1?s=46&t=AbC";
    assert_eq!(highlighted(link), ["?s=46&t=AbC"]);
    assert_eq!(highlighted("https://youtu.be/x?si=abc&t=42"), ["?si=abc"]);
  }

  #[test]
  fn site_rules_only_apply_to_their_site() {
    // `t` is tracking on X, kept on YouTube (start time), untouched elsewhere.
    assert_eq!(cleaned("https://x.com/a/status/1?t=5"), "https://x.com/a/status/1");
    assert_eq!(cleaned("https://youtu.be/x?t=5"), "https://youtu.be/x?t=5");
    assert_eq!(cleaned("https://example.com/page?t=5"), "https://example.com/page?t=5");
    // `a` is tracking only on X.
    assert_eq!(cleaned("https://www.reddit.com/r/x/?a=1"), "https://www.reddit.com/r/x/?a=1");
    // Lookalike domains don't match.
    assert_eq!(cleaned("https://notx.com/a?s=1"), "https://notx.com/a?s=1");
  }

  #[test]
  fn keep_wins_over_remove_and_except_skips_provider_rules() {
    let mut rules = bundled_rules().clone();
    let x = rules.providers.iter_mut().find(|p| p.id.as_str() == "x").unwrap();
    x.keep.push("s".to_string().try_into().unwrap());
    assert_eq!(analyze_with(&rules, "https://x.com/a?s=1&t=2").cleaned, "https://x.com/a?s=1");

    // X's /i/redirect is excepted: provider rules are skipped, global ones still apply.
    assert_eq!(
      cleaned("https://x.com/i/redirect?t=1&utm_source=x"),
      "https://x.com/i/redirect?t=1"
    );
  }

  #[test]
  fn strip_unknown_removes_everything_except_keep() {
    // Instagram uses strip_unknown: names we've never seen are removed too.
    assert_eq!(
      cleaned("https://www.instagram.com/reel/abc/?zqxw=1&img_index=3&brandnew=x"),
      "https://www.instagram.com/reel/abc/?img_index=3"
    );
    // Other providers only remove what they list.
    assert_eq!(cleaned("https://youtu.be/x?zqxw=1"), "https://youtu.be/x?zqxw=1");
  }

  #[test]
  fn keeps_text_around_links_and_handles_several() {
    let text = "Look at this: https://x.com/a/status/1?s=20 and https://youtu.be/x?si=q.";
    assert_eq!(cleaned(text), "Look at this: https://x.com/a/status/1 and https://youtu.be/x.");
    assert_eq!(analyze(text).tracking_count, 2);
  }

  #[test]
  fn segments_rebuild_the_original_text() {
    let text = "hi https://example.com/?utm_source=a&keep=1&fbclid=b bye";
    let joined: String = analyze(text).segments.iter().map(|s| s.text.as_str()).collect();
    assert_eq!(joined, text);
    assert_eq!(highlighted(text), ["?utm_source=a", "&fbclid=b"]);
  }

  #[test]
  fn leaves_text_without_tracking_alone() {
    for text in ["just some words", "https://example.com/path?q=rust", "ünïcödé https://a.com/ é"] {
      let result = analyze(text);
      assert_eq!(result.cleaned, text);
      assert_eq!(result.tracking_count, 0);
      assert!(result.segments.iter().all(|s| !s.tracking));
    }
  }

  #[test]
  fn ignores_case_and_ports() {
    assert_eq!(
      cleaned("HTTPS://WWW.YOUTUBE.COM:443/watch?v=1&SI=x"),
      "HTTPS://WWW.YOUTUBE.COM:443/watch?v=1"
    );
  }

  #[test]
  fn trims_closing_paren_only_when_unbalanced() {
    assert_eq!(cleaned("(see https://a.com/x?fbclid=1)"), "(see https://a.com/x)");
    assert_eq!(
      cleaned("https://en.wikipedia.org/wiki/Rust_(language)?utm_source=x"),
      "https://en.wikipedia.org/wiki/Rust_(language)"
    );
  }
}
