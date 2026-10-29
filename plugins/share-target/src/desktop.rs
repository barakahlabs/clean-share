use serde::de::DeserializeOwned;
use tauri::{plugin::PluginApi, AppHandle, Runtime};

use crate::models::*;

pub fn init<R: Runtime, C: DeserializeOwned>(
  app: &AppHandle<R>,
  _api: PluginApi<R, C>,
) -> crate::Result<ShareTarget<R>> {
  Ok(ShareTarget(app.clone()))
}

/// Access to the share-target APIs. Desktop has no share sheet, so nothing is ever pending.
pub struct ShareTarget<R: Runtime>(#[allow(dead_code)] AppHandle<R>);

impl<R: Runtime> ShareTarget<R> {
  pub fn get_shared(&self) -> crate::Result<SharedContent> {
    Ok(SharedContent::default())
  }
}
