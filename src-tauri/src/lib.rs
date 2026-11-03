mod bookmarks;
mod previews;
mod settings;
mod transfer;

use tauri::Manager;

/// Splits shared text into tracking / non-tracking pieces and returns the cleaned text.
#[tauri::command]
fn clean_text(text: &str) -> link_cleaner::Analysis {
    link_cleaner::analyze(text)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_share_target::init())
        // Thumbnails for the WebView: `convertFileSrc("<id>.jpg", "thumb")` in JS.
        .register_asynchronous_uri_scheme_protocol("thumb", |ctx, request, responder| {
            let app = ctx.app_handle().clone();
            let path = request.uri().path().to_string();
            std::thread::spawn(move || {
                responder.respond(app.state::<previews::Previews>().serve(&path));
            });
        })
        .setup(|app| {
            let library = bookmarks::Library::open_local(app.handle());
            app.manage(library);
            app.manage(settings::SettingsState::load(app.handle()));
            app.manage(previews::Previews::new(app.handle()));
            previews::start(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            clean_text,
            bookmarks::bookmark_prepare,
            bookmarks::bookmark_save,
            bookmarks::bookmark_list,
            bookmarks::bookmark_get,
            bookmarks::bookmark_update,
            bookmarks::bookmark_set_tags,
            bookmarks::bookmark_delete,
            bookmarks::tag_list,
            bookmarks::tag_suggest,
            bookmarks::tag_rename,
            bookmarks::tag_delete,
            previews::preview_fetch,
            previews::bookmark_preview_retry,
            transfer::bookmarks_export,
            transfer::bookmarks_import,
            settings::settings_get,
            settings::settings_set,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            // Back in front (e.g. on Wi-Fi now): let waiting previews go ahead. This is the
            // window's Resumed (Android onResume, iOS willEnterForeground), not RunEvent::Resumed,
            // which is the event loop's.
            #[cfg(mobile)]
            if let tauri::RunEvent::WindowEvent {
                event: tauri::WindowEvent::Resumed,
                ..
            } = event
            {
                app.state::<previews::Previews>().wake();
            }
            #[cfg(not(mobile))]
            let _ = (app, event);
        });
}
