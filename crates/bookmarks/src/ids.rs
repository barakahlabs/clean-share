//! Content-derived IDs (UUIDv5). The same link or tag name gets the same ID on every device,
//! so offline saves on two phones produce one row instead of two after sync.

use uuid::Uuid;

/// Fixed namespace for Clean Share IDs. Never change it: existing IDs depend on it.
const NAMESPACE: Uuid = Uuid::from_u128(0x6c1e_a75b_4d0c_4f3e_9a8e_2b1c_5f7d_0e41);

pub fn bookmark_id(cleaned_url: &str) -> String {
  Uuid::new_v5(&NAMESPACE, format!("bookmark:{cleaned_url}").as_bytes()).to_string()
}

/// Tag names are case-insensitive, so `Rust` and `rust` share an ID.
pub fn tag_id(name: &str) -> String {
  Uuid::new_v5(&NAMESPACE, format!("tag:{}", name.to_lowercase()).as_bytes()).to_string()
}
