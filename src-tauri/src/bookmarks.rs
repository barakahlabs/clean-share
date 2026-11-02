//! Bookmarks commands: thin wrappers around `crates/bookmarks`.

use std::path::PathBuf;

use ::bookmarks::{
    Bookmark, BookmarkPatch, BookmarkTag, ListQuery, NewBookmark, Page, Prepared, Saved, Store,
    Suggestion, TagCount, TagDraft, TagInput, db::Location,
};
use serde::{Serialize, Serializer};
use tauri::{AppHandle, Manager, State, async_runtime::Mutex};

/// The bookmarks database the app is using.
///
/// Today that's the local file in the app data dir. With paid sync (phase 8), sign-in swaps in
/// the user's synced database and sign-out swaps it back, by replacing the store in the mutex.
/// `None` means bookmarks are unavailable (the database couldn't be opened); cleaning and
/// sharing still work.
pub struct Library {
    store: Mutex<Option<Store>>,
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
pub struct CommandError(String);

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

type CommandResult<T> = Result<T, CommandError>;

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

/// Saves the first link in the shared text, or merges into the existing bookmark for it.
#[tauri::command]
pub async fn bookmark_save(
    library: State<'_, Library>,
    bookmark: NewBookmark,
) -> CommandResult<Saved> {
    with_store!(library, |store| store.save(bookmark))
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

#[tauri::command]
pub async fn bookmark_delete(library: State<'_, Library>, id: String) -> CommandResult<()> {
    with_store!(library, |store| store.delete(&id))
}

/// Every tag with how many bookmarks use it (0 for unused tags), most used first.
#[tauri::command]
pub async fn tag_list(library: State<'_, Library>) -> CommandResult<Vec<TagCount>> {
    with_store!(library, |store| store.tags())
}

/// Tags to suggest for the bookmark on the save sheet, best first (never one already picked).
#[tauri::command]
pub async fn tag_suggest(
    library: State<'_, Library>,
    draft: TagDraft,
) -> CommandResult<Vec<Suggestion>> {
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
