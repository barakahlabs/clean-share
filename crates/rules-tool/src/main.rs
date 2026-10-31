//! Tooling for the tracking rules in `rules/`.
//!
//!   cargo run -- check            validate rules, run their tests, check generated files are current
//!   cargo run -- generate         rewrite rules/schema/*.json and the tables in rules/README.md
//!   cargo run -- bundle <file>    write the combined rules.json for hosting

use std::{
  fs,
  path::{Path, PathBuf},
  process::ExitCode,
};

use link_rules::{ParamRule, RuleSet};

const README_START: &str = "<!-- BEGIN GENERATED: edit the TOML files, then run `cargo run -- generate` in crates/rules-tool -->";
const README_END: &str = "<!-- END GENERATED -->";

fn main() -> ExitCode {
  let args: Vec<String> = std::env::args().skip(1).collect();
  let rules_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../rules");

  let result = match args.iter().map(String::as_str).collect::<Vec<_>>().as_slice() {
    ["check"] => check(&rules_dir),
    ["generate"] => generate(&rules_dir),
    ["bundle", out] => bundle(&rules_dir, Path::new(out)),
    _ => Err(vec!["usage: rules-tool check | generate | bundle <out.json>".into()]),
  };

  match result {
    Ok(message) => {
      println!("{message}");
      ExitCode::SUCCESS
    }
    Err(errors) => {
      for error in errors {
        eprintln!("error: {error}");
      }
      ExitCode::FAILURE
    }
  }
}

/// Loads and validates the rules, then runs every `[[test]]`.
fn load_and_test(rules_dir: &Path) -> Result<RuleSet, Vec<String>> {
  let rules = link_rules::load_dir(rules_dir)?;
  let failures = link_cleaner::check_rule_tests(&rules);
  if failures.is_empty() { Ok(rules) } else { Err(failures) }
}

fn check(rules_dir: &Path) -> Result<String, Vec<String>> {
  let rules = load_and_test(rules_dir)?;
  let mut stale = Vec::new();

  for (path, expected) in generated_files(rules_dir, &rules)? {
    if fs::read_to_string(&path).ok().as_deref() != Some(expected.as_str()) {
      stale.push(format!("{} is out of date: run `rules-tool generate`", path.display()));
    }
  }
  if !stale.is_empty() {
    return Err(stale);
  }

  let tests = rules.global.tests.len() + rules.providers.iter().map(|p| p.tests.len()).sum::<usize>();
  Ok(format!(
    "ok: revision {}, {} providers, {tests} tests passed",
    rules.revision,
    rules.providers.len()
  ))
}

fn generate(rules_dir: &Path) -> Result<String, Vec<String>> {
  let rules = load_and_test(rules_dir)?;
  let files = generated_files(rules_dir, &rules)?;
  for (path, contents) in &files {
    if let Some(parent) = path.parent() {
      fs::create_dir_all(parent).map_err(|e| vec![format!("{}: {e}", parent.display())])?;
    }
    fs::write(path, contents).map_err(|e| vec![format!("{}: {e}", path.display())])?;
  }
  Ok(format!("wrote {} files", files.len()))
}

fn bundle(rules_dir: &Path, out: &Path) -> Result<String, Vec<String>> {
  let rules = load_and_test(rules_dir)?;
  let json = serde_json::to_string(&rules).map_err(|e| vec![e.to_string()])?;
  fs::write(out, json).map_err(|e| vec![format!("{}: {e}", out.display())])?;
  Ok(format!("wrote {} (revision {})", out.display(), rules.revision))
}

/// Every file `generate` writes, with its expected contents.
fn generated_files(rules_dir: &Path, rules: &RuleSet) -> Result<Vec<(PathBuf, String)>, Vec<String>> {
  let mut files: Vec<(PathBuf, String)> = link_rules::file_schemas()
    .into_iter()
    .map(|(name, schema)| {
      let path = rules_dir.join("schema").join(format!("{name}.json"));
      (path, serde_json::to_string_pretty(&schema).unwrap() + "\n")
    })
    .collect();

  let readme_path = rules_dir.join("README.md");
  let readme = fs::read_to_string(&readme_path).map_err(|e| vec![format!("{}: {e}", readme_path.display())])?;
  let (Some(start), Some(end)) = (readme.find(README_START), readme.find(README_END)) else {
    return Err(vec![format!("{}: missing generated-section markers", readme_path.display())]);
  };
  let updated = format!(
    "{}{README_START}\n\n{}\n{}",
    &readme[..start],
    readme_tables(rules),
    &readme[end..]
  );
  files.push((readme_path, updated));
  Ok(files)
}

fn readme_tables(rules: &RuleSet) -> String {
  let mut md = String::new();
  let code_list = |items: Vec<String>| {
    if items.is_empty() {
      "–".to_string()
    } else {
      items.iter().map(|i| format!("`{i}`")).collect::<Vec<_>>().join(", ")
    }
  };
  let params = |rules: &[ParamRule]| code_list(rules.iter().map(|r| r.param.to_string()).collect());

  md.push_str(&format!("Rules revision `{}`.\n\n", rules.revision));

  md.push_str("### Overview\n\n| Platform | Domains | Removed | Always kept | Not applied on |\n|---|---|---|---|---|\n");
  for p in &rules.providers {
    md.push_str(&format!(
      "| [{}](providers/{}.toml) | {} | {} | {} | {} |\n",
      p.name,
      p.id,
      code_list(p.domains.iter().map(ToString::to_string).collect()),
      if p.strip_unknown {
        let listed = params(&p.remove);
        if p.remove.is_empty() {
          "everything not kept".to_string()
        } else {
          format!("everything not kept, incl. {listed}")
        }
      } else {
        params(&p.remove)
      },
      code_list(p.keep.iter().map(ToString::to_string).collect()),
      code_list(p.except.iter().map(ToString::to_string).collect()),
    ));
  }

  md.push_str("\n### Removed on every site ([global.toml](global.toml))\n\n");
  md.push_str(&param_table(&rules.global.remove));

  for p in &rules.providers {
    md.push_str(&format!("\n### {} ([providers/{}.toml](providers/{}.toml))\n\n", p.name, p.id, p.id));
    if p.strip_unknown {
      md.push_str(&format!(
        "Removes **every** parameter except {} (`strip_unknown = true`), so new tracking names are covered automatically.",
        if p.keep.is_empty() {
          "none".to_string()
        } else {
          p.keep.iter().map(|k| format!("`{k}`")).collect::<Vec<_>>().join(", ")
        }
      ));
      md.push_str(if p.remove.is_empty() { "\n" } else { " Known ones:\n\n" });
    }
    if p.remove.is_empty() {
      if !p.strip_unknown {
        md.push_str("No site-specific parameters; the every-site rules apply.\n");
      }
    } else {
      md.push_str(&param_table(&p.remove));
    }
  }

  md.push_str("\n### Examples (from the rule tests)\n\n| Platform | Shared link | Cleaned link |\n|---|---|---|\n");
  let examples = rules
    .providers
    .iter()
    .flat_map(|p| p.tests.iter().map(move |t| (p.name.as_str(), t)))
    .chain(rules.global.tests.iter().map(|t| ("Any site", t)));
  for (name, test) in examples {
    md.push_str(&format!("| {name} | `{}` | `{}` |\n", test.input, test.output));
  }
  md
}

fn param_table(rules: &[ParamRule]) -> String {
  let mut md = String::from("| Parameter | Why it's removed |\n|---|---|\n");
  for rule in rules {
    md.push_str(&format!("| `{}` | {} |\n", rule.param, rule.reason.as_deref().unwrap_or("")));
  }
  md
}
