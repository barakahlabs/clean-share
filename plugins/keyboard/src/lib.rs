//! Shrinks the web view to the space above the on-screen keyboard, animated with it, on Android
//! and iOS. The page then lays out at the bottom of what's visible with plain CSS, and never sits
//! under the keyboard. Like Capacitor's "native" keyboard resize.
//!
//! - iOS (`ios/Sources/KeyboardPlugin.swift`): iOS covers the web view with the keyboard (and
//!   its accessory bar); on keyboard frame changes the web view's frame is resized.
//! - Android (`android/src/main/java/KeyboardPlugin.kt`): the app is edge-to-edge, which Android
//!   doesn't resize for the keyboard; the activity's content is padded by the keyboard's height.
//!
//! Nothing to call: adding the plugin is enough. Desktop ignores it.

use tauri::{
  Runtime,
  plugin::{Builder, TauriPlugin},
};

#[cfg(target_os = "ios")]
tauri::ios_plugin_binding!(init_plugin_keyboard);

pub fn init<R: Runtime>() -> TauriPlugin<R> {
  Builder::new("keyboard")
    .setup(|_app, _api| {
      #[cfg(target_os = "android")]
      _api.register_android_plugin("com.cleanshare.keyboard", "KeyboardPlugin")?;
      #[cfg(target_os = "ios")]
      _api.register_ios_plugin(init_plugin_keyboard)?;
      Ok(())
    })
    .build()
}
