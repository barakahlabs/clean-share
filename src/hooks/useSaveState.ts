import { useEffect, useState } from "react";
import { prepareBookmark } from "../bookmarks";

/**
 * Whether `text` can be saved as a bookmark:
 * - `none`: no link in it, so nothing to save
 * - `ready`: a link that isn't saved yet
 * - `saved`: the link is already saved (the button edits it)
 * `refresh` re-checks (e.g. after a save).
 */
export type SaveState = "checking" | "none" | "ready" | "saved" | "error";

export function useSaveState(text: string, refresh = 0) {
  const [state, setState] = useState<SaveState>("checking");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let current = true;
    setState("checking");
    setError(null);
    if (!text.trim()) {
      setState("none");
      return;
    }
    prepareBookmark(text)
      .then((prepared) => {
        if (!current) return;
        if (!prepared.links.length) setState("none");
        else setState(prepared.existing ? "saved" : "ready");
      })
      .catch((e) => {
        if (!current) return;
        setState("error");
        setError(String(e));
      });
    return () => {
      current = false;
    };
  }, [text, refresh]);

  return { state, error };
}
