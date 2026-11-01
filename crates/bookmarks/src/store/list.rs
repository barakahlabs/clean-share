//! Listing, filtering by tags, and search.

use turso::Value;

use super::{Store, row, tags};
use crate::{Bookmark, Error, ListQuery, Page, Result, ids};

/// Columns in the full-text index, in index order. `fts_match` and `fts_score` take the same list.
const SEARCH_COLUMNS: &str = "b.title, b.note, b.shared_text, b.description, b.tags_text";

/// Where the next page starts. Newest-first lists page by position in time; search results
/// (ordered by relevance) page by offset.
enum Cursor {
  Before { created_at: i64, id: String },
  Offset(i64),
}

impl Cursor {
  fn encode(&self) -> String {
    match self {
      Cursor::Before { created_at, id } => format!("t:{created_at}:{id}"),
      Cursor::Offset(n) => format!("o:{n}"),
    }
  }

  fn decode(text: &str) -> Result<Self> {
    let bad = || Error::InvalidCursor;
    match text.split_once(':').ok_or_else(bad)? {
      ("t", rest) => {
        let (created_at, id) = rest.split_once(':').ok_or_else(bad)?;
        Ok(Cursor::Before {
          created_at: created_at.parse().map_err(|_| bad())?,
          id: id.to_string(),
        })
      }
      ("o", n) => Ok(Cursor::Offset(n.parse().map_err(|_| bad())?)),
      _ => Err(bad()),
    }
  }
}

impl Store {
  /// Bookmarks matching the query: newest first, or most relevant first when searching.
  pub async fn list(&self, query: &ListQuery) -> Result<Page<Bookmark>> {
    let limit = i64::from(query.limit.clamp(1, 200));
    let search = query.search.as_deref().and_then(search_expression);
    let cursor = query.cursor.as_deref().map(Cursor::decode).transpose()?;

    let mut filters = Vec::new();
    let mut params: Vec<Value> = Vec::new();

    let tag_ids: Vec<String> =
      query.tags.iter().filter_map(|t| tags::normalize(t)).map(|t| ids::tag_id(&t)).collect();
    if !tag_ids.is_empty() {
      let placeholders = vec!["?"; tag_ids.len()].join(", ");
      filters.push(format!(
        "b.id IN (SELECT bookmark_id FROM bookmark_tag WHERE tag_id IN ({placeholders})
                  GROUP BY bookmark_id HAVING count(DISTINCT tag_id) = {})",
        tag_ids.len()
      ));
      params.extend(tag_ids.iter().map(|id| Value::from(id.as_str())));
    }

    // `score_for`: the search expression to rank by, if searching.
    let (score_for, offset) = match &search {
      Some(expr) => {
        filters.push(format!("fts_match({SEARCH_COLUMNS}, {})", sql_literal(expr)));
        let offset = match cursor {
          None => 0,
          Some(Cursor::Offset(n)) => n,
          Some(Cursor::Before { .. }) => return Err(Error::InvalidCursor),
        };
        (Some(expr), offset)
      }
      None => {
        match cursor {
          None => {}
          Some(Cursor::Before { created_at, id }) => {
            filters.push("(b.created_at < ? OR (b.created_at = ? AND b.id < ?))".to_string());
            params.extend([Value::from(created_at), Value::from(created_at), Value::from(id)]);
          }
          Some(Cursor::Offset(_)) => return Err(Error::InvalidCursor),
        }
        (None, 0)
      }
    };

    let columns =
      row::BOOKMARK_COLUMNS.split(", ").map(|c| format!("b.{c}")).collect::<Vec<_>>().join(", ");
    let filter =
      if filters.is_empty() { String::new() } else { format!("WHERE {}", filters.join(" AND ")) };
    // One extra row tells whether there is a next page.
    let page = format!("LIMIT {} OFFSET {offset}", limit + 1);
    let sql = match score_for {
      // Two turso 0.8.3 bugs shape this query (`search_puts_the_best_match_first` checks it):
      // - `ORDER BY fts_score(…) DESC, created_at DESC` next to `fts_match` sorts wrongly, so the
      //   ranking happens in an outer query;
      // - in that form `fts_score` returns 0 for a `?` parameter, so the search expression is
      //   written into the SQL (safe: `search_expression` keeps only letters and digits).
      Some(expr) => format!(
        "SELECT {} FROM (SELECT {columns}, fts_score({SEARCH_COLUMNS}, {}) AS score
                        FROM bookmark b {filter})
         ORDER BY score DESC, created_at DESC, id DESC {page}",
        row::BOOKMARK_COLUMNS,
        sql_literal(expr)
      ),
      None => {
        format!(
          "SELECT {columns} FROM bookmark b {filter} ORDER BY b.created_at DESC, b.id DESC {page}"
        )
      }
    };

    let mut rows = self.conn.query(&sql, params).await?;
    let mut items = Vec::new();
    while let Some(r) = rows.next().await? {
      items.push(row::bookmark(&r)?);
    }
    drop(rows);

    let more = items.len() as i64 > limit;
    items.truncate(limit as usize);
    let next = more.then(|| match &search {
      Some(_) => Cursor::Offset(offset + limit),
      None => {
        let last = items.last().expect("more than `limit` rows");
        Cursor::Before { created_at: last.created_at, id: last.id.clone() }
      }
    });

    let ids: Vec<String> = items.iter().map(|b| b.id.clone()).collect();
    let mut tags = tags::of(&self.conn, &ids).await?;
    for item in &mut items {
      item.tags = tags.remove(&item.id).unwrap_or_default();
    }
    Ok(Page { items, next: next.map(|c| c.encode()) })
  }
}

/// `text` as a SQL string literal.
fn sql_literal(text: &str) -> String {
  format!("'{}'", text.replace('\'', "''"))
}

/// User input → a search expression where every word must match, e.g. `rust async!` →
/// `"rust" AND "async"`. Words are quoted so query syntax in the input (`:`, `*`, `NOT`) is
/// treated as text. Words shorter than two characters are skipped (ngrams start at two).
/// `None` if no word is left, which means "don't filter".
fn search_expression(input: &str) -> Option<String> {
  let words: Vec<String> = input
    .split(|c: char| !c.is_alphanumeric())
    .filter(|w| w.chars().count() >= 2)
    .map(|w| format!("\"{}\"", w.to_lowercase()))
    .collect();
  (!words.is_empty()).then(|| words.join(" AND "))
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn search_expressions_contain_only_quoted_words() {
    assert_eq!(search_expression("Rust, async!").as_deref(), Some(r#""rust" AND "async""#));
    assert_eq!(
      search_expression("it's x'; DROP TABLE--").as_deref(),
      Some(r#""it" AND "drop" AND "table""#)
    );
    assert_eq!(search_expression("a ' \" *"), None);
    assert_eq!(search_expression("café 東京").as_deref(), Some(r#""café" AND "東京""#));
    assert_eq!(sql_literal("it's"), "'it''s'");
  }
}
