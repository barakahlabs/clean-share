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
- [x] CI (GitHub Actions on [Blacksmith](https://docs.blacksmith.sh) runners,
      `.github/workflows/ci.yml`) on every PR and push to `main`, skipping docs-only changes:
  - Rust: `cargo clippy -D warnings` + `cargo test` for each crate in `crates/`
  - `cargo run -- check` in `crates/rules-tool` (rules valid, tests pass, generated files current)
  - `pnpm build` (TypeScript + Vite), as part of the Android build (the separate `web` job is
    disabled)
  - Android debug APK (arm64), uploaded as an artifact for 7 days
  - Tool versions come from `mise.toml`; Android SDK packages from `mise run android:sdk`
  - Caches (Rust, Gradle) are saved from `main` only, so PRs read them but don't fill the cache
- [x] iOS simulator build in CI (`.github/workflows/ios.yml`), on GitHub's `macos-latest`, which
      is free for public repos (M1, 3 cores; slower than Blacksmith's $0.08/min M4). Runs on PRs
      that touch `src-tauri/gen/apple/`, `src-tauri/tauri.ios.conf.json` or
      `plugins/share-target/{ios,src}/`, and by hand (Actions → iOS → Run workflow).
  - Linux builds the frontend (`dist/`) and passes it on; the Mac only compiles the iOS Rust code
    and the app: `pnpm tauri ios build --debug --target aarch64-sim` with the frontend step
    skipped.
  - Simulator builds are signed ad hoc ("Sign to Run Locally"), so no certificate or Apple account
    is needed. The `.app` is kept as an artifact for 7 days.
  - If the repo goes private, macOS minutes cost money on GitHub too: make it manual-only or move
    it to Blacksmith (`blacksmith-6vcpu-macos-latest`).
- [ ] iOS device/release builds in CI (signing certificates and profiles as secrets). Until then,
      build releases on the Mac.
- [ ] `plugins/share-target`: no Rust tests yet; it's compiled by the Android job. Testing it on
      Linux needs Tauri's system packages (`libwebkit2gtk-4.1-dev` and others).
- [ ] Root `README.md`: replace the Tauri template text with what the app does and how to run it
      (mise, pnpm, `tauri android dev --open`, `tauri ios dev --open`).

## 2. Verify iOS end to end

- [ ] Update Xcode and install the matching iOS simulator runtime.
- [ ] Run on the simulator: share from Safari → Clean Share opens with the link → tracking shown →
      Share opens the iOS share sheet with the cleaned link.
- [ ] Run on a real iPhone: set the development team for both targets (`clean-share_iOS`,
      `ShareExtension`) so the App Group `group.com.sirajulm.cleanshare` works.
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
- [ ] Get to the front of the share sheet (see [Share sheet position](#share-sheet-position)).

### Share sheet position

Neither platform lets an app set its own position: both order the share sheet by how often and
how recently each target is used. What we can do:

| Platform | What | When |
|---|---|---|
| Android | **Sharing shortcuts** (Direct Share): publish "Clean & Share" as a share-target shortcut, and "Save bookmark" once bookmarks exist. Needs `res/xml/shortcuts.xml` with `<share-target>` and `ShortcutManagerCompat.pushDynamicShortcut` in `plugins/share-target`. Shortcuts can show in the top row of the share sheet, above the app list; the system still ranks them by use | Next |
| Android | **Pin tip**: many Android versions and skins (e.g. Samsung) let users long-press a target and pin it. Varies by version and maker, so verify on real devices before writing the steps | Next |
| iOS | **Favorites tip**: in the share sheet's app row, More → Edit → add Clean Share to Favorites and drag it first. Favorites stay in front | Next |
| iOS | **Action extension** "Clean link": a second entry in the action list (below the app row), which users can also move to the top | Later |
| Both | Not possible: `android:priority` on intent filters doesn't change share sheet order; the iOS top row of people/conversations is for messaging apps | – |

The tip is a one-time card after the first successful share ("Put Clean Share first"), with the
steps for the current platform, and can be reopened from Settings. Check the Android steps on a
Pixel and a Samsung before writing them.

Open question: is an iOS Action extension worth a second entry next to the Share Extension?

### Bookmarks

Save shared links with tags, inside this app, local-only to start. Plan and data model:
[`docs/BOOKMARKS.md`](BOOKMARKS.md).

- [ ] Phase 1: "Save bookmark" share entry (Android), SQLite storage, list screen.
- [ ] Phase 2: tags and search.
- [ ] Phase 3: iOS share extension support.

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
