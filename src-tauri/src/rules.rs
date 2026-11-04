//! The tracking rules in use: the ones built into the app, or newer hosted ones (crates/rules-update).
//!
//! - On launch, a cached download is used if it still verifies and is newer than the built-in rules.
//! - The app checks for new rules on launch, then every hour while it's open, and when it comes back
//!   to the front after an hour or more. Each check asks only "has it changed?" (HTTP `ETag` /
//!   `If-None-Match`): unchanged, the answer is a 304 without a body; changed, the bundle and its
//!   signature are downloaded (a few KB). Mobile data is fine.
//! - A new bundle is used only if [`rules_update::verify`] accepts it and its revision is newer.
//!   Then it's cached, the store switches to it, and saved links are re-cleaned.
//! - Anything that fails (offline, a bad file) leaves the rules in use as they were.

use std::{
    path::PathBuf,
    sync::{Arc, Mutex, OnceLock, RwLock},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use ::bookmarks::Rules;
use link_preview::{Config, Download, Fetcher};
use link_rules::RuleSet;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::Notify;

use crate::{
    bookmarks::{CommandError, CommandResult, Library},
    previews::Previews,
};

/// How often to check while the app is open.
const CHECK_EVERY: Duration = Duration::from_secs(60 * 60);

static CURRENT: OnceLock<RwLock<Arc<RuleSet>>> = OnceLock::new();

/// The rules in use right now (what the clean screen, previews and the store use).
pub fn current() -> Arc<RuleSet> {
    CURRENT
        .get_or_init(|| RwLock::new(Arc::new(link_cleaner::bundled_rules().clone())))
        .read()
        .unwrap()
        .clone()
}

fn set_current(rules: RuleSet) {
    current();
    *CURRENT.get().unwrap().write().unwrap() = Arc::new(rules);
}

/// What's in `rules/state.json` in the app data dir.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CheckState {
    /// The server's fingerprint of the last bundle downloaded.
    etag: Option<String>,
    /// When the app last asked (ms since the epoch), whatever the answer.
    checked_at: Option<i64>,
}

/// For Settings: which rules are in use, and when they were last checked.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RulesStatus {
    pub revision: String,
    /// True if they came from a download (not the ones built into the app).
    pub downloaded: bool,
    pub checked_at: Option<i64>,
}

/// What's in the rules cache.
enum Cached {
    /// Nothing saved.
    None,
    /// Saved, verified and newer than the built-in rules.
    Use(RuleSet),
    /// Saved and fine, but the built-in rules are as new (the app was updated since).
    NotNewer,
    /// Saved but doesn't verify (damaged or partly written).
    Broken,
}

pub struct RulesState {
    dir: Option<PathBuf>,
    fetcher: Option<Fetcher>,
    state: Mutex<CheckState>,
    wake: Notify,
}

impl RulesState {
    /// Loads the cached download if it verifies and beats the built-in rules.
    pub fn load(app: &AppHandle) -> Self {
        let dir = app.path().app_data_dir().ok().map(|d| d.join("rules"));
        let state = dir
            .as_ref()
            .and_then(|d| std::fs::read(d.join("state.json")).ok())
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default();
        let this = Self {
            fetcher: Fetcher::new(Config::default())
                .inspect_err(|e| eprintln!("rule updates unavailable: {e}"))
                .ok(),
            dir,
            state: Mutex::new(state),
            wake: Notify::new(),
        };
        match this.cached() {
            Cached::Use(rules) => set_current(rules),
            // A saved file that no longer verifies: forget its fingerprint, so the next check
            // downloads a good copy.
            Cached::Broken => {
                this.state.lock().unwrap().etag = None;
                this.save_state();
            }
            Cached::None | Cached::NotNewer => {}
        }
        this
    }

    fn cached(&self) -> Cached {
        let Some(dir) = self.dir.as_ref() else { return Cached::None };
        let (Ok(bundle), Ok(signature)) =
            (std::fs::read(dir.join("rules.json")), std::fs::read(dir.join("rules.json.sig")))
        else {
            return Cached::None;
        };
        match rules_update::verify(&bundle, &signature, &rules_update::PUBLIC_KEY) {
            Ok(rules) if rules_update::is_newer(&rules, &link_cleaner::bundled_rules().revision) => {
                Cached::Use(rules)
            }
            Ok(_) => Cached::NotNewer,
            Err(e) => {
                eprintln!("cached rules ignored: {e}");
                Cached::Broken
            }
        }
    }

    /// Checks again now (or right after a check that's running).
    // Used when the app comes back to the front, which only mobile has.
    #[cfg_attr(not(mobile), allow(dead_code))]
    pub fn wake(&self) {
        self.wake.notify_one();
    }

    /// Wakes the checker if the last check is an hour old or more (the app came back to the front).
    #[cfg_attr(not(mobile), allow(dead_code))]
    pub fn wake_if_due(&self) {
        let checked_at = self.state.lock().unwrap().checked_at.unwrap_or(0);
        if now_ms() - checked_at >= CHECK_EVERY.as_millis() as i64 {
            self.wake();
        }
    }

    pub fn status(&self) -> RulesStatus {
        let rules = current();
        RulesStatus {
            downloaded: rules.revision != link_cleaner::bundled_rules().revision,
            revision: rules.revision.clone(),
            checked_at: self.state.lock().unwrap().checked_at,
        }
    }

    fn save_state(&self) {
        let (Some(dir), state) = (self.dir.as_ref(), self.state.lock().unwrap().clone()) else {
            return;
        };
        let _ = std::fs::create_dir_all(dir)
            .and_then(|()| std::fs::write(dir.join("state.json"), serde_json::to_vec(&state).unwrap()));
    }

    /// One check. Returns the new rules if they should be used now.
    async fn check(&self) -> Result<Option<RuleSet>, String> {
        let fetcher = self.fetcher.as_ref().ok_or("rule updates are unavailable")?;
        let etag = self.state.lock().unwrap().etag.clone();
        let download = fetcher
            .download(rules_update::BUNDLE_URL, etag.as_deref(), rules_update::MAX_BUNDLE_BYTES)
            .await;
        self.state.lock().unwrap().checked_at = Some(now_ms());
        let (bundle, etag) = match download.map_err(|e| e.to_string())? {
            Download::NotModified => {
                self.save_state();
                return Ok(None);
            }
            Download::Fetched { bytes, etag } => (bytes, etag),
        };
        let signature = match fetcher.download(rules_update::SIGNATURE_URL, None, 1024).await {
            Ok(Download::Fetched { bytes, .. }) => bytes,
            Ok(Download::NotModified) => return Err("no signature".into()),
            Err(e) => return Err(e.to_string()),
        };
        let rules = rules_update::verify(&bundle, &signature, &rules_update::PUBLIC_KEY)
            .map_err(|e| e.to_string())?;
        if !rules_update::is_newer(&rules, &current().revision) {
            // Nothing to keep; remember the fingerprint so it isn't downloaded again.
            self.state.lock().unwrap().etag = etag;
            self.save_state();
            return Ok(None);
        }
        // Remember the fingerprint only once the file is saved: if saving fails, the next check
        // downloads it again instead of hearing "not modified" for a file the app doesn't have.
        let written = self.dir.as_ref().map(|dir| {
            std::fs::create_dir_all(dir)
                .and_then(|()| std::fs::write(dir.join("rules.json"), &bundle))
                .and_then(|()| std::fs::write(dir.join("rules.json.sig"), &signature))
        });
        match written {
            Some(Ok(())) => self.state.lock().unwrap().etag = etag,
            Some(Err(e)) => eprintln!("can't cache the new rules: {e}"),
            None => {}
        }
        self.save_state();
        Ok(Some(rules))
    }
}

/// Starts checking: now, then hourly, or sooner when woken.
pub fn start(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            check_and_apply(&app).await;
            let state = app.state::<RulesState>();
            let _ = tokio::time::timeout(CHECK_EVERY, state.wake.notified()).await;
        }
    });
}

async fn check_and_apply(app: &AppHandle) {
    let state = app.state::<RulesState>();
    match state.check().await {
        Ok(Some(rules)) => apply(app, rules).await,
        Ok(None) => {}
        Err(e) => eprintln!("checking for new rules: {e}"),
    }
    let _ = app.emit("rules-status", state.status());
}

/// Switches everything to `rules` and re-cleans saved links with them.
pub async fn apply(app: &AppHandle, rules: RuleSet) {
    eprintln!("using rules {}", rules.revision);
    set_current(rules.clone());
    let library = app.state::<Library>();
    let mut guard = library.store.lock().await;
    let Some(store) = guard.as_mut() else { return };
    store.set_cleaner(Arc::new(Rules(rules)));
    match store.reclean(false).await {
        Ok(report) => {
            for id in &report.removed {
                app.state::<Previews>().remove_thumbnail(id);
            }
        }
        Err(e) => eprintln!("re-cleaning saved links: {e}"),
    }
}

fn now_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as i64)
}

/// Which rules are in use, and when they were last checked.
#[tauri::command]
pub fn rules_status(state: State<'_, RulesState>) -> RulesStatus {
    state.status()
}

/// Checks for new rules now (Settings). Returns the status afterwards.
#[tauri::command]
pub async fn rules_check(app: AppHandle, state: State<'_, RulesState>) -> CommandResult<RulesStatus> {
    match state.check().await {
        Ok(Some(rules)) => apply(&app, rules).await,
        Ok(None) => {}
        Err(e) => return Err(CommandError(format!("Couldn't check for new rules: {e}"))),
    }
    Ok(state.status())
}
