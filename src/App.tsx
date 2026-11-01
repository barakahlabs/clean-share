import { useCallback, useEffect, useState } from "react";
import { onShared } from "./shareTarget";
import { TabBar, type Tab } from "./components/TabBar";
import { BookmarksScreen } from "./screens/BookmarksScreen";
import { SaveBookmarkScreen } from "./screens/SaveBookmarkScreen";
import { SettingsScreen } from "./screens/SettingsScreen";
import { SharedTextScreen } from "./screens/SharedTextScreen";
import "./App.css";

/** What another app shared, and which screen it's open on. */
type Overlay = { text: string; page: "share" | "save" };

/**
 * App shell: bottom tabs, with screens for a share opening on top of them. A "Clean & Share"
 * share opens the share screen (which can go on to the save screen); a "Save bookmark" share
 * opens the save screen directly. Each open screen has a history entry, so Android's back
 * button steps back through them to the tab the user was on.
 */
function App() {
  const [tab, setTab] = useState<Tab>("bookmarks");
  const [overlay, setOverlay] = useState<Overlay | null>(null);

  useEffect(() => {
    const onBack = (event: PopStateEvent) => {
      const page = event.state?.overlay;
      setOverlay((open) => (open && (page === "share" || page === "save") ? { ...open, page } : null));
    };
    window.addEventListener("popstate", onBack);
    return () => window.removeEventListener("popstate", onBack);
  }, []);

  useEffect(
    () =>
      onShared(({ text, intent }) => {
        const page = intent === "save" ? "save" : "share";
        setOverlay((open) => {
          // A new share replaces whatever share screen is open.
          if (open === null) history.pushState({ overlay: page }, "");
          else history.replaceState({ overlay: page }, "");
          return { text, page };
        });
      }),
    [],
  );

  const back = useCallback(() => history.back(), []);

  const openSave = useCallback(() => {
    history.pushState({ overlay: "save" }, "");
    setOverlay((open) => open && { ...open, page: "save" });
  }, []);

  if (overlay?.page === "share") {
    return <SharedTextScreen text={overlay.text} onClose={back} onSave={openSave} />;
  }
  if (overlay?.page === "save") {
    return <SaveBookmarkScreen text={overlay.text} onDone={back} />;
  }

  return (
    <div className="app">
      {tab === "bookmarks" ? <BookmarksScreen /> : <SettingsScreen />}
      <TabBar current={tab} onChange={setTab} />
    </div>
  );
}

export default App;
