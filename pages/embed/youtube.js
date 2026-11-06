// Plays the video named after # (#v=<id>) in YouTube's privacy-enhanced player. Anything that
// isn't a YouTube video ID is ignored.
const id = new URLSearchParams(location.hash.slice(1)).get("v");
if (id && /^[\w-]{6,20}$/.test(id)) {
  const player = document.createElement("iframe");
  player.src = `https://www.youtube-nocookie.com/embed/${id}?autoplay=1&playsinline=1&rel=0`;
  player.title = "YouTube video";
  player.allow = "autoplay; encrypted-media; picture-in-picture; fullscreen";
  player.allowFullscreen = true;
  player.referrerPolicy = "strict-origin-when-cross-origin";
  document.body.append(player);
}
