# link-cleaner

Finds tracking parameters in links inside shared text and removes them. Used by the app's
`clean_text` command to highlight tracking in red and share the cleaned link.

- Works on every `http://` / `https://` link in the text; the text around links is kept as is.
- Only query parameters (`?name=value&…`) are removed. The path and `#fragment` are kept.
- Parameters that aren't tracking are kept in their original order and spelling.
- If nothing is left after `?`, the `?` is removed too.
- Names are matched ignoring case. `utm_*` means "any name starting with `utm_`".

The rules live in [`src/rules.rs`](src/rules.rs). A test checks that every rule there is listed
in this README, so update both together.

## Removed on every site

| Parameter | What it is |
|---|---|
| `utm_*` | Campaign tags (`utm_source`, `utm_medium`, `utm_campaign`, …) |
| `fbclid` | Facebook / Instagram ad click ID |
| `gclid`, `gclsrc`, `dclid`, `gbraid`, `wbraid` | Google Ads click IDs |
| `msclkid` | Microsoft Ads click ID |
| `yclid` | Yandex Ads click ID |
| `twclid` | X (Twitter) Ads click ID |
| `ttclid` | TikTok Ads click ID |
| `li_fat_id` | LinkedIn Ads click ID |
| `rb_clickid` | Rakuten click ID |
| `s_kwcid` | Adobe Analytics keyword ID |
| `_ga`, `_gl` | Google Analytics cross-site linker |
| `_hsenc`, `_hsmi` | HubSpot email tracking |
| `mc_cid`, `mc_eid` | Mailchimp campaign / subscriber ID |
| `mkt_tok` | Marketo email tracking |
| `oly_anon_id`, `oly_enc_id` | Omeda email tracking |
| `vero_id` | Vero email tracking |
| `igshid`, `igsh` | Instagram share ID (identifies who shared) |

## Removed on specific sites

Site rules apply to the listed domains and their subdomains (`m.youtube.com`,
`music.youtube.com`, …). The same name can mean different things on different sites, which is
why `t` is removed on X but kept on YouTube.

| Platform | Domains | Removed | Kept (examples) |
|---|---|---|---|
| Instagram | `instagram.com` | `igsh`, `igshid`, `utm_source` (via the every-site rules) | `img_index` (carousel position) |
| YouTube | `youtube.com`, `youtu.be` | `si` (who shared), `feature`, `pp` | `v` (video), `t` (start time), `list` (playlist) |
| X / Twitter | `x.com`, `twitter.com` | `s` (share source), `a` (share token, newer), `t` (share token, older), `ref_src`, `ref_url` | |
| Threads | `threads.net`, `threads.com` | `xmt`, `slof` | |
| TikTok | `tiktok.com` | `_r`, `_t`, `is_from_webapp`, `is_copy_url`, `sender_device`, `sender_web_id`, `share_app_id`, `share_item_id`, `share_link_id`, `social_share_type`, `social_sharing`, `tt_from`, `u_code`, `user_id`, `sec_user_id`, `checksum`, `timestamp`, `enter_from`, `source`, `web_id`, `preview_pb` | |
| Facebook | `facebook.com`, `fb.watch` | `mibextid`, `rdid`, `share_url`, `sfnsn`, `__cft__*`, `__tn__` | `story_fbid`, `id` |
| Spotify | `open.spotify.com` | `si` (who shared), `nd`, `dlsi` | |
| LinkedIn | `linkedin.com` | `trk`, `trackingId`, `lipi`, `midToken`, `midSig`, `trkEmail`, `eid`, `rcm` | |
| Reddit | `reddit.com` | `share_id`, `rdt`, `ref`, `ref_source`, `ref_campaign` | `context` (comment thread) |

## Examples

| Platform | Shared link | Cleaned link |
|---|---|---|
| Instagram | `https://www.instagram.com/reel/C1a2B3c4D5e/?igsh=MWQ1ZGUxMzBkMA==` | `https://www.instagram.com/reel/C1a2B3c4D5e/` |
| YouTube | `https://youtu.be/dQw4w9WgXcQ?si=AbCdEf123&t=42` | `https://youtu.be/dQw4w9WgXcQ?t=42` |
| X (newer) | `https://x.com/user/status/2115652166325338163?a=uqa_a0BpGpaE4mGJaul&s=19` | `https://x.com/user/status/2115652166325338163` |
| X (older) | `https://x.com/user/status/1234567890?s=46&t=AbC-dEf_123` | `https://x.com/user/status/1234567890` |
| TikTok | `https://www.tiktok.com/@user/video/123?_r=1&_t=ZM-abc&is_from_webapp=1` | `https://www.tiktok.com/@user/video/123` |
| Spotify | `https://open.spotify.com/track/4uLU6hMCjMI75M1A2tKUQC?si=a1b2c3d4` | `https://open.spotify.com/track/4uLU6hMCjMI75M1A2tKUQC` |
| Any site | `https://example.com/a?utm_source=x&id=7&page=2#top` | `https://example.com/a?id=7&page=2#top` |

## Adding a rule

1. Add the parameter to `GLOBAL` (every site) or to a `SiteRule` in `src/rules.rs`.
2. Add it to the tables above.
3. Add a test in `src/lib.rs` with a real shared link, then run `cargo test`.
