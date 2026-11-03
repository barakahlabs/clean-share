//! App settings, kept in `settings.json` in the app data dir. Small and read often, so they live
//! in memory and are written through on every change.

use std::{path::PathBuf, sync::Mutex};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};

use crate::{
    bookmarks::{CommandError, CommandResult, Library},
    previews::Previews,
};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    /// Fetch link previews (title, image, description) for bookmarks. Off by default: it's the
    /// only thing in the app that goes online.
    pub previews: bool,
    /// Only fetch previews on unmetered connections (Wi-Fi). On mobile data or a hotspot they
    /// wait, queued, until the app is next open on Wi-Fi.
    pub previews_wifi_only: bool,
}

pub struct SettingsState {
    path: Option<PathBuf>,
    current: Mutex<Settings>,
}

impl SettingsState {
    /// Reads `settings.json`; a missing or unreadable file means the defaults.
    pub fn load(app: &AppHandle) -> Self {
        let path = app
            .path()
            .app_data_dir()
            .ok()
            .map(|dir| dir.join("settings.json"));
        let current = path
            .as_ref()
            .and_then(|p| std::fs::read(p).ok())
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default();
        Self {
            path,
            current: Mutex::new(current),
        }
    }

    pub fn get(&self) -> Settings {
        self.current.lock().unwrap().clone()
    }

    fn set(&self, settings: Settings) -> Result<(), String> {
        let path = self.path.as_ref().ok_or("no app data dir")?;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        // Write a temporary file, then rename: a crash never leaves half a file.
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_vec_pretty(&settings).unwrap())
            .map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, path).map_err(|e| e.to_string())?;
        *self.current.lock().unwrap() = settings;
        Ok(())
    }
}

#[tauri::command]
pub fn settings_get(state: State<'_, SettingsState>) -> Settings {
    state.get()
}

/// Saves the settings. Switching previews on queues every bookmark that never had one;
/// lifting Wi-Fi only lets waiting previews go ahead.
#[tauri::command]
pub async fn settings_set(
    state: State<'_, SettingsState>,
    library: State<'_, Library>,
    previews: State<'_, Previews>,
    settings: Settings,
) -> CommandResult<Settings> {
    let old = state.get();
    let turned_on = settings.previews && !old.previews;
    let unrestricted = old.previews_wifi_only && !settings.previews_wifi_only;
    state.set(settings).map_err(CommandError)?;
    if turned_on && let Some(store) = library.store.lock().await.as_mut() {
        store.queue_missing_previews().await?;
    }
    if turned_on || unrestricted {
        previews.wake();
    }
    Ok(state.get())
}
