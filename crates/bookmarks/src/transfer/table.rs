//! CSV exports. Columns are found by name, so their order doesn't matter:
//! - Pocket: `title,url,time_added,tags,status`, tags separated by `|`, time in seconds.
//! - Raindrop: `id,title,note,excerpt,url,folder,tags,created,…`, tags separated by `,`, the
//!   folder used as a tag, ISO dates.
//! - Anything else with a `url` (or `link`, `href`) column, plus optional title, tags, note.

use super::{Format, ImportItem, ParseError, date, split_tags, text};
use crate::TagInput;

/// Raindrop's default collection: not a tag.
const NO_FOLDER: &[&str] = &["unsorted", ""];

pub fn parse(csv_text: &str) -> Result<(Format, Vec<ImportItem>), ParseError> {
  let mut reader = csv::ReaderBuilder::new().flexible(true).from_reader(csv_text.as_bytes());
  let headers: Vec<String> = reader
    .headers()
    .map_err(|_| ParseError::UnknownFormat)?
    .iter()
    .map(|h| h.trim().to_ascii_lowercase())
    .collect();
  let column = |names: &[&str]| headers.iter().position(|h| names.contains(&h.as_str()));
  let Some(url_col) = column(&["url", "link", "href"]) else {
    return Err(ParseError::UnknownFormat);
  };
  let has = |name: &str| headers.iter().any(|h| h == name);
  let format = if has("time_added") && has("status") {
    Format::Pocket
  } else if has("excerpt") && has("folder") {
    Format::Raindrop
  } else {
    Format::Csv
  };
  let title_col = column(&["title", "name"]);
  let tags_col = column(&["tags", "tag", "labels"]);
  let note_col = column(&["note", "notes", "description", "comment"]);
  let folder_col = column(&["folder", "collection"]);
  let date_col = column(&["time_added", "created", "created_at", "date", "added"]);
  // Pocket separates tags with "|"; Raindrop and others with ",".
  let separators: &[char] = if format == Format::Pocket { &['|'] } else { &[',', '|'] };

  let mut items = Vec::new();
  for record in reader.records() {
    let record = record.map_err(|e| ParseError::Invalid("CSV", e.to_string()))?;
    let field = |col: Option<usize>| col.and_then(|c| record.get(c)).unwrap_or("");
    let url = field(Some(url_col)).trim();
    if !(url.starts_with("http://") || url.starts_with("https://")) {
      continue;
    }
    let mut tags = split_tags(field(tags_col), separators);
    let folder = field(folder_col).trim();
    if !NO_FOLDER.contains(&folder.to_lowercase().as_str()) {
      // Raindrop writes nested collections as "Parent/Child": the innermost is the tag.
      tags.push(TagInput::user(folder.rsplit('/').next().unwrap_or(folder).trim()));
    }
    items.push(ImportItem {
      url: url.to_string(),
      title: text(field(title_col)).filter(|t| t != url),
      note: text(field(note_col)),
      tags,
      created_at: date::parse(field(date_col)),
    });
  }
  Ok((format, items))
}
