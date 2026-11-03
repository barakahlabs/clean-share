use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShareTextRequest {
  pub text: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReturnToSourceResponse {
  /// Whether the app went back to the app that shared.
  pub returned: bool,
}

/// Content another app shared with us. `text` is `None` when nothing is pending.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SharedContent {
  pub text: Option<String>,
  /// Which share-sheet entry the user picked. Set whenever `text` is.
  pub intent: Option<ShareIntent>,
}

/// The share-sheet entry the user picked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ShareIntent {
  /// "Clean & Share": clean the link and share it onwards.
  #[default]
  Share,
  /// "Save bookmark": clean the link and save it.
  Save,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct MeteredResponse {
  pub metered: bool,
}
