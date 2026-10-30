# Clean Share roadmap

Clean Share receives links shared from other apps, highlights tracking parameters in red, and
shares the cleaned link onwards. This file lists what's done and the possible next steps, roughly
in priority order within each section.

## Where things stand (October 2026)

| Area | State |
|---|---|
| App shell | Tauri 2 + React (TypeScript), pnpm via mise |
| Receive shares | Android: tested on emulator. iOS: Share Extension builds, **not yet run** |
| Share onwards | Android: tested. iOS: builds, not yet run |
| Tracking removal | Rust (`crates/link-cleaner`), rules in typed TOML files (`rules/`), 9 platforms, 20 rule tests |
| Rule tooling | `crates/rules-tool`: `check`, `generate` (schemas + README tables), `bundle` |
| Branches | `tauri-scaffold` → `share-target` → `link-cleaner` → `rules-schema` (stacked, not pushed) |

## 1. Housekeeping (next up)

- [ ] Commit pending work: ClearURLs reference in `rules/README.md` (→ `rules-schema`), share icon
      and button restyle (→ new `share-icon` branch).
- [ ] Push the stacked branches and open linked PRs (`but pr new <top-branch> -t`).
- [ ] CI (GitHub Actions) on every PR:
  - `cargo test` for `crates/*` and `plugins/share-target`
  - `cargo run -- check` in `crates/rules-tool` (rules valid, tests pass, generated files current)
  - `pnpm build` (TypeScript + Vite)
  - Android debug build; iOS build on a macOS runner
- [ ] Root `README.md`: replace the Tauri template text with what the app does and how to run it
      (mise, pnpm, `tauri android dev --open`, `tauri ios dev --open`).

## 2. Verify iOS end to end

- [ ] Update Xcode and install the matching iOS simulator runtime.
- [ ] Run on the simulator: share from Safari → Clean Share opens with the link → tracking shown →
      Share opens the iOS share sheet with the cleaned link.
- [ ] Run on a real iPhone: set the development team for both targets (`clean-share_iOS`,
      `ShareExtension`) so the App Group `group.com.cleanshare.app` works.
- [ ] Confirm the Share Extension can still open the app on the current iOS version. If Apple blocks
      it, fall back to a small confirmation UI in the extension ("Saved, open Clean Share").
- [ ] Decide whether the extension should hide itself from our own share sheet (it currently appears
      there, iOS doesn't allow excluding it).

## 3. Hosted rules (update rules without an app release)

- [ ] Publish `rules.json` (from `rules-tool bundle`) on a static host: GitHub Pages or a release
      asset. No cookies, no analytics.
- [ ] Sign the bundle (ed25519). Embed the public key in the app; reject unsigned or tampered files.
- [ ] Fetch in Rust on launch at most once a day; cache on disk; use only if:
      signature valid → `schema` supported → `link_rules::validate` passes →
      `link_cleaner::check_rule_tests` passes → `revision` newer than the cached/bundled one.
- [ ] Fall back to the bundled rules on any failure. Show the active rules revision in the app.
- [ ] CI job that bundles, signs and publishes when `rules/` changes on `main`.

## 4. More and better rules

- [ ] Check every rule against real links shared from each app (Android and iOS), and keep the
      examples as `[[test]]` entries.
- [ ] Schema v2 additions (bump `schema`, keep v1 readable):
  - `referral`: affiliate parameters (e.g. Amazon `tag`), removal controlled by a user setting.
  - `unwrap`: extract the real destination from redirect links (`google.com/url?q=`,
    `l.facebook.com/l.php?u=`, `youtube.com/redirect?q=`, `l.instagram.com/?u=`).
  - Path rules for sites that track in the path (Amazon `/ref=…`), only where tests prove it's safe.
- [ ] New providers: Amazon, Google, Pinterest, Snapchat, Bluesky, Medium, Substack, AliExpress,
      Shopee, newsletters (beehiiv, ConvertKit). Use ClearURLs as a reference, not a source to copy.
- [ ] Short links (`t.co`, `bit.ly`, `vm.tiktok.com`) hide their tracking behind a redirect.
      Expanding them needs a network request: make it opt-in and explain it.
- [ ] Contribution guide + issue template: "this link still has tracking" with the shared link.

## 5. App features

- [ ] Show why each parameter is removed (the rule `reason`) when tapping a red part.
- [ ] Copy button for the cleaned link (alongside Share).
- [ ] Paste / type a link inside the app, for links not shared via the share sheet.
- [ ] Android "Process text" action (`ACTION_PROCESS_TEXT`): select text in any app → Clean Share
      cleans it in place.
- [ ] Quick mode: clean and immediately re-open the share sheet, skipping the screen (setting).
- [ ] Clipboard cleaning on app open ("Your clipboard has a tracked link, clean it?").
- [ ] Settings: remove affiliate tags, quick mode, rules revision / last update.
- [ ] Optional local-only history of cleaned links (off by default, never leaves the device).

## 6. Design and polish

- [ ] App icon and splash based on `src/assets/share.svg` (all Android densities, iOS asset set).
- [ ] Review light/dark mode, font scaling, screen readers (VoiceOver / TalkBack labels for the
      highlighted parts, e.g. "tracking: si=…").
- [ ] Empty state with a short "how to use" illustration.
- [ ] Localization (start with English; structure strings for translation).

## 7. Release

- [ ] Android: create an upload keystore (stored outside the repo), signed release build
      (`pnpm tauri android build`), Play Console listing, internal testing track.
- [ ] iOS: App Store Connect app, bundle IDs for the app and the extension, App Group capability,
      TestFlight.
- [ ] Privacy policy: no data collected; the only network request is the rules download.
- [ ] Versioning: app version in `tauri.conf.json`; rules versioned separately by `revision`.
- [ ] Desktop (optional): the cleaner works on desktop already; a tray / clipboard cleaner could
      reuse it.

## 8. Technical debt

- [ ] Cargo workspace at the repo root (one `Cargo.lock`, shared `target/`) for `src-tauri`,
      `crates/*` and `plugins/share-target`, after checking Tauri's mobile builds handle it.
- [ ] Generated native projects are hand-edited (`src-tauri/gen/apple/project.yml` adds the Share
      Extension). Document this, and never run `tauri ios init` again without re-applying it.
- [ ] Android app module still targets Java 8 (Tauri default); move to 17 like the plugin.
- [ ] Two Android SDKs on this machine (mise-managed and `~/Library/Android/sdk` with the
      emulator image). Consolidate on the mise one: install `emulator` and a system image there.
- [ ] `NDK_HOME` in `mise.toml` is a full path tied to the mise android-sdk version; update it when
      that version changes.
- [ ] Android Studio must support the AGP version Tauri generates (currently 9.3.1).
