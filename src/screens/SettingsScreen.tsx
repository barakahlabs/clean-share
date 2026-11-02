import { useEffect, useState } from "react";
import { listTags } from "../bookmarks";
import { getSettings, setSettings, type Settings } from "../settings";
import { TagsScreen } from "./TagsScreen";
import "./Screen.css";
import "./SettingsScreen.css";

/** Settings tab: link previews, and Tags (a sub-screen; Android's back button returns here). */
export function SettingsScreen() {
  const [page, setPage] = useState<"main" | "tags">("main");
  const [tagCount, setTagCount] = useState<number | null>(null);
  const [settings, setLocalSettings] = useState<Settings | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    getSettings()
      .then(setLocalSettings)
      .catch((e) => setError(String(e)));
  }, []);

  const togglePreviews = (previews: boolean) => {
    if (!settings) return;
    setLocalSettings({ ...settings, previews });
    setSettings({ ...settings, previews })
      .then(setLocalSettings)
      .catch((e) => {
        setLocalSettings(settings);
        setError(String(e));
      });
  };

  // Back from Tags returns here. A sheet opened inside Tags has its own history entry; closing
  // it lands back on the Tags entry, so stay on Tags then.
  useEffect(() => {
    const onBack = (event: PopStateEvent) =>
      setPage(event.state?.settingsPage === "tags" ? "tags" : "main");
    window.addEventListener("popstate", onBack);
    return () => window.removeEventListener("popstate", onBack);
  }, []);

  useEffect(() => {
    if (page !== "main") return;
    listTags()
      .then((tags) => setTagCount(tags.length))
      .catch(() => setTagCount(null));
  }, [page]);

  const openTags = () => {
    history.pushState({ settingsPage: "tags" }, "");
    setPage("tags");
  };

  if (page === "tags") return <TagsScreen onBack={() => history.back()} />;

  return (
    <main className="screen with-tabs">
      <h1>Settings</h1>
      <section className="settings-section">
        <ul className="settings-list">
          <li>
            <label className="settings-row">
              <span>Link previews</span>
              <input
                type="checkbox"
                role="switch"
                className="settings-switch"
                checked={settings?.previews ?? false}
                disabled={!settings}
                onChange={(event) => togglePreviews(event.target.checked)}
              />
            </label>
          </li>
        </ul>
        <p className="settings-note">
          Shows a picture, description and site for each bookmark. To get them, this phone opens the
          saved link once, like a browser would (without your cookies), and keeps a small picture.
          It's the only thing in Clean Share that goes online.
        </p>
        {error && <p className="settings-note settings-error">{error}</p>}
      </section>

      <ul className="settings-list">
        <li>
          <button type="button" className="settings-row" onClick={openTags}>
            <span>Tags</span>
            <span className="settings-value">
              {tagCount ?? ""} <span aria-hidden="true">›</span>
            </span>
          </button>
        </li>
      </ul>
    </main>
  );
}
