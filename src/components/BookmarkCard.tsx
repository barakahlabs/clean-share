import { useEffect, useState } from "react";
import { thumbnailUrl, type Bookmark } from "../bookmarks";
import { hostOf, titleFromLink } from "../format";
import "./BookmarkCard.css";

type Props = { bookmark: Bookmark; onOpen: () => void };

/** Tags shown on a card; the rest are counted ("+2"). */
const MAX_TAGS = 2;

/**
 * One bookmark in the grid: its picture (or a tile with the site's initial when there's none,
 * e.g. previews off or a site that gives none), title, site and tags.
 */
export function BookmarkCard({ bookmark, onOpen }: Props) {
  const site = bookmark.siteName ?? hostOf(bookmark.url);
  const src = thumbnailUrl(bookmark);
  const [loaded, setLoaded] = useState(false);
  useEffect(() => setLoaded(false), [src]);
  const extraTags = bookmark.tags.length - MAX_TAGS;

  return (
    <button type="button" className="bookmark-card" onClick={onOpen}>
      <span className="bookmark-cover" style={{ "--tile-hue": hueOf(hostOf(bookmark.url)) } as React.CSSProperties}>
        {/* The tile stays behind the picture, so a missing or broken picture still looks right. */}
        <span className="bookmark-tile" aria-hidden="true">
          {initialOf(site)}
        </span>
        {src && (
          <img
            src={src}
            alt=""
            loading="lazy"
            decoding="async"
            className={loaded ? "bookmark-image loaded" : "bookmark-image"}
            onLoad={() => setLoaded(true)}
          />
        )}
      </span>
      <span className="bookmark-body">
        <span className="bookmark-title">{bookmark.title ?? titleFromLink(bookmark.url)}</span>
        <span className="bookmark-meta">{site}</span>
        {bookmark.tags.length > 0 && (
          <span className="bookmark-tags">
            {bookmark.tags.slice(0, MAX_TAGS).map((tag) => (
              <span key={tag.name} className="bookmark-tag">
                {tag.name}
              </span>
            ))}
            {extraTags > 0 && <span className="bookmark-tag more">+{extraTags}</span>}
          </span>
        )}
      </span>
    </button>
  );
}

/** First letter or digit of the site's name, uppercase. */
function initialOf(site: string) {
  const match = site.match(/[\p{L}\p{N}]/u);
  return match ? match[0].toUpperCase() : "•";
}

/** A stable colour per site, so its cards look alike. */
function hueOf(host: string) {
  let hash = 0;
  for (const char of host) hash = (hash * 31 + char.codePointAt(0)!) | 0;
  return Math.abs(hash) % 360;
}
