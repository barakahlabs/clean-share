//! Bookmarks commands: thin wrappers around `crates/bookmarks`.

use std::path::PathBuf;

use ::bookmarks::{
    Bookmark, BookmarkPatch, BookmarkTag, ListQuery, NewBookmark, Page, Prepared, PreviewStatus,
    RecleanReport, Saved, Store, Suggestion, TagCount, TagDraft, TagInput, db::Location,
};
use serde::{Serialize, Serializer};
use tauri::{AppHandle, Manager, State, async_runtime::Mutex};

use crate::{previews::Previews, settings::SettingsState};

/// The bookmarks database the app is using.
///
/// Today that's the local file in the app data dir. With paid sync (phase 8), sign-in swaps in
/// the user's synced database and sign-out swaps it back, by replacing the store in the mutex.
/// `None` means bookmarks are unavailable (the database couldn't be opened); cleaning and
/// sharing still work.
pub struct Library {
    pub(crate) store: Mutex<Option<Store>>,
    unavailable: Option<String>,
}

impl Library {
    /// Opens (creating and migrating) `bookmarks.db` in the app data dir.
    pub fn open_local(app: &AppHandle) -> Self {
        match tauri::async_runtime::block_on(open_store(app)) {
            Ok(store) => Self {
                store: Mutex::new(Some(store)),
                unavailable: None,
            },
            Err(reason) => {
                eprintln!("bookmarks unavailable: {reason}");
                Self {
                    store: Mutex::new(None),
                    unavailable: Some(reason),
                }
            }
        }
    }
}

async fn open_store(app: &AppHandle) -> Result<Store, String> {
    let path = database_path(app)?;
    Store::open(&Location::File(path))
        .await
        .map_err(|e| e.to_string())
}

fn database_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("can't create {}: {e}", dir.display()))?;
    Ok(dir.join("bookmarks.db"))
}

/// Errors reach JS as a plain message string.
#[derive(Debug)]
pub struct CommandError(pub String);

impl Serialize for CommandError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl From<::bookmarks::Error> for CommandError {
    fn from(error: ::bookmarks::Error) -> Self {
        Self(error.to_string())
    }
}

pub type CommandResult<T> = Result<T, CommandError>;

/// Runs `$body` (a `Store` call returning a future) with the store locked, or fails with why
/// bookmarks are unavailable.
macro_rules! with_store {
    ($library:expr, |$store:ident| $body:expr) => {{
        let mut guard = $library.store.lock().await;
        match guard.as_mut() {
            Some($store) => Ok($body.await?),
            None => Err(CommandError(format!(
                "Bookmarks are unavailable: {}",
                $library.unavailable.as_deref().unwrap_or("not open")
            ))),
        }
    }};
}

/// Cleans the links in shared text and says whether the first one is already saved.
#[tauri::command]
pub async fn bookmark_prepare(
    library: State<'_, Library>,
    text: String,
) -> CommandResult<Prepared> {
    with_store!(library, |store| store.prepare(&text))
}

/// Saves the first link in the shared text, or merges into the existing bookmark for it. With
/// previews on, a bookmark without a preview is queued for one.
#[tauri::command]
pub async fn bookmark_save(
    library: State<'_, Library>,
    settings: State<'_, SettingsState>,
    previews: State<'_, Previews>,
    bookmark: NewBookmark,
) -> CommandResult<Saved> {
    let mut saved: Saved = with_store!(library, |store| store.save(bookmark))?;
    if settings.get().previews && saved.bookmark.preview_status == PreviewStatus::None {
        let id = saved.bookmark.id.clone();
        with_store!(library, |store| store.set_preview_status(&id, PreviewStatus::Pending))?;
        saved.bookmark.preview_status = PreviewStatus::Pending;
        previews.wake();
    }
    Ok(saved)
}

/// A page of bookmarks: newest first, or by relevance when searching.
#[tauri::command]
pub async fn bookmark_list(
    library: State<'_, Library>,
    query: Option<ListQuery>,
) -> CommandResult<Page<Bookmark>> {
    let query = query.unwrap_or_default();
    with_store!(library, |store| store.list(&query))
}

#[tauri::command]
pub async fn bookmark_get(
    library: State<'_, Library>,
    id: String,
) -> CommandResult<Option<Bookmark>> {
    with_store!(library, |store| store.get(&id))
}

/// Changes a bookmark's title and note (`""` clears a field, a missing field is left as is).
#[tauri::command]
pub async fn bookmark_update(
    library: State<'_, Library>,
    id: String,
    patch: BookmarkPatch,
) -> CommandResult<Bookmark> {
    with_store!(library, |store| store.update(&id, patch))
}

/// Replaces a bookmark's tags. New tag names are created.
#[tauri::command]
pub async fn bookmark_set_tags(
    library: State<'_, Library>,
    id: String,
    tags: Vec<TagInput>,
) -> CommandResult<Vec<BookmarkTag>> {
    with_store!(library, |store| store.set_tags(&id, &tags))
}

/// Deletes a bookmark and its thumbnail.
#[tauri::command]
pub async fn bookmark_delete(
    library: State<'_, Library>,
    previews: State<'_, Previews>,
    id: String,
) -> CommandResult<()> {
    with_store!(library, |store| store.delete(&id))?;
    previews.remove_thumbnail(&id);
    Ok(())
}

/// Every tag with how many bookmarks use it (0 for unused tags), most used first.
#[tauri::command]
pub async fn tag_list(library: State<'_, Library>) -> CommandResult<Vec<TagCount>> {
    with_store!(library, |store| store.tags())
}

/// Tags to suggest for the bookmark on the save sheet, best first (never one already picked).
/// Uses the page's keywords and kind when the sheet already fetched its preview.
#[tauri::command]
pub async fn tag_suggest(
    library: State<'_, Library>,
    previews: State<'_, Previews>,
    mut draft: TagDraft,
) -> CommandResult<Vec<Suggestion>> {
    if draft.page.is_none() {
        draft.page = crate::previews::page_info(&previews, &draft.text);
    }
    with_store!(library, |store| store.suggest_tags(&draft))
}

/// Renames a tag; if a tag named `to` exists, the two are merged.
#[tauri::command]
pub async fn tag_rename(
    library: State<'_, Library>,
    from: String,
    to: String,
) -> CommandResult<()> {
    with_store!(library, |store| store.rename_tag(&from, &to))
}

/// Deletes a tag, removing it from every bookmark that has it.
#[tauri::command]
pub async fn tag_delete(library: State<'_, Library>, name: String) -> CommandResult<()> {
    with_store!(library, |store| store.delete_tag(&name))
}

/// Cleans every saved link again with the current rules (Settings). Links that turn out to be
/// the same are merged; the merged-away bookmarks' thumbnails are deleted.
#[tauri::command]
pub async fn bookmarks_reclean(
    library: State<'_, Library>,
    previews: State<'_, Previews>,
) -> CommandResult<RecleanReport> {
    let report: RecleanReport = with_store!(library, |store| store.reclean(true))?;
    for id in &report.removed {
        previews.remove_thumbnail(id);
    }
    Ok(report)
}

/// On launch: re-cleans the bookmarks saved with older rules (the app's rules changed with an
/// update). Quiet; nothing to do when the rules are the same.
pub fn reclean_outdated(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let library = app.state::<Library>();
        let report = match library.store.lock().await.as_mut() {
            Some(store) => store.reclean(false).await,
            None => return,
        };
        match report {
            Ok(report) => {
                for id in &report.removed {
                    app.state::<Previews>().remove_thumbnail(id);
                }
                if report.changed > 0 {
                    eprintln!(
                        "re-cleaned {} saved links with the new rules ({} merged)",
                        report.changed, report.merged
                    );
                }
            }
            Err(e) => eprintln!("re-cleaning saved links failed: {e}"),
        }
    });
}

/// Makes the store clean with `rules` (newer rules than the built-in ones).
pub fn use_rules(app: &AppHandle, rules: std::sync::Arc<link_rules::RuleSet>) {
    let library = app.state::<Library>();
    tauri::async_runtime::block_on(async {
        if let Some(store) = library.store.lock().await.as_mut() {
            store.set_cleaner(std::sync::Arc::new(::bookmarks::Rules((*rules).clone())));
        }
    });
}
