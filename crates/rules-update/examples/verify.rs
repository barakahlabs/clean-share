//! Checks a signed bundle the way the app will: `cargo run -p rules-update --example verify --
//! rules.json rules.json.sig`. CI runs it before publishing, so a bundle the app would refuse (a
//! key mismatch, failing tests) never goes out.

fn main() {
  let args: Vec<String> = std::env::args().skip(1).collect();
  let [bundle, signature] = args.as_slice() else {
    eprintln!("usage: verify <rules.json> <rules.json.sig>");
    std::process::exit(2);
  };
  let read = |path: &str| std::fs::read(path).unwrap_or_else(|e| panic!("{path}: {e}"));
  match rules_update::verify(&read(bundle), &read(signature), &rules_update::PUBLIC_KEY) {
    Ok(rules) => println!("ok: revision {}, {} providers, signed with the app's key", rules.revision, rules.providers.len()),
    Err(e) => {
      eprintln!("refused: {e}");
      std::process::exit(1);
    }
  }
}
