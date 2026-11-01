use tauri::{command, AppHandle, Runtime};

use crate::models::*;
use crate::Result;
use crate::ShareTargetExt;

/// Opens the system share sheet with `text`.
#[command]
pub(crate) async fn share_text<R: Runtime>(app: AppHandle<R>, text: String) -> Result<()> {
  app.share_target().share_text(ShareTextRequest { text })
}

/// Goes back to the app that shared (Android). Returns false where that isn't possible (iOS,
/// desktop): the user switches back themselves.
#[command]
pub(crate) async fn return_to_source<R: Runtime>(app: AppHandle<R>) -> Result<bool> {
  app.share_target().return_to_source()
}

/// Returns the pending shared content and clears it, so each share is delivered once.
#[command]
pub(crate) async fn get_shared<R: Runtime>(app: AppHandle<R>) -> Result<SharedContent> {
  app.share_target().get_shared()
}
