use tauri::{
  plugin::{Builder, TauriPlugin},
  Manager, Runtime,
};

pub use models::*;

#[cfg(desktop)]
mod desktop;
#[cfg(mobile)]
mod mobile;

mod commands;
mod error;
mod models;

pub use error::{Error, Result};

#[cfg(desktop)]
use desktop::ShareTarget;
#[cfg(mobile)]
use mobile::ShareTarget;

/// Extensions to [`tauri::App`], [`tauri::AppHandle`] and [`tauri::Window`] to access the share-target APIs.
pub trait ShareTargetExt<R: Runtime> {
  fn share_target(&self) -> &ShareTarget<R>;
}

impl<R: Runtime, T: Manager<R>> crate::ShareTargetExt<R> for T {
  fn share_target(&self) -> &ShareTarget<R> {
    self.state::<ShareTarget<R>>().inner()
  }
}

/// Initializes the plugin.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
  Builder::new("share-target")
    .invoke_handler(tauri::generate_handler![commands::get_shared, commands::share_text])
    .setup(|app, api| {
      #[cfg(mobile)]
      let share_target = mobile::init(app, api)?;
      #[cfg(desktop)]
      let share_target = desktop::init(app, api)?;
      app.manage(share_target);
      Ok(())
    })
    .build()
}
