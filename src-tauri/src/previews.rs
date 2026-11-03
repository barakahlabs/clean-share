//! Link previews for bookmarks: a background worker that fetches them (with `crates/link-preview`),
//! makes thumbnails, and stores the result. Only runs while Settings → previews is on.
//!
//! - Saving a bookmark (previews on) marks it pending and wakes the worker.
//! - The worker takes pending bookmarks newest first, one at a time, and emits
//!   `bookmark-preview` with the bookmark id when one is done or failed.
//! - The save sheet can fetch a preview before saving (`preview_fetch`); the worker reuses it.
//! - Thumbnails are `thumbnails/<id>.jpg` in the app data dir, served to the WebView on the
//!   `thumb` URI scheme (`convertFileSrc("<id>.jpg", "thumb")` in JS).

use std::{collections::VecDeque, path::PathBuf, sync::Mutex};

use ::bookmarks::{PageInfo, PreviewData, PreviewStatus};
use link_preview::{Config, Fetcher, Preview, THUMBNAIL_SIZE};
use tauri::{
    AppHandle, Emitter, Manager, State,
    http::{Response, StatusCode, header},
};
use tokio::sync::Notify;

use crate::{
    bookmarks::{CommandError, CommandResult, Library},
    settings::SettingsState,
};

/// Previews fetched for the save sheet, kept until the bookmark is saved (or pushed out).
const CACHE_SIZE: usize = 16;
/// Pending bookmarks taken from the database per round.
const BATCH: u32 = 5;

pub struct Previews {
    /// `None` if the HTTP client couldn't be set up; previews are then unavailable.
    fetcher: Option<Fetcher>,
    wake: Notify,
    cache: Mutex<VecDeque<(String, Preview)>>,
    thumbnails: Option<PathBuf>,
}

impl Previews {
    pub fn new(app: &AppHandle) -> Self {
        let config = Config {
            accept_language: accept_language(),
            ..Config::default()
        };
        let fetcher = Fetcher::new(config)
            .inspect_err(|e| eprintln!("link previews unavailable: {e}"))
            .ok();
        Self {
            fetcher,
            wake: Notify::new(),
            cache: Mutex::new(VecDeque::new()),
            thumbnails: app
                .path()
                .app_data_dir()
                .ok()
                .map(|d| d.join("thumbnails")),
        }
    }

    /// Asks the worker to look at the queue (now, or right after its current round).
    pub fn wake(&self) {
        self.wake.notify_one();
    }

    fn thumbnail_path(&self, id: &str) -> Option<PathBuf> {
        // IDs are UUIDs; anything else never reaches the file system.
        let valid = !id.is_empty() && id.chars().all(|c| c.is_ascii_hexdigit() || c == '-');
        Some(self.thumbnails.as_ref()?.join(format!("{id}.jpg"))).filter(|_| valid)
    }

    /// Deletes a bookmark's thumbnail, if it has one.
    pub fn remove_thumbnail(&self, id: &str) {
        if let Some(path) = self.thumbnail_path(id) {
            let _ = std::fs::remove_file(path);
        }
    }

    /// Answers `thumb://localhost/<id>.jpg` (or `http://thumb.localhost/<id>.jpg` on Android).
    pub fn serve(&self, path: &str) -> Response<Vec<u8>> {
        let id = path.trim_start_matches('/').trim_end_matches(".jpg");
        match self.thumbnail_path(id).and_then(|p| std::fs::read(p).ok()) {
            Some(jpeg) => Response::builder()
                .header(header::CONTENT_TYPE, "image/jpeg")
                // A retried preview replaces the file; JS adds `?v=<fetched at>` to the URL.
                .header(header::CACHE_CONTROL, "no-cache")
                .body(jpeg)
                .unwrap(),
            None => Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Vec::new())
                .unwrap(),
        }
    }

    fn cached(&self, url: &str) -> Option<Preview> {
        let mut cache = self.cache.lock().unwrap();
        let at = cache.iter().position(|(u, _)| u == url)?;
        cache.remove(at).map(|(_, p)| p)
    }

    fn peek(&self, url: &str) -> Option<Preview> {
        let cache = self.cache.lock().unwrap();
        cache.iter().find(|(u, _)| u == url).map(|(_, p)| p.clone())
    }

    fn remember(&self, url: String, preview: Preview) {
        let mut cache = self.cache.lock().unwrap();
        cache.retain(|(u, _)| *u != url);
        if cache.len() == CACHE_SIZE {
            cache.pop_front();
        }
        cache.push_back((url, preview));
    }

    /// Fetches a preview and cleans the links in it (image, icon and embed can carry tracking).
    async fn fetch(&self, url: &str) -> Result<Preview, String> {
        let fetcher = self.fetcher.as_ref().ok_or("link previews are unavailable")?;
        let mut preview = fetcher.preview(url).await.map_err(|e| e.to_string())?;
        for link in [&mut preview.image_url, &mut preview.icon_url] {
            *link = link.take().map(|l| clean(&l));
        }
        if let Some(embed) = preview.embed.as_mut() {
            embed.url = embed.url.take().map(|l| clean(&l));
        }
        Ok(preview)
    }

    /// Downloads the preview image and stores it as the bookmark's thumbnail. Without an image
    /// (or if it can't be used), an old thumbnail is removed.
    async fn store_thumbnail(&self, id: &str, image_url: Option<&str>) {
        let (Some(fetcher), Some(path)) = (self.fetcher.as_ref(), self.thumbnail_path(id)) else {
            return;
        };
        let jpeg = match image_url {
            Some(url) => match fetcher.image(url).await {
                // Decoding and resizing is CPU work; keep it off the async threads.
                Ok(image) => tauri::async_runtime::spawn_blocking(move || {
                    link_preview::thumbnail(&image.bytes, THUMBNAIL_SIZE)
                })
                .await
                .map_err(|e| e.to_string())
                .and_then(|r| r.map_err(|e| e.to_string())),
                Err(e) => Err(e.to_string()),
            },
            None => Err("no image".into()),
        };
        match jpeg {
            Ok(jpeg) => {
                let written = path
                    .parent()
                    .map_or(Ok(()), std::fs::create_dir_all)
                    .and_then(|()| std::fs::write(&path, jpeg));
                if let Err(e) = written {
                    eprintln!("can't save thumbnail for {id}: {e}");
                }
            }
            Err(reason) => {
                if image_url.is_some() {
                    eprintln!("no thumbnail for {id}: {reason}");
                }
                let _ = std::fs::remove_file(&path);
            }
        }
    }
}

/// Starts the worker. It runs for the app's lifetime and sleeps until woken.
pub fn start(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            run_queue(&app).await;
            app.state::<Previews>().wake.notified().await;
        }
    });
}

/// Works through the pending bookmarks until none are left (or previews get switched off).
async fn run_queue(app: &AppHandle) {
    let previews = app.state::<Previews>();
    let library = app.state::<Library>();
    let settings = app.state::<SettingsState>();
    loop {
        if !settings.get().previews {
            return;
        }
        let batch = match library.store.lock().await.as_ref() {
            Some(store) => match store.preview_queue(BATCH).await {
                Ok(batch) => batch,
                Err(e) => return eprintln!("preview queue: {e}"),
            },
            None => return,
        };
        if batch.is_empty() {
            return;
        }
        for (id, url) in batch {
            let fetched = match previews.cached(&url) {
                Some(preview) => Ok(preview),
                None => previews.fetch(&url).await,
            };
            if let Ok(preview) = &fetched {
                previews
                    .store_thumbnail(&id, preview.image_url.as_deref())
                    .await;
            }
            // The store lock is only held for the write, never during network requests.
            let mut guard = library.store.lock().await;
            let Some(store) = guard.as_mut() else { return };
            let written = match fetched {
                Ok(preview) => store.set_preview(&id, &preview_data(preview)).await.map(|_| ()),
                Err(reason) => {
                    eprintln!("preview for {url} failed: {reason}");
                    store.set_preview_status(&id, PreviewStatus::Failed).await
                }
            };
            drop(guard);
            match written {
                Ok(()) => {
                    let _ = app.emit("bookmark-preview", &id);
                }
                // Deleted while its preview was being fetched.
                Err(::bookmarks::Error::NotFound) => previews.remove_thumbnail(&id),
                // Stop rather than retry the same bookmark forever.
                Err(e) => return eprintln!("can't store the preview for {id}: {e}"),
            }
        }
    }
}

fn preview_data(p: Preview) -> PreviewData {
    PreviewData {
        title: p.title,
        description: p.description,
        site_name: p.site_name,
        image_url: p.image_url,
        icon_url: p.icon_url,
        author: p.author,
        page_kind: p.kind,
        keywords: p.keywords,
        published: p.published,
        excerpt: p.excerpt,
        embed_kind: p.embed.as_ref().map(|e| e.kind.clone()),
        embed_url: p.embed.and_then(|e| e.url),
    }
}

/// What a cached preview (fetched for the save sheet) says about the page, for tag suggestions.
pub fn page_info(previews: &Previews, text: &str) -> Option<PageInfo> {
    let url = link_cleaner::links(text).into_iter().next()?.cleaned;
    let p = previews.peek(&url)?;
    Some(PageInfo {
        description: p.description,
        excerpt: p.excerpt,
        keywords: p.keywords,
        kind: p.kind,
    })
}

/// The first link in `link`, cleaned; the link itself if the rules find nothing.
fn clean(link: &str) -> String {
    link_cleaner::links(link)
        .into_iter()
        .next()
        .map_or_else(|| link.to_string(), |l| l.cleaned)
}

/// The phone's languages as an `Accept-Language` header: `de-DE,en-US;q=0.9,…`.
fn accept_language() -> Option<String> {
    let locales: Vec<String> = sys_locale::get_locales().take(5).collect();
    if locales.is_empty() {
        return None;
    }
    let header = locales
        .iter()
        .enumerate()
        .map(|(i, l)| match i {
            0 => l.clone(),
            _ => format!("{l};q=0.{}", 10 - i),
        })
        .collect::<Vec<_>>()
        .join(",");
    Some(header)
}

/// Fetches a preview for the save sheet, before saving: the first link in `text`. Returns
/// `None` when previews are off. The worker reuses it once the bookmark is saved.
#[tauri::command]
pub async fn preview_fetch(
    previews: State<'_, Previews>,
    settings: State<'_, SettingsState>,
    text: String,
) -> CommandResult<Option<Preview>> {
    if !settings.get().previews {
        return Ok(None);
    }
    let Some(link) = link_cleaner::links(&text).into_iter().next() else {
        return Ok(None);
    };
    if let Some(preview) = previews.peek(&link.cleaned) {
        return Ok(Some(preview));
    }
    let preview = previews.fetch(&link.cleaned).await.map_err(CommandError)?;
    previews.remember(link.cleaned, preview.clone());
    Ok(Some(preview))
}

/// Queues a bookmark's preview again (e.g. after it failed).
#[tauri::command]
pub async fn bookmark_preview_retry(
    library: State<'_, Library>,
    previews: State<'_, Previews>,
    id: String,
) -> CommandResult<()> {
    if let Some(store) = library.store.lock().await.as_mut() {
        store.set_preview_status(&id, PreviewStatus::Pending).await?;
    }
    previews.wake();
    Ok(())
}
