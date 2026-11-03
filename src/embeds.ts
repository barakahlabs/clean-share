import type { Bookmark } from "./bookmarks";

/**
 * A player for a bookmark's link, shown in the detail screen once the user taps Play (nothing is
 * loaded before that). Worked out from the link itself, so it works with previews off too; the
 * preview's embed link (from oEmbed) is the fallback for SoundCloud.
 */
export type Player = {
  /** The iframe's address. */
  src: string;
  /** Who it plays from, for "Plays from YouTube". */
  site: string;
  /** Width / height for video; `null` for a fixed `height` (audio, posts). */
  ratio: number | null;
  height: number;
};

const VIDEO = 16 / 9;

export function playerFor(bookmark: Bookmark): Player | null {
  let url: URL;
  try {
    url = new URL(bookmark.url);
  } catch {
    return null;
  }
  const host = url.hostname.replace(/^(www|m|music|mobile)\./, "");
  const path = url.pathname.split("/").filter(Boolean);

  if (host === "youtube.com" || host === "youtu.be") {
    const id =
      host === "youtu.be"
        ? path[0]
        : url.searchParams.get("v") ?? (["shorts", "live", "embed"].includes(path[0]) ? path[1] : null);
    if (!id || !/^[\w-]{6,}$/.test(id)) return null;
    const shorts = path[0] === "shorts";
    // youtube-nocookie: YouTube's privacy-enhanced player (no cookies until the video plays).
    return {
      src: `https://www.youtube-nocookie.com/embed/${id}?autoplay=1&playsinline=1&rel=0`,
      site: "YouTube",
      ratio: shorts ? 9 / 16 : VIDEO,
      height: 0,
    };
  }
  if (host === "vimeo.com" && /^\d+$/.test(path[0] ?? "")) {
    return { src: `https://player.vimeo.com/video/${path[0]}?autoplay=1&dnt=1`, site: "Vimeo", ratio: VIDEO, height: 0 };
  }
  if (host === "open.spotify.com") {
    // Links can start with a locale: /intl-de/track/…
    const parts = path[0]?.startsWith("intl-") ? path.slice(1) : path;
    const [type, id] = parts;
    if (!["track", "album", "playlist", "episode", "show", "artist"].includes(type) || !id) return null;
    const compact = type === "track" || type === "episode";
    return { src: `https://open.spotify.com/embed/${type}/${id}`, site: "Spotify", ratio: null, height: compact ? 152 : 352 };
  }
  if ((host === "x.com" || host === "twitter.com") && path[1] === "status" && /^\d+$/.test(path[2] ?? "")) {
    return { src: `https://platform.twitter.com/embed/Tweet.html?id=${path[2]}&dnt=true`, site: "X", ratio: null, height: 560 };
  }
  if (host === "tiktok.com" && path[1] === "video" && /^\d+$/.test(path[2] ?? "")) {
    return { src: `https://www.tiktok.com/embed/v2/${path[2]}`, site: "TikTok", ratio: null, height: 740 };
  }
  if (host === "soundcloud.com" && bookmark.embedUrl?.startsWith("https://w.soundcloud.com/")) {
    return { src: bookmark.embedUrl, site: "SoundCloud", ratio: null, height: 166 };
  }
  return null;
}
