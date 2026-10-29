use serde::{Deserialize, Serialize};

/// Content another app shared with us. `None` when nothing is pending.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SharedContent {
  pub text: Option<String>,
}
