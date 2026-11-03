//! Export and import commands: files in and out of the bookmarks (`crates/bookmarks` transfer).
//!
//! Export writes the file to the cache dir's `exports/` folder and opens the share sheet with it
//! (save to Files or Drive, send it, …). Import reads the file the system picker returned.

use std::{str::FromStr, time::SystemTime};

use ::bookmarks::ImportReport;
use serde::Deserialize;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_fs::{FilePath, FsExt};
use tauri_plugin_share_target::{ShareFileRequest, ShareTargetExt};

use crate::{
    bookmarks::{CommandError, CommandResult, Library},
    previews::Previews,
    settings::SettingsState,
};

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    /// Clean Share's own backup: everything, for importing here again.
    Json,
    /// Netscape bookmark HTML, for browsers and other bookmark apps.
    Html,
}

/// Writes every bookmark to a file and opens the share sheet with it.
#[tauri::command]
pub async fn bookmarks_export(
    app: AppHandle,
    library: State<'_, Library>,
    format: ExportFormat,
) -> CommandResult<()> {
    let (content, extension, mime_type) = {
        let guard = library.store.lock().await;
        let store = guard
            .as_ref()
            .ok_or_else(|| CommandError("Bookmarks are unavailable".into()))?;
        match format {
            ExportFormat::Json => (store.export_json().await?, "json", "application/json"),
            ExportFormat::Html => (store.export_html().await?, "html", "text/html"),
        }
    };
    let dir = app
        .path()
        .app_cache_dir()
        .map_err(|e| CommandError(e.to_string()))?
        .join("exports");
    // Only the latest export is kept; older ones were shared already.
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).map_err(|e| CommandError(e.to_string()))?;
    let path = dir.join(format!("clean-share-bookmarks-{}.{extension}", today()));
    std::fs::write(&path, content).map_err(|e| CommandError(e.to_string()))?;
    app.share_target()
        .share_file(ShareFileRequest {
            path: path.to_string_lossy().into_owned(),
            mime_type: mime_type.into(),
        })
        .map_err(|e| CommandError(e.to_string()))
}

/// Imports the file at `path` (as the file picker returned it). With link previews on, the
/// imported bookmarks are queued for one.
#[tauri::command]
pub async fn bookmarks_import(
    app: AppHandle,
    library: State<'_, Library>,
    settings: State<'_, SettingsState>,
    previews: State<'_, Previews>,
    path: String,
) -> CommandResult<ImportReport> {
    let file = FilePath::from_str(&path).map_err(|e| CommandError(e.to_string()))?;
    let text = app
        .fs()
        .read_to_string(file)
        .map_err(|e| CommandError(format!("Can't read the file: {e}")))?;
    let report = {
        let mut guard = library.store.lock().await;
        let store = guard
            .as_mut()
            .ok_or_else(|| CommandError("Bookmarks are unavailable".into()))?;
        let report = store
            .import(&text)
            .await
            .map_err(|e| CommandError(e.to_string()))?;
        if settings.get().previews && report.added > 0 {
            store.queue_missing_previews().await?;
        }
        report
    };
    previews.wake();
    Ok(report)
}

/// Today's date as YYYY-MM-DD (UTC), for file names.
fn today() -> String {
    let days = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() / 86_400) as i64;
    // Howard Hinnant's civil_from_days.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}
