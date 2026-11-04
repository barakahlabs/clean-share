//! Hosted rules: the tracking rules published by CI on every rules change (`.github/workflows/
//! rules.yml`), so apps get new rules without an app release.
//!
//! - [`BUNDLE_URL`] is the latest `rules.json` for this schema version ([`link_rules::SCHEMA_VERSION`]
//!   in the path), and [`SIGNATURE_URL`] its ed25519 signature: 64 raw bytes over the file's bytes.
//! - [`verify`] accepts a download only if the signature matches [`PUBLIC_KEY`], the bundle is a
//!   valid rule set this app understands, and every rule's own tests pass with it.
//! - [`is_newer`] says whether it should replace the rules in use.
//!
//! The app does the downloading and caching; this crate does no I/O, so it's all testable.

use link_rules::RuleSet;
use ring::signature::{ED25519, UnparsedPublicKey};

/// Where CI publishes the rules (GitHub Pages). `v1` is the schema version: a breaking format
/// change publishes `v2` next to it, and apps keep reading the version they understand.
pub const BUNDLE_URL: &str = "https://barakahlabs.github.io/clean-share/rules/v1/rules.json";
pub const SIGNATURE_URL: &str = "https://barakahlabs.github.io/clean-share/rules/v1/rules.json.sig";

/// The ed25519 public key whose private half (GitHub secret `RULES_SIGNING_KEY`, local copy in
/// `.secrets/rules-signing-key.pem`) signs the bundle. As SPKI PEM:
/// `MCowBQYDK2VwAyEACdmtAIZU3RxClte2jwXu1306qWhhjUUJYBluaqBgRuk=`.
pub const PUBLIC_KEY: [u8; 32] = [
  0x09, 0xd9, 0xad, 0x00, 0x86, 0x54, 0xdd, 0x1c, 0x42, 0x96, 0xd7, 0xb6, 0x8f, 0x05, 0xee, 0xd7, 0x7d,
  0x3a, 0xa9, 0x68, 0x61, 0x8d, 0x45, 0x09, 0x60, 0x19, 0x6e, 0x6a, 0xa0, 0x60, 0x46, 0xe9,
];

/// The most a bundle may be (today's is a few KB).
pub const MAX_BUNDLE_BYTES: usize = 1024 * 1024;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum Error {
  #[error("the rules' signature doesn't match")]
  BadSignature,
  #[error("the rules can't be used: {}", .0.join("; "))]
  Invalid(Vec<String>),
  #[error("the rules fail their own tests: {}", .0.join("; "))]
  FailingTests(Vec<String>),
}

/// Checks a downloaded bundle and its signature against `public_key` ([`PUBLIC_KEY`] in the app).
/// Returns the rules only if the signature matches, the bundle parses and validates for this
/// app's schema version (fields it doesn't know are ignored), and every rule test passes.
pub fn verify(bundle: &[u8], signature: &[u8], public_key: &[u8; 32]) -> Result<RuleSet, Error> {
  UnparsedPublicKey::new(&ED25519, public_key).verify(bundle, signature).map_err(|_| Error::BadSignature)?;
  let rules = link_rules::parse_bundle(bundle).map_err(Error::Invalid)?;
  let failures = link_cleaner::check_rule_tests(&rules);
  if !failures.is_empty() {
    return Err(Error::FailingTests(failures));
  }
  Ok(rules)
}

/// True if `candidate` should replace the rules in use (revision `current`).
pub fn is_newer(candidate: &RuleSet, current: &str) -> bool {
  link_rules::is_newer(&candidate.revision, current)
}

#[cfg(test)]
mod tests {
  use ring::{
    rand::SystemRandom,
    signature::{Ed25519KeyPair, KeyPair},
  };

  use super::*;

  struct Signer {
    pair: Ed25519KeyPair,
  }

  impl Signer {
    fn new() -> Self {
      let pkcs8 = Ed25519KeyPair::generate_pkcs8(&SystemRandom::new()).unwrap();
      Self { pair: Ed25519KeyPair::from_pkcs8(pkcs8.as_ref()).unwrap() }
    }

    fn public_key(&self) -> [u8; 32] {
      self.pair.public_key().as_ref().try_into().unwrap()
    }

    fn sign(&self, bytes: &[u8]) -> Vec<u8> {
      self.pair.sign(bytes).as_ref().to_vec()
    }
  }

  fn bundle(revision: &str) -> Vec<u8> {
    let mut rules = link_cleaner::bundled_rules().clone();
    rules.revision = revision.into();
    serde_json::to_vec(&rules).unwrap()
  }

  #[test]
  fn a_signed_valid_bundle_is_accepted() {
    let signer = Signer::new();
    let bytes = bundle("2099-01-01.1");
    let rules = verify(&bytes, &signer.sign(&bytes), &signer.public_key()).unwrap();
    assert_eq!(rules.revision, "2099-01-01.1");
    assert!(is_newer(&rules, &link_cleaner::bundled_rules().revision));
  }

  #[test]
  fn a_changed_bundle_or_another_key_is_refused() {
    let signer = Signer::new();
    let bytes = bundle("2099-01-01.1");
    let signature = signer.sign(&bytes);
    let mut tampered = bytes.clone();
    tampered[10] ^= 1;
    assert_eq!(verify(&tampered, &signature, &signer.public_key()).unwrap_err(), Error::BadSignature);
    assert_eq!(verify(&bytes, &signature, &Signer::new().public_key()).unwrap_err(), Error::BadSignature);
    assert_eq!(verify(&bytes, b"not a signature", &signer.public_key()).unwrap_err(), Error::BadSignature);
  }

  #[test]
  fn signed_rules_that_fail_their_tests_are_refused() {
    let signer = Signer::new();
    let mut rules = link_cleaner::bundled_rules().clone();
    // A rule test that the rules don't satisfy.
    rules.global.tests[0].output = "https://example.com/something-else".into();
    let bytes = serde_json::to_vec(&rules).unwrap();
    assert!(matches!(verify(&bytes, &signer.sign(&bytes), &signer.public_key()), Err(Error::FailingTests(_))));
  }

  #[test]
  fn signed_rules_for_another_schema_are_refused() {
    let signer = Signer::new();
    let mut rules = link_cleaner::bundled_rules().clone();
    rules.schema += 1;
    let bytes = serde_json::to_vec(&rules).unwrap();
    assert!(matches!(verify(&bytes, &signer.sign(&bytes), &signer.public_key()), Err(Error::Invalid(_))));
  }

  #[test]
  fn the_real_public_key_is_a_valid_ed25519_key() {
    // A signature from another key must fail cleanly (not panic) against the real key.
    let signer = Signer::new();
    let bytes = bundle("2099-01-01.1");
    assert_eq!(verify(&bytes, &signer.sign(&bytes), &PUBLIC_KEY).unwrap_err(), Error::BadSignature);
  }
}
