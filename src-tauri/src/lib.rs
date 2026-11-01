mod bookmarks;

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
        .plugin(tauri_plugin_share_target::init())
        .setup(|app| {
            let library = bookmarks::Library::open_local(app.handle());
            app.manage(library);
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
            bookmarks::tag_rename,
            bookmarks::tag_delete,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
