//! Loads and validates the TOML rules in `rules/` at compile time and embeds
//! them as JSON. A malformed or conflicting rule fails the build.

use std::{env, fs, path::PathBuf};

fn main() {
  if env::var_os("CARGO_FEATURE_BUNDLED").is_none() {
    return;
  }
  let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
  let rules_dir = manifest_dir.join("../../rules");
  println!("cargo:rerun-if-changed={}", rules_dir.display());

  let rules = link_rules::load_dir(&rules_dir).unwrap_or_else(|errors| {
    for error in &errors {
      println!("cargo:warning={error}");
    }
    panic!("invalid rules in {}: {} problem(s), see warnings above", rules_dir.display(), errors.len());
  });

  let out = PathBuf::from(env::var("OUT_DIR").unwrap()).join("rules.json");
  fs::write(out, serde_json::to_string(&rules).unwrap()).unwrap();
}
