//! On iOS the on-screen keyboard covers the web view, and the page has to measure it (with
//! `visualViewport`, which lags behind the keyboard's animation) and can still be dragged around
//! under it. This plugin does what Android does instead: the web view shrinks to the space above
//! the keyboard (its accessory bar included), animated with it, so the page lays out at the
//! bottom of what's visible with plain CSS. Like Capacitor's "native" keyboard resize.
//!
//! Nothing to call: adding the plugin is enough. Other platforms ignore it.

use tauri::{
  Runtime,
  plugin::{Builder, TauriPlugin},
};

#[cfg(target_os = "ios")]
tauri::ios_plugin_binding!(init_plugin_keyboard);

pub fn init<R: Runtime>() -> TauriPlugin<R> {
  Builder::new("keyboard")
    .setup(|_app, _api| {
      #[cfg(target_os = "ios")]
      _api.register_ios_plugin(init_plugin_keyboard)?;
      Ok(())
    })
    .build()
}
