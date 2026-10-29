use tauri::{command, AppHandle, Runtime};

use crate::models::*;
use crate::Result;
use crate::ShareTargetExt;

/// Opens the system share sheet with `text`.
#[command]
pub(crate) async fn share_text<R: Runtime>(app: AppHandle<R>, text: String) -> Result<()> {
  app.share_target().share_text(ShareTextRequest { text })
}

/// Returns the pending shared content and clears it, so each share is delivered once.
#[command]
pub(crate) async fn get_shared<R: Runtime>(app: AppHandle<R>) -> Result<SharedContent> {
  app.share_target().get_shared()
}
