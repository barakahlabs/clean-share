import { hasTracking, type Analysis } from "../linkCleaner";
import "../screens/SharedTextScreen.css";

type Props = { analysis: Analysis; label: string; action?: string };

/**
 * Text with its tracking parameters highlighted (struck through, red) and a one-line summary. A
 * redirect link is struck through whole, followed by the link it leads to.
 */
export function CleanedText({ analysis, label, action = "when you share" }: Props) {
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
            <mark key={i} className="tracking">
              {segment.text}
            </mark>
          ) : (
            <span key={i}>{segment.text}</span>
          ),
        )}
      </p>
      <p className={hasTracking(analysis) ? "tracking-summary found" : "tracking-summary"}>
        {hasTracking(analysis) ? `${summary(analysis)} ${action}.` : "No tracking found."}
      </p>
    </section>
  );
}

function summary({ trackingCount, redirectCount }: Analysis) {
  const params = trackingCount === 1 ? "1 tracking parameter" : `${trackingCount} tracking parameters`;
  const redirects = redirectCount === 1 ? "Redirect" : `${redirectCount} redirects`;
  if (!redirectCount) return `${params} removed`;
  if (!trackingCount) return `${redirects} removed`;
  return `${redirects} and ${params} removed`;
}
