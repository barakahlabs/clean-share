import { useEffect, useState } from "react";
import { shareText } from "../shareTarget";
import { cleanText, type Analysis } from "../linkCleaner";
import { prepareBookmark, saveBookmark } from "../bookmarks";
import shareIcon from "../assets/share.svg";
import "./Screen.css";
import "./SharedTextScreen.css";

/**
 * - `none`: no link in the shared text, so nothing to save
 * - `ready`: a link that isn't saved yet
 * - `saved`: saved now, or it already was
 */
type SaveState = "checking" | "none" | "ready" | "saving" | "saved" | "error";

/**
 * Shows text shared from another app with its tracking highlighted, and shares it cleaned or
 * saves it as a bookmark. (Until the separate "Save bookmark" share entry exists, saving
 * happens here.)
 */
export function SharedTextScreen({ text, onClose }: { text: string; onClose: () => void }) {
  const [analysis, setAnalysis] = useState<Analysis | null>(null);
  const [save, setSave] = useState<SaveState>("checking");
  const [saveError, setSaveError] = useState<string | null>(null);

  useEffect(() => {
    let current = true;
    setAnalysis(null);
    setSave("checking");
    setSaveError(null);
    cleanText(text)
      .then((result) => current && setAnalysis(result))
      .catch(console.error);
    prepareBookmark(text)
      .then((prepared) => {
        if (!current) return;
        if (!prepared.links.length) setSave("none");
        else setSave(prepared.existing ? "saved" : "ready");
      })
      .catch((error) => {
        if (!current) return;
        setSave("error");
        setSaveError(String(error));
      });
    return () => {
      current = false;
    };
  }, [text]);

  const saveLink = () => {
    setSave("saving");
    saveBookmark({ sharedText: text })
      .then(() => setSave("saved"))
      .catch((error) => {
        setSave("error");
        setSaveError(String(error));
      });
  };

  return (
    <main className="screen">
      <header className="screen-header">
        <h1>Clean Share</h1>
        <button type="button" className="text-button" onClick={onClose}>
          Done
        </button>
      </header>
      {analysis ? (
        <section className="shared-card">
          <p className="shared-label">Shared with you</p>
          <p className="shared-text">
            {analysis.segments.map((segment, i) =>
              segment.tracking ? (
                <mark key={i} className="tracking">
                  {segment.text}
                </mark>
              ) : (
                <span key={i}>{segment.text}</span>
              ),
            )}
          </p>
          <p className={analysis.trackingCount ? "tracking-summary found" : "tracking-summary"}>
            {analysis.trackingCount
              ? `${trackingLabel(analysis.trackingCount)} will be removed when you share.`
              : "No tracking found."}
          </p>
        </section>
      ) : (
        <p className="screen-empty">Cleaning…</p>
      )}

      {saveError && <p className="save-error">{saveError}</p>}

      <footer className="share-footer">
        {save !== "none" && (
          <button
            type="button"
            className="share-button secondary"
            disabled={save !== "ready"}
            onClick={saveLink}
          >
            {save === "saved" ? "Saved" : save === "saving" ? "Saving…" : "Save bookmark"}
          </button>
        )}
        <button
          type="button"
          className="share-button"
          disabled={!analysis}
          onClick={() => analysis && shareText(analysis.cleaned).catch(console.error)}
        >
          <img src={shareIcon} alt="" className="share-icon" />
          {analysis?.trackingCount ? "Share clean link" : "Share"}
        </button>
      </footer>
    </main>
  );
}

function trackingLabel(count: number) {
  return count === 1 ? "1 tracking parameter" : `${count} tracking parameters`;
}
