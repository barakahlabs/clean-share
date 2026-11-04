import { useEffect, useState } from "react";
import { copyText } from "../clipboard";

type Props = { text: string | null; label?: string; className?: string };

/** Copies `text`, then says "Copied" for a moment. */
export function CopyButton({ text, label = "Copy", className = "share-button secondary" }: Props) {
  const [copied, setCopied] = useState(false);
  useEffect(() => {
    if (!copied) return;
    const timer = setTimeout(() => setCopied(false), 1500);
    return () => clearTimeout(timer);
  }, [copied]);

  return (
    <button
      type="button"
      className={className}
      disabled={!text}
      onClick={() =>
        text &&
        copyText(text)
          .then(() => setCopied(true))
          .catch(console.error)
      }
    >
      {copied ? "Copied" : label}
    </button>
  );
}
