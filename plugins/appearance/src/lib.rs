//! Makes the native parts of the app follow the light or dark mode chosen in the app, on Android
//! and iOS. The page's colours are CSS (src/theme); this covers what CSS can't reach.
//!
//! - iOS (`ios/Sources/AppearancePlugin.swift`): sets the window's interface style, so the status
//!   bar, the keyboard and the page's `prefers-color-scheme` follow it.
//! - Android (`android/src/main/java/AppearancePlugin.kt`): sets the status and navigation bar
//!   icons to dark or light.
//!
//! One command, `set_mode` with `"system"`, `"light"` or `"dark"`. Desktop ignores it.

use serde::{Deserialize, Serialize};
use tauri::{
  AppHandle, Runtime,
  plugin::{Builder, TauriPlugin},
};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
  /// Follow the phone's setting.
  System,
  Light,
  Dark,
}

#[cfg(target_os = "ios")]
tauri::ios_plugin_binding!(init_plugin_appearance);

#[cfg(mobile)]
struct Native<R: Runtime>(tauri::plugin::PluginHandle<R>);

#[derive(Serialize)]
struct SetMode {
  mode: Mode,
}

// Async, so it runs off the main thread: the native side switches to the main thread to change the
// window, and a command waiting on the main thread for that would never finish (iOS kills the app).
#[tauri::command]
async fn set_mode<R: Runtime>(app: AppHandle<R>, mode: Mode) -> Result<(), String> {
  #[cfg(mobile)]
  {
    use tauri::Manager;
    app
      .state::<Native<R>>()
      .0
      .run_mobile_plugin::<serde::de::IgnoredAny>("setMode", SetMode { mode })
      .map_err(|e| e.to_string())?;
  }
  #[cfg(not(mobile))]
  let _ = (app, SetMode { mode });
  Ok(())
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
  Builder::new("appearance")
    .invoke_handler(tauri::generate_handler![set_mode])
    .setup(|_app, _api| {
      #[cfg(target_os = "android")]
      {
        use tauri::Manager;
        let handle = _api.register_android_plugin("com.cleanshare.appearance", "AppearancePlugin")?;
        _app.manage(Native(handle));
      }
      #[cfg(target_os = "ios")]
      {
        use tauri::Manager;
        let handle = _api.register_ios_plugin(init_plugin_appearance)?;
        _app.manage(Native(handle));
      }
      Ok(())
    })
    .build()
}
