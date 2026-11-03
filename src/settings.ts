import { invoke } from "@tauri-apps/api/core";

// App settings (src-tauri/src/settings.rs). Kept on this phone only.

export type Settings = {
  /** Fetch link previews (title, image, description) for bookmarks. Off by default. */
  previews: boolean;
  /** Only fetch previews on Wi-Fi (unmetered); on mobile data they wait until the app is next
   * open on Wi-Fi. */
  previewsWifiOnly: boolean;
};

export function getSettings(): Promise<Settings> {
  return invoke("settings_get");
}

/** Saves the settings. Switching previews on fetches them for existing bookmarks too. */
export function setSettings(settings: Settings): Promise<Settings> {
  return invoke("settings_set", { settings });
}
