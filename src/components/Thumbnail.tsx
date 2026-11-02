import { useEffect, useState } from "react";

type Props = { src: string | null; className?: string; alt?: string };

/** A bookmark's thumbnail from the phone. Shows nothing if there's none or it can't load. */
export function Thumbnail({ src, className, alt = "" }: Props) {
  const [failed, setFailed] = useState(false);
  useEffect(() => setFailed(false), [src]);
  if (!src || failed) return null;
  return <img src={src} className={className} alt={alt} loading="lazy" decoding="async" onError={() => setFailed(true)} />;
}
