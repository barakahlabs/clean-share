mod bookmarks;
mod previews;
mod rules;
mod settings;
mod transfer;

use tauri::Manager;

/// Splits shared text into tracking / non-tracking pieces and returns the cleaned text.
#[tauri::command]
fn clean_text(text: &str) -> link_cleaner::Analysis {
    link_cleaner::analyze_with(&rules::current(), text)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_share_target::init())
        // The web view shrinks above the on-screen keyboard (Android and iOS).
        .plugin(tauri_plugin_keyboard::init())
        // The status bar and keyboard follow the app's light or dark mode (Android and iOS).
        .plugin(tauri_plugin_appearance::init())
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
            // Newer rules downloaded earlier: the store cleans with them too. Then saved links
            // from older rules are re-cleaned, and checking for new rules starts.
            app.manage(rules::RulesState::load(app.handle()));
            if app.state::<rules::RulesState>().status().downloaded {
                bookmarks::use_rules(app.handle(), rules::current());
            }
            previews::start(app.handle().clone());
            bookmarks::reclean_outdated(app.handle());
            rules::start(app.handle().clone());
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
            bookmarks::bookmarks_reclean,
            previews::preview_fetch,
            previews::bookmark_preview_retry,
            transfer::bookmarks_export,
            transfer::bookmarks_import,
            rules::rules_status,
            rules::rules_check,
            settings::settings_get,
            settings::settings_set,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            // Back in front (e.g. on Wi-Fi now): let waiting previews go ahead, and check for new
            // rules if the last check is an hour old. This is the
            // window's Resumed (Android onResume, iOS willEnterForeground), not RunEvent::Resumed,
            // which is the event loop's.
            #[cfg(mobile)]
            if let tauri::RunEvent::WindowEvent {
                event: tauri::WindowEvent::Resumed,
                ..
            } = event
            {
                app.state::<previews::Previews>().wake();
                app.state::<rules::RulesState>().wake_if_due();
            }
            #[cfg(not(mobile))]
            let _ = (app, event);
        });
}
