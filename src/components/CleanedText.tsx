import { useState } from "react";
import { hasTracking, type Analysis, type Why } from "../linkCleaner";
import "../screens/SharedTextScreen.css";

type Props = { analysis: Analysis; label: string; action?: string };

/**
 * Text with its tracking parameters highlighted (struck through, red) and a one-line summary. A
 * redirect link is struck through whole, followed by the link it leads to. Tapping a red piece
 * explains why it's removed.
 */
export function CleanedText({ analysis, label, action = "when you share" }: Props) {
  const [picked, setPicked] = useState<{ analysis: Analysis; index: number } | null>(null);
  // A new analysis (the text changed) closes the explanation.
  const index = picked?.analysis === analysis ? picked.index : null;
  const why = index === null ? undefined : analysis.segments[index]?.why;
  const pick = (i: number) => setPicked(i === index ? null : { analysis, index: i });

  return (
    <section className="shared-card">
      <p className="shared-label">{label}</p>
      <p className="shared-text">
        {analysis.segments.map((segment, i) =>
          segment.inserted ? (
            <ins key={i} className="unwrapped">
              {segment.text}
            </ins>
          ) : segment.tracking ? (
            <mark
              key={i}
              className={i === index ? "tracking picked" : "tracking"}
              role="button"
              tabIndex={0}
              aria-expanded={i === index}
              aria-label={`${segment.text}: why it's removed`}
              onClick={() => pick(i)}
              onKeyDown={(event) => {
                if (event.key === "Enter" || event.key === " ") {
                  event.preventDefault();
                  pick(i);
                }
              }}
            >
              {segment.text}
            </mark>
          ) : (
            <span key={i}>{segment.text}</span>
          ),
        )}
      </p>
      {why && <Explanation why={why} />}
      <p className={hasTracking(analysis) ? "tracking-summary found" : "tracking-summary"}>
        {hasTracking(analysis) ? `${summary(analysis)} ${action}.` : "No tracking found."}
        {hasTracking(analysis) && !why && <span className="tracking-hint"> Tap a red part to see why.</span>}
      </p>
    </section>
  );
}

function Explanation({ why }: { why: Why }) {
  const source = why.source ?? "Every site";
  return (
    <div className="tracking-why" role="status">
      <p className="tracking-why-title">
        {why.redirect ? "Redirect" : <code>{why.name}</code>}
        <span className="tracking-why-source"> · {source}</span>
      </p>
      <p className="tracking-why-text">
        {why.reason ?? (why.redirect ? "A page that records the click before sending you on." : "Used for tracking; the link works without it.")}
        {why.redirect && " Replaced by the link it leads to."}
      </p>
    </div>
  );
}

function summary({ trackingCount, redirectCount }: Analysis) {
  const params = trackingCount === 1 ? "1 tracking parameter" : `${trackingCount} tracking parameters`;
  const redirects = redirectCount === 1 ? "Redirect" : `${redirectCount} redirects`;
  if (!redirectCount) return `${params} removed`;
  if (!trackingCount) return `${redirects} removed`;
  return `${redirects} and ${params} removed`;
}
