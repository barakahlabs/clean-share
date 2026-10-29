import { useEffect, useState } from "react";
import { onSharedText, shareText } from "../shareTarget";
import "./SharedTextScreen.css";

export function SharedTextScreen() {
  const [text, setText] = useState<string | null>(null);

  useEffect(() => onSharedText(setText), []);

  return (
    <main className="shared-screen">
      <h1>Clean Share</h1>
      {text ? (
        <section className="shared-card">
          <p className="shared-label">Shared with you</p>
          <p className="shared-text">{text}</p>
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
          disabled={!text}
          onClick={() => text && shareText(text).catch(console.error)}
        >
          Share
        </button>
      </footer>
    </main>
  );
}
