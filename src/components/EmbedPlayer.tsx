import { useEffect, useState } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import type { Player } from "../embeds";
import "./EmbedPlayer.css";

type Props = { player: Player; poster: string | null; title: string; link: string };

/**
 * The bookmark's player in the detail card: first its picture with a play button (nothing is
 * loaded from the site yet), then, once tapped, the site's own player in a sandboxed iframe: it
 * can run and go fullscreen, but can't navigate the app. Players that can refuse a video
 * (`openInApp`) also get an "Open in YouTube" link under them.
 */
export function EmbedPlayer({ player, poster, title, link }: Props) {
  const [playing, setPlaying] = useState(false);
  const [posterFailed, setPosterFailed] = useState(false);
  useEffect(() => {
    setPlaying(false);
    setPosterFailed(false);
  }, [player.src, poster]);

  const size = player.ratio ? { aspectRatio: String(player.ratio) } : { height: player.height };

  if (playing) {
    return (
      <>
        <div className="embed-frame" style={size}>
          <iframe
            src={player.src}
            title={title}
            allow="autoplay; encrypted-media; picture-in-picture; fullscreen; clipboard-write"
            allowFullScreen
            referrerPolicy="strict-origin-when-cross-origin"
            sandbox="allow-scripts allow-same-origin allow-presentation allow-popups allow-popups-to-escape-sandbox"
          />
        </div>
        {player.openInApp && (
          <button type="button" className="embed-open" onClick={() => openUrl(link).catch(console.error)}>
            Not playing? Open in {player.site} ›
          </button>
        )}
      </>
    );
  }

  return (
    <button
      type="button"
      className="embed-poster"
      onClick={() => setPlaying(true)}
      aria-label={`Play ${title}`}
    >
      {poster && !posterFailed && <img src={poster} alt="" onError={() => setPosterFailed(true)} />}
      <span className="embed-play" aria-hidden="true">
        <svg viewBox="0 0 24 24" width="28" height="28">
          <path d="M8 5.5v13l11-6.5z" fill="currentColor" />
        </svg>
      </span>
      <span className="embed-from">Plays from {player.site}</span>
    </button>
  );
}
