import { useLayoutEffect, useRef, useState } from "react";
import "./ClampedText.css";

type Props = { text: string; lines: number; className?: string };

/**
 * Text cut to `lines` lines, with "Show more" / "Show less" only when it's actually longer
 * (measured after layout, and again when the width changes, e.g. on rotation).
 */
export function ClampedText({ text, lines, className }: Props) {
  const ref = useRef<HTMLParagraphElement>(null);
  const [expanded, setExpanded] = useState(false);
  const [overflows, setOverflows] = useState(false);

  useLayoutEffect(() => {
    setExpanded(false);
  }, [text]);

  useLayoutEffect(() => {
    const el = ref.current;
    if (!el) return;
    const measure = () => {
      // Only measurable while clamped; once expanded, it overflowed by definition.
      if (!expanded) setOverflows(el.scrollHeight > el.clientHeight + 1);
    };
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(el);
    return () => observer.disconnect();
  }, [text, lines, expanded]);

  return (
    <div className="clamped">
      <p
        ref={ref}
        className={className}
        style={expanded ? undefined : { WebkitLineClamp: lines, display: "-webkit-box", WebkitBoxOrient: "vertical", overflow: "hidden" }}
      >
        {text}
      </p>
      {(overflows || expanded) && (
        <button type="button" className="text-button clamped-toggle" aria-expanded={expanded} onClick={() => setExpanded(!expanded)}>
          {expanded ? "Show less" : "Show more"}
        </button>
      )}
    </div>
  );
}
