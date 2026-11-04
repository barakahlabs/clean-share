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
  /// True if this piece isn't in the original: the link a redirect leads to, shown right after
  /// the redirect link it replaces.
  #[serde(skip_serializing_if = "std::ops::Not::not")]
  pub inserted: bool,
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
  /// How many redirect links were replaced by the link they lead to.
  pub redirect_count: usize,
}

/// A link found in shared text, as shared and after cleaning.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Link {
  pub original: String,
  pub cleaned: String,
  /// Id of the provider whose domains cover this link (e.g. `youtube`), if any.
  pub provider: Option<String>,
}

/// Analyzes `text` with the bundled rules.
#[cfg(feature = "bundled")]
pub fn analyze(text: &str) -> Analysis {
  analyze_with(bundled_rules(), text)
}

/// The links in `text`, in order, cleaned with the bundled rules.
#[cfg(feature = "bundled")]
pub fn links(text: &str) -> Vec<Link> {
  links_with(bundled_rules(), text)
}

pub fn links_with(rules: &RuleSet, text: &str) -> Vec<Link> {
  find_urls(text)
    .into_iter()
    .map(|range| {
      let original = &text[range];
      let cleaned = clean_url(rules, original).cleaned;
      Link {
        provider: provider_for(rules, &host_of(&cleaned)).map(|p| p.id.as_str().to_string()),
        original: original.to_string(),
        cleaned,
      }
    })
    .collect()
}

fn provider_for<'a>(rules: &'a RuleSet, host: &str) -> Option<&'a link_rules::Provider> {
  rules.providers.iter().find(|p| p.domains.iter().any(|d| d.matches(host)))
}

pub fn analyze_with(rules: &RuleSet, text: &str) -> Analysis {
  let mut tracking_ranges = Vec::new();
  let mut cleaned = String::with_capacity(text.len());
  let mut tracking_count = 0;
  let mut redirect_count = 0;
  let mut pos = 0;

  for url in find_urls(text) {
    let link = clean_url(rules, &text[url.clone()]);
    cleaned.push_str(&text[pos..url.start]);
    cleaned.push_str(&link.cleaned);
    tracking_count += link.params;
    redirect_count += link.redirects;
    if link.redirects > 0 {
      tracking_ranges.push((url.clone(), Some(link.cleaned)));
    } else {
      tracking_ranges.extend(link.removed.iter().map(|r| (r.start + url.start..r.end + url.start, None)));
    }
    pos = url.end;
  }
  cleaned.push_str(&text[pos..]);

  Analysis {
    segments: segments(text, &tracking_ranges),
    cleaned,
    tracking_count,
    redirect_count,
  }
}

struct CleanedUrl {
  cleaned: String,
  /// Ranges (within the URL) of each removed parameter, including its `?`/`&`. Empty for a
  /// redirect: the whole URL goes.
  removed: Vec<Range<usize>>,
  /// Tracking parameters removed (for a redirect, those in the link it leads to).
  params: usize,
  /// Redirects unwrapped (more than one if a redirect leads to another).
  redirects: usize,
}

/// How many redirects inside redirects are followed.
const MAX_REDIRECTS: usize = 3;

fn clean_url(rules: &RuleSet, url: &str) -> CleanedUrl {
  clean_url_within(rules, url, MAX_REDIRECTS)
}

fn clean_url_within(rules: &RuleSet, url: &str, redirects_left: usize) -> CleanedUrl {
  if redirects_left > 0
    && let Some(target) = redirect_target(rules, url)
  {
    let inner = clean_url_within(rules, &target, redirects_left - 1);
    return CleanedUrl { redirects: inner.redirects + 1, removed: Vec::new(), ..inner };
  }
  let fragment_start = url.find('#').unwrap_or(url.len());
  let Some(query_start) = url[..fragment_start].find('?') else {
    return CleanedUrl { cleaned: url.to_string(), removed: Vec::new(), params: 0, redirects: 0 };
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
  CleanedUrl { cleaned, params: removed.len(), removed, redirects: 0 }
}

/// The link `url` forwards to, if it's a redirect link in the rules and carries a valid link.
fn redirect_target(rules: &RuleSet, url: &str) -> Option<String> {
  let provider = provider_for(rules, &host_of(url))?;
  let path = link_rules::url_path(url);
  let redirect = provider.redirects.iter().find(|r| r.path == path)?;
  let fragment_start = url.find('#').unwrap_or(url.len());
  let query = &url[url[..fragment_start].find('?')? + 1..fragment_start];
  redirect.params.iter().find_map(|name| {
    query
      .split('&')
      .filter_map(|p| p.split_once('='))
      .filter(|(n, _)| n == name)
      .find_map(|(_, value)| percent_decode(value).filter(|v| is_web_link(v)))
  })
}

/// Decodes `%XX` escapes; None if they don't make valid UTF-8.
fn percent_decode(value: &str) -> Option<String> {
  let bytes = value.as_bytes();
  let mut out = Vec::with_capacity(bytes.len());
  let mut i = 0;
  while i < bytes.len() {
    let hex = |b: u8| (b as char).to_digit(16);
    if bytes[i] == b'%'
      && let (Some(hi), Some(lo)) = (bytes.get(i + 1).and_then(|&b| hex(b)), bytes.get(i + 2).and_then(|&b| hex(b)))
    {
      out.push((hi * 16 + lo) as u8);
      i += 3;
    } else {
      out.push(bytes[i]);
      i += 1;
    }
  }
  String::from_utf8(out).ok()
}

/// An http(s) link with a host and no spaces: safe to put in place of the redirect.
fn is_web_link(link: &str) -> bool {
  let lower = link.to_ascii_lowercase();
  let rest = lower.strip_prefix("https://").or_else(|| lower.strip_prefix("http://"));
  rest.is_some_and(|r| !r.is_empty() && !r.starts_with(['/', '?', '#']))
    && !link.chars().any(|c| c.is_whitespace() || c.is_control())
}

/// True if query parameter `name` is tracking for a link on `host` at `path`.
pub fn is_tracking(rules: &RuleSet, host: &str, path: &str, name: &str) -> bool {
  if let Some(provider) = provider_for(rules, host) {
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
    // A test for a feature this code doesn't have (newer rules): skipped, not failed.
    .filter(|(_, test)| test.supported())
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
  while let Some(last) = url[..end].chars().last() {
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

/// Splits `text` at the (sorted, non-overlapping) tracking ranges, merging neighbours. A range
/// with a replacement (a redirect) is followed by the replacement, as an inserted piece.
fn segments(text: &str, tracking: &[(Range<usize>, Option<String>)]) -> Vec<Segment> {
  let mut out: Vec<Segment> = Vec::new();
  let mut push = |piece: &str, tracking: bool, inserted: bool| {
    if piece.is_empty() {
      return;
    }
    match out.last_mut() {
      Some(last) if last.tracking == tracking && !last.inserted && !inserted => last.text.push_str(piece),
      _ => out.push(Segment { text: piece.to_string(), tracking, inserted }),
    }
  };

  let mut pos = 0;
  for (range, replacement) in tracking {
    push(&text[pos..range.start], false, false);
    push(&text[range.clone()], true, false);
    if let Some(replacement) = replacement {
      push(replacement, false, true);
    }
    pos = range.end;
  }
  push(&text[pos..], false, false);
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
  fn a_redirect_is_highlighted_whole_and_followed_by_its_link() {
    let text = "see https://l.facebook.com/l.php?u=https%3A%2F%2Fexample.com%2Fa%3Futm_source%3Dfb&h=AT1 now";
    let analysis = analyze(text);
    assert_eq!(analysis.cleaned, "see https://example.com/a now");
    assert_eq!((analysis.redirect_count, analysis.tracking_count), (1, 1));
    let pieces: Vec<_> = analysis.segments.iter().map(|s| (s.text.as_str(), s.tracking, s.inserted)).collect();
    assert_eq!(pieces, [
      ("see ", false, false),
      ("https://l.facebook.com/l.php?u=https%3A%2F%2Fexample.com%2Fa%3Futm_source%3Dfb&h=AT1", true, false),
      ("https://example.com/a", false, true),
      (" now", false, false),
    ]);
    assert_eq!(links(text)[0].provider, None, "the provider is the destination's");
  }

  #[test]
  fn redirects_without_a_web_link_are_cleaned_as_usual() {
    for link in [
      "https://www.google.com/url?q=javascript%3Aalert(1)",
      "https://www.google.com/url?q=not%20a%20link",
      "https://www.google.com/url?q=https%3A%2F%2F",
      "https://www.google.com/url?q=%FF%FE",
      "https://www.google.com/url?sa=t",
    ] {
      let analysis = analyze(link);
      assert_eq!((analysis.cleaned.as_str(), analysis.redirect_count), (link, 0), "{link}");
    }
  }

  #[test]
  fn redirects_inside_redirects_are_followed_a_few_times() {
    let inner = "https%3A%2F%2Fl.facebook.com%2Fl.php%3Fu%3Dhttps%253A%252F%252Fexample.com%252F";
    let analysis = analyze(&format!("https://www.google.com/url?q={inner}"));
    assert_eq!((analysis.cleaned.as_str(), analysis.redirect_count), ("https://example.com/", 2));
  }

  #[test]
  fn tests_for_unknown_features_are_skipped() {
    let mut rules = bundled_rules().clone();
    rules.global.tests.push(link_rules::TestCase {
      input: "https://example.com/?a=1".into(),
      output: "something a future feature makes".into(),
      needs: vec!["teleport".into()],
    });
    assert!(check_rule_tests(&rules).is_empty());
    assert_eq!(link_rules::lint(&rules), ["global: test `https://example.com/?a=1` needs unknown feature `teleport`"]);
  }

  #[test]
  fn redirect_tests_must_say_they_need_redirects() {
    let mut rules = bundled_rules().clone();
    let google = rules.providers.iter_mut().find(|p| p.id.as_str() == "google").unwrap();
    google.tests[0].needs.clear();
    assert_eq!(link_rules::lint(&rules).len(), 1);
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
  fn links_lists_each_link_with_its_provider() {
    let found = links("Watch https://youtu.be/x?si=abc then read https://example.com/a?utm_source=b.");
    assert_eq!(
      found,
      [
        Link {
          original: "https://youtu.be/x?si=abc".into(),
          cleaned: "https://youtu.be/x".into(),
          provider: Some("youtube".into()),
        },
        Link {
          original: "https://example.com/a?utm_source=b".into(),
          cleaned: "https://example.com/a".into(),
          provider: None,
        },
      ]
    );
    assert!(links("no links here").is_empty());
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
