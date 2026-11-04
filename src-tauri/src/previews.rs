//! Link previews for bookmarks: a background worker that fetches them (with `crates/link-preview`),
//! makes thumbnails, and stores the result. Only runs while Settings → previews is on, and with
//! "Wi-Fi only" only on unmetered connections (otherwise previews stay queued until the app is
//! next brought to the front on Wi-Fi).
//!
//! - Saving a bookmark (previews on) marks it pending and wakes the worker.
//! - The worker takes pending bookmarks newest first, one at a time, and emits
//!   `bookmark-preview` with the bookmark id when one is done or failed.
//! - The save sheet can fetch a preview before saving (`preview_fetch`); the worker reuses it.
//! - Thumbnails are `thumbnails/<id>.jpg` in the app data dir, served to the WebView on the
//!   `thumb` URI scheme (`convertFileSrc("<id>.jpg", "thumb")` in JS). A bookmark without a
//!   usable picture gets its site's icon instead, `thumbnails/<id>-icon.png` (the page's
//!   declared icon, else the site's `/favicon.ico`).

use std::{collections::VecDeque, path::PathBuf, sync::Mutex};

use ::bookmarks::{PageInfo, PreviewData, PreviewStatus};
use link_preview::{Config, Fetcher, ICON_SIZE, Preview, THUMBNAIL_SIZE};
use tauri::{
    AppHandle, Emitter, Manager, State,
    http::{Response, StatusCode, header},
};
use tauri_plugin_share_target::ShareTargetExt;
use tokio::sync::Notify;

use crate::{
    bookmarks::{CommandError, CommandResult, Library},
    settings::{Settings, SettingsState},
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

    /// `thumbnails/<id><suffix>`. IDs are UUIDs; anything else never reaches the file system.
    fn file(&self, id: &str, suffix: &str) -> Option<PathBuf> {
        let valid = !id.is_empty() && id.chars().all(|c| c.is_ascii_hexdigit() || c == '-');
        Some(self.thumbnails.as_ref()?.join(format!("{id}{suffix}"))).filter(|_| valid)
    }

    /// Deletes a bookmark's thumbnail and icon, if it has them.
    pub fn remove_thumbnail(&self, id: &str) {
        for suffix in [THUMB, ICON] {
            if let Some(path) = self.file(id, suffix) {
                let _ = std::fs::remove_file(path);
            }
        }
    }

    /// Answers `thumb://localhost/<id>.jpg` or `…/<id>-icon.png` (on Android
    /// `http://thumb.localhost/…`).
    pub fn serve(&self, path: &str) -> Response<Vec<u8>> {
        let name = path.trim_start_matches('/');
        let (id, suffix, content_type) = match name.strip_suffix(ICON) {
            Some(id) => (id, ICON, "image/png"),
            None => (name.trim_end_matches(THUMB), THUMB, "image/jpeg"),
        };
        match self.file(id, suffix).and_then(|p| std::fs::read(p).ok()) {
            Some(bytes) => Response::builder()
                .header(header::CONTENT_TYPE, content_type)
                // A retried preview replaces the file; JS adds `?v=<fetched at>` to the URL.
                .header(header::CACHE_CONTROL, "no-cache")
                .body(bytes)
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

    /// Stores the bookmark's picture: the preview image as a thumbnail, or, without a usable
    /// one, the site's icon. Whatever doesn't apply any more (after a retry) is removed.
    async fn store_pictures(&self, id: &str, preview: &Preview) {
        let thumbnail = match preview.image_url.as_deref() {
            Some(url) => self.picture(url, |b| link_preview::thumbnail(b, THUMBNAIL_SIZE)).await,
            None => Err("no image".into()),
        };
        if let Err(reason) = &thumbnail
            && preview.image_url.is_some()
        {
            eprintln!("no thumbnail for {id}: {reason}");
        }
        let icon = if thumbnail.is_ok() {
            Err("has a thumbnail".to_string())
        } else {
            match preview.icon_url.clone().or_else(|| favicon(&preview.url)) {
                Some(url) => self.picture(&url, |b| link_preview::icon(b, ICON_SIZE)).await,
                None => Err("no icon".into()),
            }
        };
        self.keep(id, THUMB, thumbnail.ok());
        self.keep(id, ICON, icon.ok());
    }

    /// After a failed fetch (e.g. the page is gone): the site's icon, from its homepage's
    /// declared icons, else its `/favicon.ico`. Not if the bookmark already has a picture or icon
    /// from an earlier preview, which it keeps.
    async fn store_site_icon(&self, id: &str, page_url: &str) {
        let has_picture = [THUMB, ICON].iter().any(|s| self.file(id, s).is_some_and(|p| p.exists()));
        let Some(home) = origin(page_url) else { return };
        if has_picture {
            return;
        }
        let declared = match self.fetch(&format!("{home}/")).await {
            Ok(preview) => preview.icon_url,
            Err(_) => None,
        };
        for url in declared.into_iter().chain(favicon(page_url)) {
            if let Ok(png) = self.picture(&url, |b| link_preview::icon(b, ICON_SIZE)).await {
                return self.keep(id, ICON, Some(png));
            }
        }
    }

    /// Downloads an image and converts it (off the async threads: decoding is CPU work).
    async fn picture(
        &self,
        url: &str,
        convert: fn(&[u8]) -> link_preview::Result<Vec<u8>>,
    ) -> Result<Vec<u8>, String> {
        let fetcher = self.fetcher.as_ref().ok_or("link previews are unavailable")?;
        let image = fetcher.image(url).await.map_err(|e| e.to_string())?;
        tauri::async_runtime::spawn_blocking(move || convert(&image.bytes))
            .await
            .map_err(|e| e.to_string())?
            .map_err(|e| e.to_string())
    }

    /// Writes `thumbnails/<id><suffix>`, or removes it when there's nothing to keep.
    fn keep(&self, id: &str, suffix: &str, bytes: Option<Vec<u8>>) {
        let Some(path) = self.file(id, suffix) else { return };
        let result = match bytes {
            Some(bytes) => path
                .parent()
                .map_or(Ok(()), std::fs::create_dir_all)
                .and_then(|()| std::fs::write(&path, bytes)),
            None => match std::fs::remove_file(&path) {
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
                other => other,
            },
        };
        if let Err(e) = result {
            eprintln!("can't update {}: {e}", path.display());
        }
    }
}

const THUMB: &str = ".jpg";
const ICON: &str = "-icon.png";

/// `https://blog.rust-lang.org` for any page on it.
fn origin(page_url: &str) -> Option<String> {
    let url = tauri::Url::parse(page_url).ok()?;
    Some(url.origin().ascii_serialization()).filter(|o| o != "null")
}

/// The site's standard icon, for pages that declare none (or that refused us, like Reddit's).
fn favicon(page_url: &str) -> Option<String> {
    Some(format!("{}/favicon.ico", origin(page_url)?))
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

/// True if previews may go online now: switched on, and on Wi-Fi when that's asked for.
fn may_fetch(app: &AppHandle, settings: &Settings) -> bool {
    settings.previews
        && !(settings.previews_wifi_only && app.share_target().is_metered().unwrap_or(true))
}

/// Works through the pending bookmarks until none are left, or previews may not go online.
async fn run_queue(app: &AppHandle) {
    let previews = app.state::<Previews>();
    let library = app.state::<Library>();
    let settings = app.state::<SettingsState>();
    loop {
        if !may_fetch(app, &settings.get()) {
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
            match &fetched {
                Ok(preview) => previews.store_pictures(&id, preview).await,
                // The page failed (e.g. 404), but the site usually still has an icon.
                Err(_) => previews.store_site_icon(&id, &url).await,
            }
            // The store lock is only held for the write, never during network requests.
            let mut guard = library.store.lock().await;
            let Some(store) = guard.as_mut() else { return };
            let written = match fetched {
                Ok(preview) => store.set_preview(&id, &preview_data(preview)).await.map(|_| ()),
                Err(reason) => {
                    eprintln!("preview for {url} failed: {reason}");
                    store.preview_failed(&id).await
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
    let url = link_cleaner::links_with(&crate::rules::current(), text).into_iter().next()?.cleaned;
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
    link_cleaner::links_with(&crate::rules::current(), link)
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
/// `None` when previews are off (or Wi-Fi only on mobile data). The worker reuses it once the
/// bookmark is saved.
#[tauri::command]
pub async fn preview_fetch(
    app: AppHandle,
    previews: State<'_, Previews>,
    settings: State<'_, SettingsState>,
    text: String,
) -> CommandResult<Option<Preview>> {
    if !may_fetch(&app, &settings.get()) {
        return Ok(None);
    }
    let Some(link) = link_cleaner::links_with(&crate::rules::current(), &text).into_iter().next() else {
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
