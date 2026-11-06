import { useCallback, useEffect, useRef, useState } from "react";
import { onShared, returnToSource } from "./shareTarget";
import { TabBar, type Tab } from "./components/TabBar";
import { BookmarksScreen } from "./screens/BookmarksScreen";
import { SettingsScreen } from "./screens/SettingsScreen";
import { ShareSheet } from "./screens/ShareSheet";
import "./App.css";

/**
 * The share sheet: what another app shared (`external`), or an empty one from the Clean button
 * (`start: "input"`). `start: "save"` opens on the bookmark form. `id` gives each a fresh sheet.
 */
type Overlay = { id: number; text: string; start: "clean" | "input" | "save"; external: boolean };

/**
 * App shell: bottom tabs, with the share sheet over them when another app shares something. The
 * sheet has a history entry, so Android's back button closes it. Closing a sheet that another app
 * opened goes back to that app (Android).
 */
function App() {
  const [tab, setTab] = useState<Tab>("bookmarks");
  const [overlay, setOverlay] = useState<Overlay | null>(null);
  // Bumped after a save, so the screens behind the sheet show it.
  const [saves, setSaves] = useState(0);
  const overlayRef = useRef(overlay);
  overlayRef.current = overlay;
  const nextId = useRef(0);

  useEffect(() => {
    const onBack = () => {
      const closing = overlayRef.current;
      setOverlay(null);
      if (closing?.external) returnToSource().catch(console.error);
    };
    window.addEventListener("popstate", onBack);
    return () => window.removeEventListener("popstate", onBack);
  }, []);

  const open = useCallback((text: string, start: Overlay["start"], external: boolean) => {
    // A new share replaces whatever is open.
    if (overlayRef.current === null) history.pushState({ overlay: "sheet" }, "");
    setOverlay({ id: nextId.current++, text, start, external });
  }, []);

  useEffect(() => onShared(({ text, intent }) => open(text, intent === "save" ? "save" : "clean", true)), [open]);

  const close = useCallback((saved: boolean) => {
    if (saved) setSaves((n) => n + 1);
    // The history entry's popstate closes the sheet (and returns to the other app).
    history.back();
  }, []);

  // The Clean button: the sheet with a field to paste or type into, over the current tab.
  const clean = useCallback(() => open("", "input", false), [open]);

  const screen = tab === "bookmarks" ? <BookmarksScreen key={saves} /> : <SettingsScreen />;

  return (
    <>
      <div className="app">
        {screen}
        <TabBar current={tab} onChange={setTab} onClean={clean} />
      </div>
      {overlay && <ShareSheet key={overlay.id} text={overlay.text} start={overlay.start} onClose={close} />}
    </>
  );
}

export default App;
