//! Typed schema for Clean Share's tracking rules.
//!
//! Rules are written as TOML files in `rules/`:
//!
//! - `manifest.toml` — schema version and revision ([`Manifest`])
//! - `global.toml` — parameters removed on every site ([`Global`])
//! - `providers/<id>.toml` — one file per service ([`Provider`])
//!
//! [`load_dir`] reads and validates them into one [`RuleSet`], which is what
//! the app embeds and what gets published as the hosted `rules.json` bundle.

mod types;

use std::{collections::HashMap, fs, path::Path};

use schemars::{JsonSchema, Schema};
use serde::{Deserialize, Serialize};

pub use types::{Domain, ParamPattern, PathPrefix, ProviderId};

/// Bump when the format changes in a way older apps can't read or would get wrong if they
/// ignored the new part. Fields that are safe to ignore (an older app just removes a little less)
/// keep the version: every type here accepts unknown fields, so older apps load newer bundles.
/// The TOML files are still strict: [`load_dir`] reports unknown keys as errors (typos).
pub const SCHEMA_VERSION: u32 = 1;

/// `rules/manifest.toml`
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct Manifest {
  /// Format version. Must equal the version this app understands.
  pub schema: u32,
  /// Content version, e.g. `2026-10-30.1`. A newer revision replaces cached rules.
  pub revision: String,
}

/// `rules/global.toml`: parameters removed on every site.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct Global {
  #[serde(default)]
  pub remove: Vec<ParamRule>,
  /// Example links and their expected cleaned version. At least one.
  #[serde(rename = "test")]
  #[schemars(length(min = 1))]
  pub tests: Vec<TestCase>,
}

/// `rules/providers/<id>.toml`: rules for one service.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct Provider {
  /// Must match the file name: `id = "x"` lives in `x.toml`.
  pub id: ProviderId,
  /// Display name, e.g. "X / Twitter".
  pub name: String,
  /// Domains this provider covers. Subdomains are included.
  #[schemars(length(min = 1))]
  pub domains: Vec<Domain>,
  /// Tracking parameters to remove on these domains.
  #[serde(default)]
  pub remove: Vec<ParamRule>,
  /// Parameters that make links work. Never removed, even if a global rule matches.
  #[serde(default)]
  pub keep: Vec<ParamPattern>,
  /// Paths where this provider's rules don't apply (global rules still do).
  #[serde(default)]
  pub except: Vec<PathPrefix>,
  /// Remove every parameter not listed in `keep`. For sites whose links work without query
  /// parameters and that keep inventing new tracking names (e.g. Instagram).
  #[serde(default, skip_serializing_if = "std::ops::Not::not")]
  pub strip_unknown: bool,
  /// Example links and their expected cleaned version. At least one.
  #[serde(rename = "test")]
  #[schemars(length(min = 1))]
  pub tests: Vec<TestCase>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct ParamRule {
  pub param: ParamPattern,
  /// Why this is tracking, in plain words. Shown in docs and can be shown in the app.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct TestCase {
  /// A link as shared by the app.
  #[serde(rename = "in")]
  pub input: String,
  /// The same link after cleaning.
  #[serde(rename = "out")]
  pub output: String,
}

/// All rules combined: the embedded defaults and the hosted `rules.json` bundle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleSet {
  pub schema: u32,
  pub revision: String,
  pub global: Global,
  pub providers: Vec<Provider>,
}

/// JSON Schemas for the rule files, by name: `manifest`, `global`, `provider`.
///
/// Draft-07, because editor tooling (Zed, VS Code, Taplo) doesn't fully support 2020-12 yet.
pub fn file_schemas() -> Vec<(&'static str, Schema)> {
  fn draft07<T: JsonSchema>() -> Schema {
    schemars::generate::SchemaSettings::draft07()
      .into_generator()
      .into_root_schema_for::<T>()
  }
  vec![
    ("manifest", draft07::<Manifest>()),
    ("global", draft07::<Global>()),
    ("provider", draft07::<Provider>()),
  ]
}

/// Reads `manifest.toml`, `global.toml` and `providers/*.toml` from `dir`, then validates them.
/// Returns every problem found, each prefixed with the file it came from.
pub fn load_dir(dir: &Path) -> Result<RuleSet, Vec<String>> {
  let mut errors = Vec::new();

  let manifest: Option<Manifest> = read_toml(&dir.join("manifest.toml"), &mut errors);
  let global: Option<Global> = read_toml(&dir.join("global.toml"), &mut errors);

  let mut providers = Vec::new();
  let providers_dir = dir.join("providers");
  match fs::read_dir(&providers_dir) {
    Ok(entries) => {
      let mut paths: Vec<_> = entries
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|ext| ext == "toml"))
        .collect();
      paths.sort();
      for path in paths {
        let Some(provider) = read_toml::<Provider>(&path, &mut errors) else { continue };
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or_default();
        if stem != provider.id.as_str() {
          errors.push(format!(
            "{}: id `{}` must match the file name (`{}.toml`)",
            path.display(),
            provider.id,
            provider.id
          ));
        }
        providers.push(provider);
      }
    }
    Err(e) => errors.push(format!("{}: {e}", providers_dir.display())),
  }

  let (Some(manifest), Some(global)) = (manifest, global) else {
    return Err(errors);
  };
  let rules = RuleSet {
    schema: manifest.schema,
    revision: manifest.revision,
    global,
    providers,
  };
  if let Err(problems) = validate(&rules) {
    errors.extend(problems);
  }
  if errors.is_empty() { Ok(rules) } else { Err(errors) }
}

/// Checks the rules beyond what the types enforce. Run on load and on any downloaded bundle.
pub fn validate(rules: &RuleSet) -> Result<(), Vec<String>> {
  let mut errors = Vec::new();

  if rules.schema != SCHEMA_VERSION {
    errors.push(format!(
      "schema version {} is not supported (expected {SCHEMA_VERSION})",
      rules.schema
    ));
  }
  if rules.revision.trim().is_empty() {
    errors.push("revision must not be empty".into());
  }
  if rules.global.tests.is_empty() {
    errors.push("global: add at least one [[test]]".into());
  }

  let mut ids = HashMap::new();
  let mut domain_owners: HashMap<&str, &str> = HashMap::new();

  for provider in &rules.providers {
    let id = provider.id.as_str();
    if ids.insert(id, ()).is_some() {
      errors.push(format!("{id}: duplicate provider id"));
    }
    if provider.domains.is_empty() {
      errors.push(format!("{id}: add at least one domain"));
    }
    if provider.tests.is_empty() {
      errors.push(format!("{id}: add at least one [[test]]"));
    }
    for domain in &provider.domains {
      if let Some(owner) = domain_owners.insert(domain.as_str(), id) {
        errors.push(format!("{id}: domain `{domain}` is already covered by `{owner}`"));
      }
    }
    for kept in &provider.keep {
      let literal = kept.as_str().trim_end_matches('*');
      if let Some(rule) = provider.remove.iter().find(|r| r.param.matches(literal)) {
        errors.push(format!(
          "{id}: `{kept}` is in both `keep` and `remove` (matched by `{}`)",
          rule.param
        ));
      }
    }
  }

  if errors.is_empty() { Ok(()) } else { Err(errors) }
}

/// Reads a rules bundle (the hosted `rules.json`, or the one embedded at build time) and validates
/// it. Unknown fields are ignored: they come from newer rules that this app doesn't know yet.
pub fn parse_bundle(json: &[u8]) -> Result<RuleSet, Vec<String>> {
  let rules: RuleSet = serde_json::from_slice(json).map_err(|e| vec![format!("rules.json: {e}")])?;
  validate(&rules)?;
  Ok(rules)
}

/// True if revision `a` is newer than `b`. Revisions are dates with a counter (`2026-10-31.2`),
/// compared number by number, so `…31.10` is newer than `…31.2`.
pub fn is_newer(a: &str, b: &str) -> bool {
  fn parts(revision: &str) -> Vec<u64> {
    revision.split(|c: char| !c.is_ascii_digit()).filter(|p| !p.is_empty()).map(|p| p.parse().unwrap_or(0)).collect()
  }
  parts(a) > parts(b)
}

fn read_toml<T: serde::de::DeserializeOwned>(path: &Path, errors: &mut Vec<String>) -> Option<T> {
  let text = match fs::read_to_string(path) {
    Ok(text) => text,
    Err(e) => {
      errors.push(format!("{}: {e}", path.display()));
      return None;
    }
  };
  let deserializer = match toml::Deserializer::parse(&text) {
    Ok(d) => d,
    Err(e) => {
      errors.push(format!("{}: {e}", path.display()));
      return None;
    }
  };
  // The types accept unknown keys (for bundles from newer rules); in a rules file they're typos.
  let mut unknown = Vec::new();
  let value = serde_ignored::deserialize(deserializer, |key| unknown.push(key.to_string()));
  for key in &unknown {
    errors.push(format!("{}: unknown key `{key}`", path.display()));
  }
  match value {
    Ok(value) if unknown.is_empty() => Some(value),
    Ok(_) => None,
    Err(e) => {
      errors.push(format!("{}: {e}", path.display()));
      None
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn revisions_compare_number_by_number() {
    assert!(is_newer("2026-10-31.10", "2026-10-31.2"));
    assert!(is_newer("2026-11-01.1", "2026-10-31.9"));
    assert!(is_newer("2026-10-31.2", "2026-10-31"));
    assert!(!is_newer("2026-10-31.2", "2026-10-31.2"));
    assert!(!is_newer("2026-10-30.5", "2026-10-31.1"));
  }

  fn rules_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../rules")
  }

  #[test]
  fn a_bundle_with_fields_from_newer_rules_still_loads() {
    let rules = load_dir(&rules_dir()).unwrap();
    let mut json: serde_json::Value = serde_json::to_value(&rules).unwrap();
    json["future_option"] = serde_json::json!(true);
    json["providers"][0]["unwrap"] = serde_json::json!(["q"]);
    let parsed = parse_bundle(serde_json::to_string(&json).unwrap().as_bytes()).unwrap();
    assert_eq!(parsed.revision, rules.revision);
    assert_eq!(parsed.providers.len(), rules.providers.len());
  }

  #[test]
  fn a_bundle_with_another_schema_version_is_refused() {
    let mut rules = load_dir(&rules_dir()).unwrap();
    rules.schema = SCHEMA_VERSION + 1;
    let errors = parse_bundle(serde_json::to_string(&rules).unwrap().as_bytes()).unwrap_err();
    assert!(errors[0].contains("schema version"), "{errors:?}");
  }

  #[test]
  fn unknown_keys_in_a_rules_file_are_errors() {
    let dir = std::env::temp_dir().join(format!("link-rules-test-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("manifest.toml");
    fs::write(&path, "schema = 1\nrevision = \"2026-01-01.1\"\nrevison_note = \"typo\"\n").unwrap();
    let mut errors = Vec::new();
    let manifest: Option<Manifest> = read_toml(&path, &mut errors);
    fs::remove_dir_all(&dir).ok();
    assert!(manifest.is_none());
    assert_eq!(errors.len(), 1);
    assert!(errors[0].contains("unknown key `revison_note`"), "{errors:?}");
  }
}
