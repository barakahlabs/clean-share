# Tracking rules

These files decide which link parameters Clean Share treats as tracking. They are embedded in
the app at build time and will also be published as a hosted `rules.json`, so rules can be
updated without an app release.

| File | What it holds | Schema |
|---|---|---|
| [`manifest.toml`](manifest.toml) | Format version and rules revision | [`schema/manifest.json`](schema/manifest.json) |
| [`global.toml`](global.toml) | Parameters removed on every site | [`schema/global.json`](schema/global.json) |
| [`providers/<id>.toml`](providers) | One file per service | [`schema/provider.json`](schema/provider.json) |

The schemas are generated from the Rust types in `crates/link-rules`, which are the source of
truth. Editors with TOML schema support (e.g. VS Code with *Even Better TOML*) pick them up from
the `#:schema` line at the top of each file.

## How matching works

- Only query parameters (`?name=value&…`) are removed. The path and `#fragment` are kept.
- Names match ignoring case. A trailing `*` matches by prefix: `utm_*` covers `utm_source`.
- A provider applies to its domains and their subdomains (`youtube.com` covers
  `music.youtube.com`), so the same name can be tracking on one site and not another: `t` is a
  share token on X but the start time on YouTube.
- `keep` always wins, even over every-site rules. `except` paths skip the provider's own rules.
- If nothing is left after `?`, the `?` is removed too.

## Changing rules

1. Edit or add a file. A new service gets `providers/<id>.toml` with `id = "<id>"`.
2. Add a `[[test]]` with a real shared link and the expected cleaned link.
3. Bump `revision` in `manifest.toml`.
4. From `crates/rules-tool`: `cargo run -- generate` (updates the schemas and the tables below),
   then `cargo run -- check`.

A rule file that doesn't match the schema, a domain claimed by two providers, a parameter in
both `keep` and `remove`, or a failing test fails `check` and the app build.

## License

The rules in this directory are dedicated to the public domain under
[CC0 1.0](LICENSE): use, copy and adapt them for any purpose, no attribution needed. The code
that reads them (`crates/`) is MIT licensed, see the [root LICENSE](../LICENSE).

## References

- [ClearURLs Rules](https://github.com/ClearURLs/Rules): the community rule list behind the
  ClearURLs browser extension. Rules for 200+ sites in one file,
  [`data.min.json`](https://raw.githubusercontent.com/ClearURLs/Rules/master/data.min.json).
  Useful for finding parameters when adding a new provider. Our format borrows its ideas
  (per-provider rules, exceptions) but uses domain lists and simple patterns instead of
  regexes, and requires tests for every provider.
  - Updated infrequently (as of October 2026 the last commit was March 2026), so check rules
    against real shared links. For example, its Twitter rule doesn't cover `x.com` or the
    newer `a` share token.
  - Licensed LGPL-3.0: use it as a reference, don't copy its data files into this repo.

## Current rules

<!-- BEGIN GENERATED: edit the TOML files, then run `cargo run -- generate` in crates/rules-tool -->

Rules revision `2026-10-31.2`.

### Overview

| Platform | Domains | Removed | Always kept | Not applied on |
|---|---|---|---|---|
| [Facebook](providers/facebook.toml) | `facebook.com`, `fb.watch` | `mibextid`, `rdid`, `share_url`, `sfnsn`, `__cft__*`, `__tn__` | `story_fbid`, `id`, `v` | – |
| [Instagram](providers/instagram.toml) | `instagram.com` | everything not kept, incl. `srtk`, `psln`, `obrf` | `img_index` | – |
| [LinkedIn](providers/linkedin.toml) | `linkedin.com` | `trk`, `trackingId`, `lipi`, `midToken`, `midSig`, `trkEmail`, `eid`, `rcm` | – | – |
| [Reddit](providers/reddit.toml) | `reddit.com` | `share_id`, `rdt`, `ref`, `ref_source`, `ref_campaign` | `context` | – |
| [Spotify](providers/spotify.toml) | `open.spotify.com` | `si`, `nd`, `dlsi` | – | – |
| [Threads](providers/threads.toml) | `threads.net`, `threads.com` | `xmt`, `slof` | – | – |
| [TikTok](providers/tiktok.toml) | `tiktok.com` | `_r`, `_t`, `is_from_webapp`, `is_copy_url`, `sender_device`, `sender_web_id`, `share_app_id`, `share_item_id`, `share_link_id`, `social_share_type`, `social_sharing`, `tt_from`, `u_code`, `user_id`, `sec_user_id`, `checksum`, `timestamp`, `enter_from`, `source`, `web_id`, `preview_pb` | – | – |
| [X / Twitter](providers/x.toml) | `x.com`, `twitter.com` | `a`, `t`, `s`, `ref_src`, `ref_url` | – | `/i/redirect` |
| [YouTube](providers/youtube.toml) | `youtube.com`, `youtu.be` | `si`, `is`, `feature`, `pp` | `v`, `t`, `list`, `index` | – |

### Removed on every site ([global.toml](global.toml))

| Parameter | Why it's removed |
|---|---|
| `utm_*` | Campaign tags: where the link was shared and which campaign it belongs to |
| `fbclid` | Facebook / Instagram ad click ID |
| `gclid` | Google Ads click ID |
| `gclsrc` | Google Ads click source |
| `dclid` | Google Display Ads click ID |
| `gbraid` | Google Ads click ID (iOS app campaigns) |
| `wbraid` | Google Ads click ID (iOS web campaigns) |
| `msclkid` | Microsoft Ads click ID |
| `yclid` | Yandex Ads click ID |
| `twclid` | X (Twitter) Ads click ID |
| `ttclid` | TikTok Ads click ID |
| `li_fat_id` | LinkedIn Ads click ID |
| `rb_clickid` | Rakuten click ID |
| `s_kwcid` | Adobe Analytics keyword ID |
| `_ga` | Google Analytics cross-site visitor ID |
| `_gl` | Google Analytics cross-site linker |
| `_hsenc` | HubSpot email tracking |
| `_hsmi` | HubSpot email tracking |
| `mc_cid` | Mailchimp campaign ID |
| `mc_eid` | Mailchimp subscriber ID |
| `mkt_tok` | Marketo email tracking |
| `oly_anon_id` | Omeda email tracking |
| `oly_enc_id` | Omeda email tracking |
| `vero_id` | Vero email tracking |
| `igshid` | Instagram share ID: identifies who shared the link |
| `igsh` | Instagram share ID: identifies who shared the link |

### Facebook ([providers/facebook.toml](providers/facebook.toml))

| Parameter | Why it's removed |
|---|---|
| `mibextid` | Share tracking ID from the mobile app |
| `rdid` | Redirect tracking ID |
| `share_url` | Original share link, used for tracking |
| `sfnsn` | Share source flag |
| `__cft__*` | Click tracking token |
| `__tn__` | Click tracking flag |

### Instagram ([providers/instagram.toml](providers/instagram.toml))

Removes **every** parameter except `img_index` (`strip_unknown = true`), so new tracking names are covered automatically. Known ones:

| Parameter | Why it's removed |
|---|---|
| `srtk` | Share tracking token |
| `psln` | Share tracking token |
| `obrf` | Share tracking token |

### LinkedIn ([providers/linkedin.toml](providers/linkedin.toml))

| Parameter | Why it's removed |
|---|---|
| `trk` | Where the click came from |
| `trackingId` | Click tracking ID |
| `lipi` | Page impression ID |
| `midToken` | Email tracking token |
| `midSig` | Email tracking signature |
| `trkEmail` | Email tracking |
| `eid` | Email tracking ID |
| `rcm` | Recommendation tracking |

### Reddit ([providers/reddit.toml](providers/reddit.toml))

| Parameter | Why it's removed |
|---|---|
| `share_id` | Share ID: identifies who shared the link |
| `rdt` | Ad click ID |
| `ref` | Where the link was shared from |
| `ref_source` | Where the link was shared from |
| `ref_campaign` | Share campaign |

### Spotify ([providers/spotify.toml](providers/spotify.toml))

| Parameter | Why it's removed |
|---|---|
| `si` | Share ID: identifies who shared the link |
| `nd` | Share tracking flag |
| `dlsi` | Deep link share ID |

### Threads ([providers/threads.toml](providers/threads.toml))

| Parameter | Why it's removed |
|---|---|
| `xmt` | Share tracking token |
| `slof` | Share source flag |

### TikTok ([providers/tiktok.toml](providers/tiktok.toml))

| Parameter | Why it's removed |
|---|---|
| `_r` | Share tracking flag |
| `_t` | Share tracking token |
| `is_from_webapp` | Whether the link came from the web app |
| `is_copy_url` | Whether the link was copied |
| `sender_device` | Device type of the person who shared |
| `sender_web_id` | Web ID of the person who shared |
| `share_app_id` | App the link was shared from |
| `share_item_id` | Shared item tracking ID |
| `share_link_id` | Unique ID for this share |
| `social_share_type` | How the link was shared |
| `social_sharing` | Share tracking flag |
| `tt_from` | Where the share came from |
| `u_code` | User code of the person who shared |
| `user_id` | ID of the person who shared |
| `sec_user_id` | Encrypted ID of the person who shared |
| `checksum` | Share tracking checksum |
| `timestamp` | When the link was shared |
| `enter_from` | Where in TikTok the share started |
| `source` | Share source |
| `web_id` | Web visitor ID |
| `preview_pb` | Preview tracking flag |

### X / Twitter ([providers/x.toml](providers/x.toml))

| Parameter | Why it's removed |
|---|---|
| `a` | Share token (newer format): identifies who shared the link |
| `t` | Share token (older format): identifies who shared the link |
| `s` | Which app or platform the link was shared from |
| `ref_src` | Referral source on embedded posts |
| `ref_url` | Page the embedded post was shown on |

### YouTube ([providers/youtube.toml](providers/youtube.toml))

| Parameter | Why it's removed |
|---|---|
| `si` | Share ID: identifies who shared the link |
| `is` | Share ID (newer format): identifies who shared the link |
| `feature` | Where in YouTube the link was shared from |
| `pp` | Search and recommendation tracking |

### Examples (from the rule tests)

| Platform | Shared link | Cleaned link |
|---|---|---|
| Facebook | `https://www.facebook.com/share/p/abc123/?mibextid=WC7FNe` | `https://www.facebook.com/share/p/abc123/` |
| Facebook | `https://www.facebook.com/permalink.php?story_fbid=1&id=2&__cft__[0]=AZX&__tn__=%2CO` | `https://www.facebook.com/permalink.php?story_fbid=1&id=2` |
| Instagram | `https://www.instagram.com/reel/C1a2B3c4D5e/?igsh=MWQ1ZGUxMzBkMA==` | `https://www.instagram.com/reel/C1a2B3c4D5e/` |
| Instagram | `https://www.instagram.com/p/abc/?utm_source=ig_web_copy_link&igshid=xyz` | `https://www.instagram.com/p/abc/` |
| Instagram | `https://www.instagram.com/reel/C1a2B3c4D5e/?srtk=aB3dEf9GhIj` | `https://www.instagram.com/reel/C1a2B3c4D5e/` |
| Instagram | `https://www.instagram.com/reel/C1a2B3c4D5e/?psln=Q2xvdWQ&obrf=7Hk2` | `https://www.instagram.com/reel/C1a2B3c4D5e/` |
| Instagram | `https://www.instagram.com/p/abc/?img_index=2&igsh=xyz` | `https://www.instagram.com/p/abc/?img_index=2` |
| LinkedIn | `https://www.linkedin.com/posts/someone_activity-123?utm_source=share&rcm=ACoAA&trk=public_post` | `https://www.linkedin.com/posts/someone_activity-123` |
| Reddit | `https://www.reddit.com/r/rust/comments/abc/title/?share_id=Xyz&utm_medium=android_app&utm_source=share` | `https://www.reddit.com/r/rust/comments/abc/title/` |
| Reddit | `https://www.reddit.com/r/rust/comments/abc/title/def/?context=3&ref=share` | `https://www.reddit.com/r/rust/comments/abc/title/def/?context=3` |
| Spotify | `https://open.spotify.com/track/4uLU6hMCjMI75M1A2tKUQC?si=a1b2c3d4` | `https://open.spotify.com/track/4uLU6hMCjMI75M1A2tKUQC` |
| Threads | `https://www.threads.net/@user/post/C9abc?xmt=AQGz&slof=1` | `https://www.threads.net/@user/post/C9abc` |
| TikTok | `https://www.tiktok.com/@user/video/123?_r=1&_t=ZM-abc&is_from_webapp=1` | `https://www.tiktok.com/@user/video/123` |
| X / Twitter | `https://x.com/leviackermanft/status/2115652166325338163?a=uqa_a0BpGpaE4mGJaul&s=19` | `https://x.com/leviackermanft/status/2115652166325338163` |
| X / Twitter | `https://x.com/someone/status/1234567890?s=46&t=AbC-dEf_123` | `https://x.com/someone/status/1234567890` |
| X / Twitter | `https://twitter.com/someone/status/1?ref_src=twsrc%5Etfw` | `https://twitter.com/someone/status/1` |
| YouTube | `https://youtu.be/dQw4w9WgXcQ?si=AbCdEf123&t=42` | `https://youtu.be/dQw4w9WgXcQ?t=42` |
| YouTube | `https://www.youtube.com/watch?v=dQw4w9WgXcQ&si=xyz&feature=shared` | `https://www.youtube.com/watch?v=dQw4w9WgXcQ` |
| YouTube | `https://youtu.be/dQw4w9WgXcQ?is=Xk3pQ9rT2mLw&t=15` | `https://youtu.be/dQw4w9WgXcQ?t=15` |
| YouTube | `https://music.youtube.com/watch?v=abc&list=PL123&si=q` | `https://music.youtube.com/watch?v=abc&list=PL123` |
| Any site | `https://example.com/a?utm_source=x&id=7&utm_medium=y&page=2#top` | `https://example.com/a?id=7&page=2#top` |
| Any site | `https://example.com/?fbclid=IwAR0abc&gclid=Cj0K` | `https://example.com/` |
| Any site | `https://example.com/page?t=5` | `https://example.com/page?t=5` |

<!-- END GENERATED -->
