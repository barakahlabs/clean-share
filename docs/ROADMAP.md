# Clean Share roadmap

Clean Share receives links shared from other apps, highlights tracking parameters in red, and
shares the cleaned link onwards. This file lists what's done and the possible next steps, roughly
in priority order within each section.

## Where things stand (November 2026)

| Area | State |
|---|---|
| App shell | Tauri 2 + React (TypeScript); tools pinned in `mise.toml`; Bookmarks and Settings tabs |
| Share entries | *Clean & Share* and *Save bookmark*, on Android (activity alias) and iOS (two share extensions, App Group); tested on the emulator and on an iPhone 13 Pro |
| Tracking removal | Rust (`crates/link-cleaner`), typed TOML rules (`rules/`), 9 providers + global rules, 23 rule tests |
| Rule tooling | `crates/rules-tool`: `check`, `generate` (schemas + README tables), `bundle` |
| Bookmarks | Phases 0–3, 5 and 7 done (see [Bookmarks](#bookmarks)); AI tagging, re-cleaning and sync open |
| CI | Blacksmith: Rust crates + Android APK on every PR; iOS simulator build by hand |
| Dependencies | Renovate, grouped (Android, toolchain, Tauri); presets copied from `barakahlabs/configs` |

## 1. Housekeeping

- [x] Commit and push the first stacked branches (scaffold, share target, cleaner, rules schema).
- [x] CI (GitHub Actions on [Blacksmith](https://docs.blacksmith.sh) runners,
      `.github/workflows/ci.yml`) on every PR and push to `main`, skipping changes that only touch docs, `README.md`, `LICENSE` or `renovate.json`:
  - Rust: `cargo clippy -D warnings` + `cargo test` for the workspace (all crates; the app and
    the plugin are built by the Android job, since on Linux they need Tauri's system libraries)
  - `cargo run -p rules-tool -- check` (rules valid, tests pass, generated files current)
  - `pnpm build` (TypeScript + Vite), as part of the Android build (the separate `web` job is
    disabled)
  - Android debug APK (arm64), uploaded as an artifact for 7 days
  - Tool versions come from `mise.toml`; Android SDK packages from `mise run android:sdk`
  - Caches (Rust, Gradle) are saved from `main` only, so PRs read them but don't fill the cache
- [x] iOS simulator build in CI (`.github/workflows/ios.yml`), on GitHub's `macos-latest`, which
      is free for public repos (M1, 3 cores; slower than Blacksmith's $0.08/min M4). Manual only:
      Actions → iOS → Run workflow, then pick the branch.
  - Linux builds the frontend (`dist/`) and passes it on; the Mac only compiles the iOS Rust code
    and the app: `pnpm tauri ios build --debug --target aarch64-sim` with the frontend step
    skipped.
  - Simulator builds are signed ad hoc ("Sign to Run Locally"), so no certificate or Apple account
    is needed. The `.app` is kept as an artifact for 7 days.
  - If the repo goes private, macOS minutes cost money on GitHub too; Blacksmith
    (`blacksmith-6vcpu-macos-latest`, $0.08/min) is the faster alternative.
- [ ] iOS device/release builds in CI (signing certificates and profiles as secrets). Until then,
      build releases on the Mac.
- [ ] `plugins/share-target`: no Rust tests yet; it's compiled by the Android job. Testing it on
      Linux needs Tauri's system packages (`libwebkit2gtk-4.1-dev` and others).
- [x] Root `README.md`: what the app does, layout, setup, run, test, CI.
- Note on stacked PRs: GitHub's own rebase of a stack after a merge can produce unsigned
  commits. After each merge, update locally (`but pull`) and push the rest before merging the
  next one.

## 2. Verify iOS end to end

- [x] Run on the simulator: share → Clean Share opens with the link → tracking shown → Share
      opens the iOS share sheet with the cleaned link.
- [x] Run on a real iPhone (13 Pro): development team set for the app and both extensions
      (`ShareExtension`, `SaveExtension`), so the App Group `group.com.sirajulm.cleanshare` works;
      Save bookmark works from other apps.
- [x] The share extensions still open the app on the current iOS version.
- [ ] Decide whether the extensions should hide themselves from our own share sheet (they appear
      there; iOS doesn't allow excluding them).
- [ ] YouTube inline on iOS: the embedded player fails there (no `https` referrer from
      `tauri://localhost`), so it opens the YouTube app. Backlog: a native player sheet
      ([bookmarks phase 5](BOOKMARKS.md)).

## 3. Hosted rules (update rules without an app release)

- [x] CI publishes the rules on every change on `main` (`.github/workflows/rules.yml`): `rules-tool
      check`, `bundle`, sign, verify with the app's own check, deploy to GitHub Pages:
      `https://barakahlabs.github.io/clean-share/rules/v1/rules.json` and `rules.json.sig`. One
      file per schema version, overwritten each time; a breaking format change would add `v2`.
- [x] Signed with ed25519: private key in the GitHub secret `RULES_SIGNING_KEY` (local copy in
      `.secrets/`, backup in Passwords); public key in `crates/rules-update`.
- [x] The app checks on launch, every hour while open, and when it comes back after an hour; each
      check is a conditional request (`If-None-Match`), so an unchanged file costs a 304 and no
      body. Mobile data is fine (a few KB).
- [x] A download is used only if the signature is valid, the schema is the app's, the rules
      validate and pass their own tests, and the revision is newer; then it's cached, used
      everywhere (clean screen, previews, bookmarks) and saved links are re-cleaned.
- [x] Older apps read newer bundles: unknown fields are ignored (the TOML files stay strict:
      `rules-tool check` reports unknown keys). Settings shows the rules revision, where it came
      from and when it was last checked, with a "check now".
- If a check fails (offline, Pages down, a bad file), the app keeps the rules it has.

## 4. More and better rules

- [ ] Check every rule against real links shared from each app (Android and iOS), and keep the
      examples as `[[test]]` entries.
- [x] Redirect links are replaced by the link they lead to (`[[redirect]]`): `google.com/url`,
      `l.facebook.com/l.php`, `l.instagram.com/?u=`, `youtube.com/redirect`,
      `linkedin.com/redir/redirect`. Still `schema = 1`: tests say which feature they need
      (`needs = ["redirect"]`), and older apps skip those tests instead of refusing the rules.
- [ ] More rule features (the same way: a `needs` name, older apps skip their tests):
  - `referral`: affiliate parameters (e.g. Amazon `tag`), removal controlled by a user setting.
  - Path rules for sites that track in the path (Amazon `/ref=…`), only where tests prove it's safe.
- [ ] New providers: Amazon, Pinterest, Snapchat, Bluesky, Medium, Substack, AliExpress,
      Shopee, newsletters (beehiiv, ConvertKit). Use ClearURLs as a reference, not a source to copy.
- [ ] Short links (`t.co`, `bit.ly`, `vm.tiktok.com`) hide their tracking behind a redirect.
      Expanding them needs a network request: make it opt-in and explain it.
- [ ] Contribution guide + issue template: "this link still has tracking" with the shared link.

## 5. App features

- [x] Show why each parameter is removed (the rule `reason`) when tapping a red part; redirects
      too.
- [x] Copy button for the cleaned link (alongside Share), on the share screen and the Clean tab.
- [x] Paste / type a link inside the app (Clean tab), for links not shared via the share sheet:
      cleaned as you type, then copy, share or save.
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

Save shared links with tags, inside this app, local-only. Plan, data model and phase details:
[`docs/BOOKMARKS.md`](BOOKMARKS.md).

- [x] Phase 0–1: storage on Turso (`crates/bookmarks`, migrations), "Save bookmark" share entry on
      Android and iOS, save sheet, list and detail screens.
- [x] Phase 2: tags (Settings → Tags), search as you type, tag filters, tag suggestions
      (`crates/tag-suggest`).
- [x] Phase 3: link previews (`crates/link-preview`): opt-in, optionally Wi-Fi only; thumbnails
      and site icons on the phone; grid of cards.
- [x] Phase 5: tap-to-play players (YouTube, Vimeo, Spotify, X, TikTok, SoundCloud).
- [x] Phase 7: export (JSON backup, browser HTML) and import (backups, browser bookmarks, Pocket,
      Raindrop).
- [x] Phase 4, first part: keyword suggestions from the page text, on every phone.
- [ ] Phase 4: on-device AI tagging needs a newer phone (Apple Intelligence: iPhone 15 Pro+;
      Gemini Nano: recent Pixels/Samsungs).
- [x] Phase 6: re-clean saved links when the rules change (on launch, or Settings → Clean saved
      links again), merging links that become duplicates, with a report.
- [ ] Phase 8: accounts and paid sync (one Turso database per user).
- [ ] Faster big imports: one transaction instead of one save per bookmark.

## 6. Design and polish

- [ ] App icon and splash based on `src/assets/share.svg` (all Android densities, iOS asset set).
- [ ] Review light/dark mode, font scaling, screen readers (VoiceOver / TalkBack labels for the
      highlighted parts, e.g. "tracking: si=…").
- [ ] Empty state with a short "how to use" illustration.
- [x] Bookmarks as a grid of cards (two per row on a phone), with site icons or initials as the
      fallback picture; pull-down no longer drags the tab bar.
- [ ] Localization (start with English; structure strings for translation).

## 7. Release

- [ ] Android: create an upload keystore (stored outside the repo), signed release build
      (`pnpm tauri android build`), Play Console listing, internal testing track.
- [ ] iOS: App Store Connect app, bundle IDs for the app and the extension, App Group capability,
      TestFlight.
- [ ] Privacy policy: no data collected. Network requests: link previews (off by default; each
      saved link is fetched once with an honest user agent, no cookies), players (only when Play
      is tapped), and the rules download once hosted rules exist.
- [ ] Versioning: app version in `tauri.conf.json`; rules versioned separately by `revision`.
- [ ] Desktop (optional): the cleaner works on desktop already; a tray / clipboard cleaner could
      reuse it.

## 8. Technical debt

- [x] Cargo workspace at the repo root (one `Cargo.lock`, shared `target/`) for `src-tauri`,
      `crates/*` and `plugins/share-target`; Tauri's Android and iOS builds use the shared
      `target/` (they ask Cargo where it is).
- [ ] Generated native projects are hand-edited. Document this, and never run `tauri ios init` or
      `tauri android init` again without re-applying:
  - iOS: `src-tauri/gen/apple/project.yml` adds the two share extensions (`ShareExtension`,
    `SaveExtension`) and the App Group.
  - Android: the "Save bookmark" activity alias and its string (`AndroidManifest.xml`,
    `strings.xml`); `MainActivity.kt` pads content by the keyboard's height and turns off the
    WebView's overscroll stretch.
- [ ] Android app module still targets Java 8 (Tauri default); move to 17 like the plugin.
- [ ] Two Android SDKs on this machine (mise-managed and `~/Library/Android/sdk` with the
      emulator image). Consolidate on the mise one: install `emulator` and a system image there.
- [ ] `NDK_HOME` in `mise.toml` is a full path tied to the mise android-sdk version; update it when
      that version changes.
- [ ] Android Studio must support the Android Gradle plugin version in use (now 9.5.0).
