// No commands: the plugin only reacts to the keyboard on iOS.
const COMMANDS: &[&str] = &[];

fn main() {
  tauri_plugin::Builder::new(COMMANDS).ios_path("ios").build();
}
