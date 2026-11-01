use link_cleaner::{Link, RuleSet};

/// Finds and cleans the links in shared text. The store cleans every link through this before
/// storing it, so no caller can save a link with tracking.
pub trait Cleaner: Send + Sync {
  fn links(&self, text: &str) -> Vec<Link>;
  /// Revision of the rules in use, stored with each bookmark so it can be re-cleaned later.
  fn revision(&self) -> &str;
}

/// The rules embedded in the app at build time.
pub struct BundledRules;

impl Cleaner for BundledRules {
  fn links(&self, text: &str) -> Vec<Link> {
    link_cleaner::links(text)
  }

  fn revision(&self) -> &str {
    &link_cleaner::bundled_rules().revision
  }
}

/// Any other rule set, e.g. a newer one downloaded at runtime.
pub struct Rules(pub RuleSet);

impl Cleaner for Rules {
  fn links(&self, text: &str) -> Vec<Link> {
    link_cleaner::links_with(&self.0, text)
  }

  fn revision(&self) -> &str {
    &self.0.revision
  }
}
