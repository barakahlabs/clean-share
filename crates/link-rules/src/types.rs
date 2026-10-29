//! String types that check their own format when a rule file is read,
//! so an invalid value can't get into a loaded [`RuleSet`](crate::RuleSet).

use std::{borrow::Cow, fmt};

use schemars::{json_schema, JsonSchema, Schema, SchemaGenerator};
use serde::{Deserialize, Serialize};

/// Defines a `String` newtype that is validated on deserialize and documented
/// in the JSON Schema with a regex `pattern`.
macro_rules! checked_string {
  ($(#[$doc:meta])* $name:ident, pattern = $pattern:literal, example = $example:literal, check = $check:expr) => {
    $(#[$doc])*
    #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
    #[serde(try_from = "String", into = "String")]
    pub struct $name(String);

    impl $name {
      pub fn as_str(&self) -> &str {
        &self.0
      }
    }

    impl TryFrom<String> for $name {
      type Error = String;

      fn try_from(value: String) -> Result<Self, String> {
        let check: fn(&str) -> Result<String, String> = $check;
        check(&value).map(Self)
      }
    }

    impl From<$name> for String {
      fn from(value: $name) -> String {
        value.0
      }
    }

    impl fmt::Display for $name {
      fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
      }
    }

    impl JsonSchema for $name {
      fn schema_name() -> Cow<'static, str> {
        stringify!($name).into()
      }

      fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({
          "type": "string",
          "pattern": $pattern,
          "examples": [$example],
        })
      }
    }
  };
}

checked_string!(
  /// Provider ID: lowercase letters, digits and `-`. Must match the file name (`x` → `x.toml`).
  ProviderId,
  pattern = "^[a-z0-9]+(-[a-z0-9]+)*$",
  example = "youtube",
  check = |s| {
    let ok = !s.is_empty()
      && !s.starts_with('-')
      && !s.ends_with('-')
      && !s.contains("--")
      && s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    ok.then(|| s.to_string())
      .ok_or_else(|| format!("invalid provider id `{s}`: use lowercase letters, digits and `-`"))
  }
);

checked_string!(
  /// Domain without scheme or path, e.g. `x.com`. Subdomains are matched too.
  Domain,
  pattern = "^([a-z0-9]([a-z0-9-]*[a-z0-9])?\\.)+[a-z]{2,}$",
  example = "youtube.com",
  check = |s| {
    let labels_ok = s.split('.').all(|label| {
      !label.is_empty()
        && !label.starts_with('-')
        && !label.ends_with('-')
        && label.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    });
    (labels_ok && s.contains('.'))
      .then(|| s.to_string())
      .ok_or_else(|| {
        format!("invalid domain `{s}`: write it lowercase without scheme or path, e.g. `x.com`")
      })
  }
);

checked_string!(
  /// Query parameter name, matched ignoring case. A trailing `*` matches by prefix (`utm_*`).
  ParamPattern,
  pattern = "^[A-Za-z0-9_.\\-\\[\\]]+\\*?$",
  example = "utm_*",
  check = |s| {
    let name = s.strip_suffix('*').unwrap_or(s);
    let ok = !name.is_empty()
      && name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-' | '[' | ']'));
    ok.then(|| s.to_string()).ok_or_else(|| {
      format!("invalid parameter `{s}`: use letters, digits, `_ . - [ ]`, optionally ending in `*`")
    })
  }
);

checked_string!(
  /// URL path prefix starting with `/`, e.g. `/i/redirect`.
  PathPrefix,
  pattern = "^/\\S*$",
  example = "/i/redirect",
  check = |s| {
    (s.starts_with('/') && !s.chars().any(char::is_whitespace))
      .then(|| s.to_string())
      .ok_or_else(|| format!("invalid path `{s}`: must start with `/` and contain no spaces"))
  }
);

impl ParamPattern {
  /// True if `name` (any case) matches this pattern.
  pub fn matches(&self, name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    let pattern = self.0.to_ascii_lowercase();
    match pattern.strip_suffix('*') {
      Some(prefix) => name.starts_with(prefix),
      None => name == pattern,
    }
  }
}

impl Domain {
  /// True if `host` (lowercase) is this domain or one of its subdomains.
  pub fn matches(&self, host: &str) -> bool {
    host == self.0
      || host
        .strip_suffix(self.0.as_str())
        .is_some_and(|rest| rest.ends_with('.'))
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn parse<T: TryFrom<String, Error = String>>(s: &str) -> Result<T, String> {
    T::try_from(s.to_string())
  }

  #[test]
  fn rejects_malformed_values() {
    assert!(parse::<Domain>("https://x.com").is_err());
    assert!(parse::<Domain>("X.com").is_err());
    assert!(parse::<Domain>("localhost").is_err());
    assert!(parse::<ParamPattern>("utm source").is_err());
    assert!(parse::<ParamPattern>("*").is_err());
    assert!(parse::<PathPrefix>("i/redirect").is_err());
    assert!(parse::<ProviderId>("YouTube").is_err());
  }

  #[test]
  fn params_match_ignoring_case_and_by_prefix() {
    let utm = parse::<ParamPattern>("utm_*").unwrap();
    assert!(utm.matches("UTM_Source"));
    assert!(!utm.matches("xutm_source"));
    assert!(parse::<ParamPattern>("trackingId").unwrap().matches("trackingid"));
  }

  #[test]
  fn domains_match_subdomains_not_lookalikes() {
    let x = parse::<Domain>("x.com").unwrap();
    assert!(x.matches("x.com") && x.matches("mobile.x.com"));
    assert!(!x.matches("notx.com"));
  }
}
