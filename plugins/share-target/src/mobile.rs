use serde::de::DeserializeOwned;
use tauri::{
  plugin::{PluginApi, PluginHandle},
  AppHandle, Runtime,
};

use crate::models::*;

#[cfg(target_os = "ios")]
tauri::ios_plugin_binding!(init_plugin_share_target);

// initializes the Kotlin or Swift plugin classes
pub fn init<R: Runtime, C: DeserializeOwned>(
  _app: &AppHandle<R>,
  api: PluginApi<R, C>,
) -> crate::Result<ShareTarget<R>> {
  #[cfg(target_os = "android")]
  let handle = api.register_android_plugin("com.cleanshare.sharetarget", "ShareTargetPlugin")?;
  #[cfg(target_os = "ios")]
  let handle = api.register_ios_plugin(init_plugin_share_target)?;
  Ok(ShareTarget(handle))
}

/// Access to the share-target APIs.
pub struct ShareTarget<R: Runtime>(PluginHandle<R>);

impl<R: Runtime> ShareTarget<R> {
  pub fn share_text(&self, payload: ShareTextRequest) -> crate::Result<()> {
    self
      .0
      .run_mobile_plugin("shareText", payload)
      .map_err(Into::into)
  }

  pub fn return_to_source(&self) -> crate::Result<bool> {
    self
      .0
      .run_mobile_plugin::<ReturnToSourceResponse>("returnToSource", ())
      .map(|response| response.returned)
      .map_err(Into::into)
  }

  /// Opens the share sheet with a file. On Android it must be in the app's cache dir (the
  /// template FileProvider's paths).
  pub fn share_file(&self, payload: ShareFileRequest) -> crate::Result<()> {
    self.0.run_mobile_plugin("shareFile", payload).map_err(Into::into)
  }

  /// Whether the phone's current connection is metered (mobile data, a hotspot, Low Data
  /// Mode) or there's none.
  pub fn is_metered(&self) -> crate::Result<bool> {
    self
      .0
      .run_mobile_plugin::<MeteredResponse>("isMetered", ())
      .map(|response| response.metered)
      .map_err(Into::into)
  }

  pub fn get_shared(&self) -> crate::Result<SharedContent> {
    self
      .0
      .run_mobile_plugin("getShared", ())
      .map_err(Into::into)
  }
}
