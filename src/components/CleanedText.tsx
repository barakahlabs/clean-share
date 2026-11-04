import type { Analysis } from "../linkCleaner";
import "../screens/SharedTextScreen.css";

type Props = { analysis: Analysis; label: string; action?: string };

/** Text with its tracking parameters highlighted (struck through, red) and a one-line summary. */
export function CleanedText({ analysis, label, action = "when you share" }: Props) {
  return (
    <section className="shared-card">
      <p className="shared-label">{label}</p>
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
        {analysis.trackingCount ? `${trackingLabel(analysis.trackingCount)} removed ${action}.` : "No tracking found."}
      </p>
    </section>
  );
}

function trackingLabel(count: number) {
  return count === 1 ? "1 tracking parameter" : `${count} tracking parameters`;
}
