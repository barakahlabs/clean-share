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
        .invoke_handler(tauri::generate_handler![clean_text])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
