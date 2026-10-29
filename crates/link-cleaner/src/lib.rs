//! Finds tracking parameters in links inside shared text and removes them.
//!
//! [`analyze`] returns the original text split into segments (so a UI can
//! highlight the tracking parts) and the cleaned text to share onwards.
//! Everything outside the removed parameters is kept byte-for-byte.

mod rules;

use std::ops::Range;

use serde::Serialize;

pub use rules::is_tracking;

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

pub fn analyze(text: &str) -> Analysis {
  let mut tracking_ranges = Vec::new();
  let mut cleaned = String::with_capacity(text.len());
  let mut tracking_count = 0;
  let mut pos = 0;

  for url in find_urls(text) {
    let link = clean_url(&text[url.clone()]);
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

fn clean_url(url: &str) -> CleanedUrl {
  let fragment_start = url.find('#').unwrap_or(url.len());
  let Some(query_start) = url[..fragment_start].find('?') else {
    return CleanedUrl { cleaned: url.to_string(), removed: Vec::new() };
  };

  let host = host_of(url);
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
    if is_tracking(&host, name) {
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

#[cfg(test)]
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

  #[test]
  fn youtube_removes_si_but_keeps_timestamp() {
    let link = "https://youtu.be/dQw4w9WgXcQ?si=AbCdEf123&t=42";
    assert_eq!(cleaned(link), "https://youtu.be/dQw4w9WgXcQ?t=42");
    assert_eq!(highlighted(link), ["?si=AbCdEf123"]);

    assert_eq!(
      cleaned("https://www.youtube.com/watch?v=dQw4w9WgXcQ&si=xyz&feature=shared"),
      "https://www.youtube.com/watch?v=dQw4w9WgXcQ"
    );
  }

  #[test]
  fn instagram_removes_igsh() {
    assert_eq!(
      cleaned("https://www.instagram.com/reel/C1a2B3c4D5e/?igsh=MWQ1ZGUxMzBkMA=="),
      "https://www.instagram.com/reel/C1a2B3c4D5e/"
    );
    assert_eq!(
      cleaned("https://www.instagram.com/p/abc/?utm_source=ig_web_copy_link&igshid=xyz"),
      "https://www.instagram.com/p/abc/"
    );
  }

  #[test]
  fn x_removes_share_tokens() {
    let link = "https://x.com/someone/status/1234567890?s=46&t=AbC-dEf_123";
    assert_eq!(cleaned(link), "https://x.com/someone/status/1234567890");
    assert_eq!(highlighted(link), ["?s=46&t=AbC-dEf_123"]);
  }

  #[test]
  fn x_removes_newer_a_share_token() {
    let link = "https://x.com/leviackermanft/status/2115652166325338163?a=uqa_a0BpGpaE4mGJaul&s=19";
    assert_eq!(cleaned(link), "https://x.com/leviackermanft/status/2115652166325338163");
    assert_eq!(highlighted(link), ["?a=uqa_a0BpGpaE4mGJaul&s=19"]);
  }

  #[test]
  fn site_rules_only_apply_to_their_site() {
    // `t` is tracking on X, but not on other sites.
    assert_eq!(cleaned("https://example.com/page?t=5"), "https://example.com/page?t=5");
    // Lookalike domains don't match.
    assert_eq!(cleaned("https://notx.com/a?s=1"), "https://notx.com/a?s=1");
  }

  #[test]
  fn tiktok_and_spotify() {
    assert_eq!(
      cleaned("https://www.tiktok.com/@user/video/123?_r=1&_t=ZM-abc&is_from_webapp=1"),
      "https://www.tiktok.com/@user/video/123"
    );
    assert_eq!(
      cleaned("https://open.spotify.com/track/4uLU6hMCjMI75M1A2tKUQC?si=a1b2c3d4"),
      "https://open.spotify.com/track/4uLU6hMCjMI75M1A2tKUQC"
    );
  }

  #[test]
  fn keeps_text_around_links_and_handles_several() {
    let text = "Look at this: https://x.com/a/status/1?s=20 and https://youtu.be/x?si=q.";
    assert_eq!(cleaned(text), "Look at this: https://x.com/a/status/1 and https://youtu.be/x.");
    assert_eq!(analyze(text).tracking_count, 2);
  }

  #[test]
  fn keeps_fragment_and_non_tracking_params_in_order() {
    assert_eq!(
      cleaned("https://example.com/a?utm_source=x&id=7&utm_medium=y&page=2#top"),
      "https://example.com/a?id=7&page=2#top"
    );
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
