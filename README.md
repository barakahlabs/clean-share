# Clean Share

Share a link to Clean Share and it removes the tracking (`?si=…`, `utm_…`, `igsh=…` and the
like): the tracking is highlighted in red, and the cleaned link goes on to wherever you share it
next. Or save it as a bookmark: tags with suggestions, search, optional link previews, players for
video and music, and export and import.

Android and iOS, built with [Tauri 2](https://tauri.app) (Rust core, React UI). Everything stays on
the phone; nothing goes online unless you turn link previews on or tap Play.

## What it does

- **Two share-sheet entries**: *Clean & Share* (clean, then share onwards) and *Save bookmark*
  (clean, then save with tags; on Android it goes straight back to the app you shared from).
- **Tracking rules** per service in plain TOML ([`rules/`](rules/README.md)), with tests:
  YouTube, Instagram, TikTok, X, Threads, Facebook, Reddit, LinkedIn, Spotify, plus global rules.
- **Bookmarks** ([design](docs/BOOKMARKS.md)): a grid of cards; tags with suggestions (from the
  text, the site, tags used together, and the page itself); search as you type and tag filters;
  link previews with thumbnails (off by default, optionally Wi-Fi only); tap-to-play YouTube,
  Vimeo, Spotify, X, TikTok and SoundCloud; export (JSON backup, browser HTML) and import
  (backups, browser bookmarks, Pocket, Raindrop).

## Layout

| Path | What |
|---|---|
| `src/` | React UI (screens, components, typed wrappers for the Rust commands) |
| `src-tauri/` | The Tauri app: commands, preview worker, settings. `gen/` holds the Android and iOS projects (hand-edited, see the roadmap's technical debt) |
| `crates/link-rules` | Typed rules schema, loading and validation |
| `crates/link-cleaner` | Finds links in shared text and cleans them; rules are embedded at build time |
| `crates/rules-tool` | `check` (rules and their tests), `generate` (schemas, README tables), `bundle` |
| `crates/bookmarks` | Storage on [Turso](https://turso.tech) (one database per user), tags, search, export/import |
| `crates/tag-suggest` | Tag suggestions, no network |
| `crates/link-preview` | Link previews: oEmbed, Open Graph, JSON-LD, thumbnails, with request limits |
| `plugins/share-target` | Tauri plugin: receive shares, share text and files, network type (Kotlin, Swift) |
| `rules/` | The tracking rules (CC0) |
| `docs/` | [Roadmap](docs/ROADMAP.md), [bookmarks design](docs/BOOKMARKS.md), [decisions](docs/adr/) |

## Setup

Tool versions are pinned in [`mise.toml`](mise.toml) (Node, pnpm, Java, Android command-line
tools). With [mise](https://mise.jdx.dev) and Rust installed:

```sh
mise install
mise run android:sdk      # Android platform, build tools and NDK (once)
pnpm install
```

iOS needs Xcode. Signing uses the development team in `src-tauri/tauri.conf.json` and
`src-tauri/gen/apple/project.yml` (both targets, so the App Group shared with the share
extensions works); change it to yours.

## Run

```sh
pnpm tauri android dev    # emulator or connected phone
pnpm tauri ios dev        # simulator or connected iPhone
pnpm tauri dev            # desktop window (no share sheet, handy for the UI)
```

Debug builds: `pnpm tauri android build --debug --apk --target aarch64`,
`pnpm tauri ios build --debug`.

## Test

```sh
cargo clippy --workspace --all-targets -- -D warnings   # everything, from the repo root
cargo test --workspace
cargo run -p rules-tool -- check                        # rules
pnpm build                                              # TypeScript
```

One Cargo workspace (`Cargo.toml` at the root) holds the app, the crates and the plugin, with one
`Cargo.lock` and one `target/`. On Linux, add `--exclude clean-share --exclude
tauri-plugin-share-target` unless Tauri's system libraries are installed (CI does this).
`cargo run -p link-preview --example preview -- <link>` prints a live preview.

## CI

- **CI** ([`ci.yml`](.github/workflows/ci.yml), on [Blacksmith](https://docs.blacksmith.sh)
  runners): every crate's clippy and tests, the rules check, and an Android debug APK, on each PR
  and push to `main`. Changes to docs, `README.md`, `LICENSE` or `renovate.json` alone skip it.
- **iOS** ([`ios.yml`](.github/workflows/ios.yml)): simulator build, run by hand from Actions.
- **Renovate** keeps dependencies current ([`renovate.json`](renovate.json)).

## License

Code: [MIT](LICENSE). Tracking rules: [CC0](rules/LICENSE).
