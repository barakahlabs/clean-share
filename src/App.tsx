import { useCallback, useEffect, useState } from "react";
import { onShared } from "./shareTarget";
import { TabBar, type Tab } from "./components/TabBar";
import { BookmarksScreen } from "./screens/BookmarksScreen";
import { SettingsScreen } from "./screens/SettingsScreen";
import { SharedTextScreen } from "./screens/SharedTextScreen";
import "./App.css";

/**
 * App shell: bottom tabs, with the share screen opening on top whenever another app shares
 * something. Closing it (Done, or Android's back button) returns to the tab the user was on.
 */
function App() {
  const [tab, setTab] = useState<Tab>("bookmarks");
  const [sharedText, setSharedText] = useState<string | null>(null);

  // A history entry per open share screen, so Android's back button closes it.
  useEffect(() => {
    const onBack = () => setSharedText(null);
    window.addEventListener("popstate", onBack);
    return () => window.removeEventListener("popstate", onBack);
  }, []);

  useEffect(
    () =>
      // "save" shares get their own screen in phase 1; until then both open the share screen.
      onShared(({ text }) => {
        setSharedText((open) => {
          if (open === null) history.pushState({ shareScreen: true }, "");
          return text;
        });
      }),
    [],
  );

  const closeShared = useCallback(() => history.back(), []);

  if (sharedText !== null) {
    return <SharedTextScreen text={sharedText} onClose={closeShared} />;
  }

  return (
    <div className="app">
      {tab === "bookmarks" ? <BookmarksScreen /> : <SettingsScreen />}
      <TabBar current={tab} onChange={setTab} />
    </div>
  );
}

export default App;
