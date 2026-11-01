import { useCallback, useEffect, useRef, useState } from "react";
import { onShared, returnToSource } from "./shareTarget";
import { TabBar, type Tab } from "./components/TabBar";
import { BookmarksScreen } from "./screens/BookmarksScreen";
import { SaveBookmarkSheet } from "./screens/SaveBookmarkSheet";
import { SettingsScreen } from "./screens/SettingsScreen";
import { SharedTextScreen } from "./screens/SharedTextScreen";
import "./App.css";

/**
 * What another app shared. `share` shows the share screen; `saving` shows the save sheet (over
 * the share screen, or over the tabs for a "Save bookmark" share).
 */
type Overlay = { text: string; share: boolean; saving: boolean };

/**
 * App shell: bottom tabs, with the share screen and save sheet opening on top when another app
 * shares something. Each has a history entry, so Android's back button closes the sheet, then
 * the share screen, then returns to the tab the user was on.
 */
function App() {
  const [tab, setTab] = useState<Tab>("bookmarks");
  const [overlay, setOverlay] = useState<Overlay | null>(null);
  // Bumped after a save, so the share screen behind the sheet re-checks the link ("Edit bookmark").
  const [saves, setSaves] = useState(0);
  const overlayRef = useRef(overlay);
  overlayRef.current = overlay;

  useEffect(() => {
    const onBack = (event: PopStateEvent) => {
      const page = event.state?.overlay;
      setOverlay((open) => {
        if (!open) return null;
        if (page === "save") return { ...open, saving: true };
        if (page === "share" && open.share) return { ...open, saving: false };
        return null;
      });
    };
    window.addEventListener("popstate", onBack);
    return () => window.removeEventListener("popstate", onBack);
  }, []);

  useEffect(
    () =>
      onShared(({ text, intent }) => {
        const saving = intent === "save";
        setOverlay((open) => {
          // A new share replaces whatever is open.
          if (open === null) history.pushState({ overlay: saving ? "save" : "share" }, "");
          else history.replaceState({ overlay: saving ? "save" : "share" }, "");
          return { text, share: !saving, saving };
        });
      }),
    [],
  );

  const back = useCallback(() => history.back(), []);

  const openSave = useCallback(() => {
    history.pushState({ overlay: "save" }, "");
    setOverlay((open) => open && { ...open, saving: true });
  }, []);

  const closeSave = useCallback((saved: boolean) => {
    if (saved) setSaves((n) => n + 1);
    // Straight from another app's "Save bookmark": go back to that app (Android).
    const fromSaveShare = overlayRef.current?.share === false;
    history.back();
    if (fromSaveShare) returnToSource().catch(console.error);
  }, []);

  const base =
    overlay?.share ? (
      <SharedTextScreen key={saves} text={overlay.text} onClose={back} onSave={openSave} />
    ) : (
      <div className="app">
        {tab === "bookmarks" ? <BookmarksScreen key={saves} /> : <SettingsScreen />}
        <TabBar current={tab} onChange={setTab} />
      </div>
    );

  return (
    <>
      {base}
      {overlay?.saving && <SaveBookmarkSheet text={overlay.text} onDone={closeSave} />}
    </>
  );
}

export default App;
