const COMMANDS: &[&str] = &["get_shared", "register_listener", "remove_listener"];

// Adds the app to the Android share sheet for plain text (links shared from
// browsers arrive as text). Inserted into the app's MainActivity at build time.
// The helper indents every line to match the parent, so keep these relative.
const ANDROID_SHARE_INTENT_FILTER: &str = r#"<intent-filter>
    <action android:name="android.intent.action.SEND" />
    <category android:name="android.intent.category.DEFAULT" />
    <data android:mimeType="text/plain" />
</intent-filter>"#;

fn main() {
  tauri_plugin::Builder::new(COMMANDS)
    .android_path("android")
    .ios_path("ios")
    .build();

  tauri_plugin::mobile::update_android_manifest(
    "SHARE TARGET PLUGIN",
    "activity",
    ANDROID_SHARE_INTENT_FILTER.to_string(),
  )
  .expect("failed to add share intent-filter to AndroidManifest.xml");
}
