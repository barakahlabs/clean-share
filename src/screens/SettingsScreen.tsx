import { useEffect, useState } from "react";
import { listTags } from "../bookmarks";
import { TagsScreen } from "./TagsScreen";
import "./Screen.css";
import "./SettingsScreen.css";

/** Settings tab. Sub-screens (Tags) open inside it; Android's back button returns here. */
export function SettingsScreen() {
  const [page, setPage] = useState<"main" | "tags">("main");
  const [tagCount, setTagCount] = useState<number | null>(null);

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
