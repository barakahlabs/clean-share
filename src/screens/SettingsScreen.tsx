import { useEffect, useState } from "react";
import { listTags, recleanBookmarks, type RecleanReport } from "../bookmarks";
import { getSettings, setSettings, type Settings } from "../settings";
import { TagsScreen } from "./TagsScreen";
import { TransferScreen } from "./TransferScreen";
import "./Screen.css";
import "./SettingsScreen.css";

type Page = "main" | "tags" | "transfer";
const SUB_PAGES: Page[] = ["tags", "transfer"];

/** Settings tab: link previews, plus Tags and Export and import (sub-screens; Android's back
 * button returns here). */
export function SettingsScreen() {
  const [page, setPage] = useState<Page>("main");
  const [tagCount, setTagCount] = useState<number | null>(null);
  const [settings, setLocalSettings] = useState<Settings | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [recleaning, setRecleaning] = useState(false);
  const [reclean, setReclean] = useState<RecleanReport | null>(null);
  const [recleanError, setRecleanError] = useState<string | null>(null);

  const cleanAgain = () => {
    setRecleaning(true);
    setRecleanError(null);
    recleanBookmarks()
      .then(setReclean)
      .catch((e) => setRecleanError(String(e)))
      .finally(() => setRecleaning(false));
  };

  useEffect(() => {
    getSettings()
      .then(setLocalSettings)
      .catch((e) => setError(String(e)));
  }, []);

  const change = (patch: Partial<Settings>) => {
    if (!settings) return;
    setLocalSettings({ ...settings, ...patch });
    setSettings({ ...settings, ...patch })
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
      setPage(SUB_PAGES.includes(event.state?.settingsPage) ? event.state.settingsPage : "main");
    window.addEventListener("popstate", onBack);
    return () => window.removeEventListener("popstate", onBack);
  }, []);

  useEffect(() => {
    if (page !== "main") return;
    listTags()
      .then((tags) => setTagCount(tags.length))
      .catch(() => setTagCount(null));
  }, [page]);

  const openPage = (sub: Exclude<Page, "main">) => {
    history.pushState({ settingsPage: sub }, "");
    setPage(sub);
  };

  if (page === "tags") return <TagsScreen onBack={() => history.back()} />;
  if (page === "transfer") return <TransferScreen onBack={() => history.back()} />;

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
                onChange={(event) => change({ previews: event.target.checked })}
              />
            </label>
          </li>
          {settings?.previews && (
            <li>
              <label className="settings-row">
                <span>Only on Wi-Fi</span>
                <input
                  type="checkbox"
                  role="switch"
                  className="settings-switch"
                  checked={settings.previewsWifiOnly}
                  onChange={(event) => change({ previewsWifiOnly: event.target.checked })}
                />
              </label>
            </li>
          )}
        </ul>
        <p className="settings-note">
          Shows a picture, description and site for each bookmark. To get them, this phone opens the
          saved link once, like a browser would (without your cookies), and keeps a small picture.
          It's the only thing in Clean Share that goes online on its own.
          {settings?.previews && settings.previewsWifiOnly && (
            <> On mobile data, previews wait until the app is next open on Wi-Fi.</>
          )}
        </p>
        {error && <p className="settings-note settings-error">{error}</p>}
      </section>

      <ul className="settings-list">
        <li>
          <button type="button" className="settings-row" onClick={() => openPage("tags")}>
            <span>Tags</span>
            <span className="settings-value">
              {tagCount ?? ""} <span aria-hidden="true">›</span>
            </span>
          </button>
        </li>
        <li>
          <button type="button" className="settings-row" onClick={() => openPage("transfer")}>
            <span>Export and import</span>
            <span className="settings-value" aria-hidden="true">
              ›
            </span>
          </button>
        </li>
      </ul>

      <section className="settings-section">
        <ul className="settings-list">
          <li>
            <button type="button" className="settings-row" disabled={recleaning} onClick={cleanAgain}>
              <span>{recleaning ? "Cleaning…" : "Clean saved links again"}</span>
            </button>
          </li>
        </ul>
        <p className="settings-note">
          {reclean
            ? recleanSummary(reclean)
            : "Runs every saved link through the current tracking rules. Happens by itself after an update that changes the rules; links that turn out to be the same are merged."}
        </p>
        {recleanError && <p className="settings-note settings-error">{recleanError}</p>}
      </section>
    </main>
  );
}

function recleanSummary(r: RecleanReport) {
  if (r.changed === 0) return `Checked ${r.checked} saved ${r.checked === 1 ? "link" : "links"}: all already clean.`;
  const merged = r.merged > 0 ? `, ${r.merged} merged into ${r.merged === 1 ? "a bookmark" : "bookmarks"} you already had` : "";
  return `Checked ${r.checked}: ${r.changed} cleaned again${merged}.`;
}
