//! Netscape bookmark HTML: the format browsers export and import. It isn't well-formed HTML
//! (`<DT>` and `<p>` are never closed), so it's read with a small scanner, not an HTML parser:
//! `<H3>` names a folder, the `<DL>` after it holds the folder's bookmarks, `<A HREF>` is a
//! bookmark and a `<DD>` right after it is its description.

use super::{ImportItem, date, split_tags, text};
use crate::{Bookmark, TagInput};

/// The browsers' own top folders: not tags.
const ROOT_FOLDERS: &[&str] = &[
  "bookmarks",
  "bookmarks bar",
  "bookmarks toolbar",
  "bookmarks menu",
  "other bookmarks",
  "mobile bookmarks",
  "favorites",
  "favourites",
  "favorites bar",
  "reading list",
  "unsorted",
  "imported",
  "clean share",
];

pub fn parse(html: &str) -> Vec<ImportItem> {
  let mut items: Vec<ImportItem> = Vec::new();
  // Folder names, innermost last; `None` for a `<DL>` without a folder heading.
  let mut folders: Vec<Option<String>> = Vec::new();
  let mut pending_folder: Option<String> = None;
  let mut rest = html;

  while let Some(at) = rest.find('<') {
    rest = &rest[at + 1..];
    let end = rest.find('>').unwrap_or(rest.len());
    let tag = &rest[..end];
    let name = tag.split(|c: char| c.is_whitespace() || c == '/').find(|s| !s.is_empty()).unwrap_or("");
    let closing = tag.starts_with('/');
    let after = &rest[(end + 1).min(rest.len())..];
    match (closing, name.to_ascii_lowercase().as_str()) {
      (false, "h3") => {
        pending_folder = text(&unescape(until_tag(after)));
      }
      (false, "dl") => folders.push(pending_folder.take()),
      (true, "dl") => {
        folders.pop();
      }
      (false, "a") => {
        let attr = |key: &str| attribute(tag, key);
        let Some(url) = attr("href").filter(|u| u.starts_with("http://") || u.starts_with("https://")) else {
          continue;
        };
        let mut tags = split_tags(&attr("tags").unwrap_or_default(), &[',']);
        if let Some(Some(folder)) = folders.last()
          && !ROOT_FOLDERS.contains(&folder.to_lowercase().as_str())
        {
          tags.push(TagInput::user(folder.clone()));
        }
        items.push(ImportItem {
          url,
          title: text(&unescape(until_tag(after))),
          note: None,
          tags,
          created_at: attr("add_date").or_else(|| attr("time_added")).and_then(|d| date::parse(&d)),
        });
      }
      (false, "dd") => {
        if let Some(last) = items.last_mut().filter(|i| i.note.is_none()) {
          last.note = text(&unescape(until_tag(after)));
        }
      }
      _ => {}
    }
    rest = after;
  }
  items
}

/// Writes bookmarks as Netscape bookmark HTML, in one "Clean Share" folder, with tags in the
/// `TAGS` attribute (Firefox reads them) and notes as descriptions.
pub fn export(bookmarks: &[Bookmark]) -> String {
  let mut out = String::from(
    "<!DOCTYPE NETSCAPE-Bookmark-file-1>\n\
     <!-- This is an automatically generated file. It will be read and overwritten. DO NOT EDIT! -->\n\
     <META HTTP-EQUIV=\"Content-Type\" CONTENT=\"text/html; charset=UTF-8\">\n\
     <TITLE>Bookmarks</TITLE>\n\
     <H1>Bookmarks</H1>\n\
     <DL><p>\n    <DT><H3>Clean Share</H3>\n    <DL><p>\n",
  );
  for b in bookmarks {
    let tags: Vec<&str> = b.tags.iter().map(|t| t.name.as_str()).collect();
    out.push_str(&format!(
      "        <DT><A HREF=\"{}\" ADD_DATE=\"{}\" LAST_MODIFIED=\"{}\"{}>{}</A>\n",
      escape(&b.url),
      b.created_at / 1000,
      b.updated_at / 1000,
      if tags.is_empty() { String::new() } else { format!(" TAGS=\"{}\"", escape(&tags.join(","))) },
      escape(b.title.as_deref().unwrap_or(&b.url)),
    ));
    if let Some(note) = &b.note {
      out.push_str(&format!("        <DD>{}\n", escape(note)));
    }
  }
  out.push_str("    </DL><p>\n</DL><p>\n");
  out
}

/// Text up to the next tag.
fn until_tag(s: &str) -> &str {
  &s[..s.find('<').unwrap_or(s.len())]
}

/// `KEY="value"` (or `'value'`, or unquoted) from inside a tag, case-insensitive, unescaped.
fn attribute(tag: &str, key: &str) -> Option<String> {
  let lower = tag.to_ascii_lowercase();
  let mut from = 0;
  while let Some(found) = lower[from..].find(key) {
    let at = from + found;
    from = at + key.len();
    let before_ok = at == 0 || lower.as_bytes()[at - 1].is_ascii_whitespace();
    let rest = tag[at + key.len()..].trim_start();
    if !before_ok || !rest.starts_with('=') {
      continue;
    }
    let value = rest[1..].trim_start();
    let raw = match value.chars().next() {
      Some(q @ ('"' | '\'')) => value[1..].split(q).next().unwrap_or(""),
      _ => value.split(|c: char| c.is_whitespace() || c == '>').next().unwrap_or(""),
    };
    return Some(unescape(raw));
  }
  None
}

pub(super) fn escape(s: &str) -> String {
  s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// The character references bookmark files use.
fn unescape(s: &str) -> String {
  if !s.contains('&') {
    return s.to_string();
  }
  let mut out = String::with_capacity(s.len());
  let mut rest = s;
  while let Some(at) = rest.find('&') {
    out.push_str(&rest[..at]);
    rest = &rest[at..];
    let decoded = rest.find(';').filter(|&e| e <= 10).and_then(|end| {
      let name = &rest[1..end];
      let c = match name {
        "amp" => '&',
        "lt" => '<',
        "gt" => '>',
        "quot" => '"',
        "apos" => '\'',
        "nbsp" => ' ',
        _ => name
          .strip_prefix("#x")
          .or_else(|| name.strip_prefix("#X"))
          .map(|h| u32::from_str_radix(h, 16).ok())
          .unwrap_or_else(|| name.strip_prefix('#').and_then(|d| d.parse().ok()))
          .and_then(char::from_u32)?,
      };
      Some((c, end + 1))
    });
    match decoded {
      Some((c, len)) => {
        out.push(c);
        rest = &rest[len..];
      }
      None => {
        out.push('&');
        rest = &rest[1..];
      }
    }
  }
  out.push_str(rest);
  out
}
