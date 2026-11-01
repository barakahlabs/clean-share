import { useEffect, useState } from "react";
import { onShared, shareText } from "../shareTarget";
import { cleanText, type Analysis } from "../linkCleaner";
import shareIcon from "../assets/share.svg";
import "./SharedTextScreen.css";

export function SharedTextScreen() {
  const [analysis, setAnalysis] = useState<Analysis | null>(null);

  useEffect(
    () => onShared(({ text }) => cleanText(text).then(setAnalysis).catch(console.error)),
    [],
  );

  return (
    <main className="shared-screen">
      <h1>Clean Share</h1>
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
        <p className="shared-empty">
          Share a link or some text from another app and pick Clean Share. It will show up here.
        </p>
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
