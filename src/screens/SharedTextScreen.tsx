import { useEffect, useState } from "react";
import { shareText } from "../shareTarget";
import { cleanText, type Analysis } from "../linkCleaner";
import shareIcon from "../assets/share.svg";
import "./Screen.css";
import "./SharedTextScreen.css";

/** Shows text shared from another app with its tracking highlighted, and shares it cleaned. */
export function SharedTextScreen({ text, onClose }: { text: string; onClose: () => void }) {
  const [analysis, setAnalysis] = useState<Analysis | null>(null);

  useEffect(() => {
    let current = true;
    setAnalysis(null);
    cleanText(text)
      .then((result) => current && setAnalysis(result))
      .catch(console.error);
    return () => {
      current = false;
    };
  }, [text]);

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

      <footer className="share-footer">
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
