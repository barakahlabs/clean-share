# Bookmarks

Status: **in progress**. `crates/bookmarks` (storage, tags, search) exists with tests; not yet
wired into the app. Work happens on the `bookmarks` branch.

Save links shared from other apps, tag them, and find them later. Bookmarks live inside Clean Share,
so every saved link is cleaned of tracking first.

The goal is **rich bookmarks**: a saved link can grow from plain text, to a preview card (title,
image, site), to a full embed (YouTube player, X post), with tags that the app can suggest, including
with AI. We start with text only and design the data so each step adds to it without migrations that
throw anything away.

## Goals

- Save a link in two taps from any app's share sheet.
- Tag links and find them again quickly (search, filter by tag).
- Store clean links only: no share tokens, no tracking parameters.
- Rich display when wanted: preview cards, and embeds on tap.
- Tag suggestions, from simple matching up to optional AI.
- Local-first: works offline; anything that contacts the network is opt-in and explained.
- Optional paid sync: bookmarks survive reinstalls and appear on every device the user signs in
  on (see [Accounts and sync](#accounts-and-sync)). Without it, the app needs no account.

## Non-goals (for now)

- Sharing collections between users.
- Archiving full pages for offline reading.
- Browser extensions.

## Principles

1. **Clean before anything else.** Every link is cleaned by `crates/link-cleaner` before it is
   stored, fetched, embedded or sent to an AI model.
2. **Nothing leaves the device by default.** Previews, cloud AI and embeds are each separate,
   opt-in settings, with a one-line explanation of what gets contacted.
3. **Embeds load on tap.** Third-party embed scripts track viewers, so the list shows our own
   preview cards; the real embed loads only when the user taps it.
4. **AI suggests, the user decides.** Suggested tags are shown as chips to accept; nothing is
   tagged automatically without confirmation (an "auto-accept" setting can come later).
5. **Rust core, thin UI.** Storage, search, preview parsing and tag suggestions live in Rust crates
   with unit tests; React only renders and calls commands.

## Decisions

| Decision | Choice | Why |
|---|---|---|
| Separate app or combined | Combined into Clean Share | Same entry point (share sheet); cleaning is the first step of saving |
| Platforms | Android and iOS together from phase 1 | Storage and UI are shared; only the share entry differs |
| Storage | Turso (`turso` Rust crate, pinned to 0.8.3), a local database file on the phone, hand-written SQL (not Toasty: [ADR 0001](adr/0001-bookmarks-storage-turso-crate-vs-toasty.md)). Fallback: `libsql` | SQLite-compatible, offline, one file; a synced database gives the same `Connection` type. Confirmed by the [storage spike](#storage-spike-results) |
| Search | Turso full-text index with the `ngram` tokenizer | Search-as-you-type; Turso's `word*` prefix queries don't match yet |
| IDs | Text UUIDv5 derived from content: bookmark from its cleaned URL, tag from its lowercased name | Two phones saving the same link offline create the same row; sync replays rows by primary key |
| Multi-tenancy | **One database per user** in Turso Cloud | SQLite has no row-level security; per-user databases make Turso enforce isolation and let phones sync directly |
| Sync | Opt-in, paid. Local database ⇄ the user's cloud database via `push()` / `pull()` | Free tier stays account-free and offline; sync is what costs us money |
| Accounts | Only for sync. Clerk or Auth0 (Sign in with Apple / Google); Turso checks their tokens via JWKS | No Turso key stored on the phone; login again = access again |
| Master database | Small `account` table on our backend (user → database name, plan) | Only the backend reads it; phones never connect to it |
| Backend | One small serverless function | Holds the Turso admin key; creates/deletes user databases; handles billing events |
| Link cleaning | Reuse `crates/link-cleaner` + `rules/` | Same rules everywhere; dedupes links that differ only by tracking |
| Share sheet entries | Two: **Clean & Share** and **Save bookmark** | Each entry does one thing; the intent is clear up front |
| Original link | Kept in `original_url`, never shown by default | Lets bookmarks be re-cleaned when rules improve, or recovered if a rule breaks a link |
| Titles in phase 1 | From the shared text only | No network until previews are switched on |
| Previews | Opt-in fetch of the cleaned URL, no cookies | Rich cards without tracking the user |
| Embeds | Tap to load, per item | Keeps the list tracker-free |
| AI tagging | Tiered: suggestions → on-device AI → opt-in cloud AI | Works on every phone; better where available |

## User flows

### Save from another app

1. In Instagram / YouTube / X / a browser: **Share → Save bookmark**.
2. Clean Share opens the save screen:
   - cleaned link (removed tracking shown in red, as on the share screen)
   - title (from the shared text, editable)
   - tag input with suggested tags as chips
   - optional note
3. **Save** → a short "Saved" confirmation; on Android the app finishes and returns to the source
   app; on iOS the user swipes back (iOS doesn't allow returning automatically).

If the cleaned link is already saved, the screen opens that bookmark instead, with its tags, so
new tags can be added. No duplicates.

### Browse and find

- List of bookmarks, newest first: title, site, tags, and (once previews exist) a thumbnail.
- Search across title, URL, note, content and tags.
- Filter by one or more tags (AND).
- Tap a bookmark → detail: open in browser, share (cleaned), edit, delete, and later "Show embed".

### Organize

- Edit title, note and tags.
- Tag management: rename, merge, delete.
- Re-clean all bookmarks when the rules revision changes (merging any new duplicates).

## Architecture

```text
Share sheet ──► plugins/share-target ──► { text, intent: share | save }
                                                │
React (src/) ── screens ── invoke() ──► src-tauri commands
                                                │
                    ┌───────────────────────────┼─────────────────────────┐
                    ▼                           ▼                         ▼
          crates/bookmarks            crates/link-preview        crates/tag-suggest
          (Turso local DB, search,    (fetch + parse OG tags,    (matching, AI providers
           migrations, sync)           oEmbed; phase 3)           behind one trait; phase 2+)
                    │
          crates/link-cleaner (already exists)

Paid sync only (phase 8):

  Phone ──sign in──► Clerk / Auth0 ──JWT──► Phone
  Phone ──JWT──► Backend function ──► master DB (account table)
                       │                └─ creates the user's DB with the Turso admin key
                       └─► returns db URL
  Phone ──JWT + db URL──► Turso Cloud: u-<userid>   (push / pull)
```

| Piece | Location | Responsibility |
|---|---|---|
| Storage | `crates/bookmarks` (new) | Schema, migrations, save/get/list/update/delete, tags, search, sync (phase 8) |
| Backend | `backend/` (new, phase 8) | Serverless function + master DB: account lookup, DB creation, billing, deletion |
| Previews | `crates/link-preview` (new, phase 3) | HTTP fetch with limits, Open Graph / meta parsing, oEmbed |
| Tag suggestions | `crates/tag-suggest` (new, phase 2) | `TagSuggester` trait; matching suggester; AI providers later |
| Tauri commands | `src-tauri/src/bookmarks.rs` | Thin wrappers; opens the DB in the app data dir; settings |
| Share routing | `plugins/share-target` | Report which share entry was used |
| On-device AI | `plugins/on-device-ai` (new, phase 4) | Swift (Foundation Models) / Kotlin (Gemini Nano) bridge |
| UI | `src/screens/`, `src/components/` | Save, list, detail, tags, settings screens |
| iOS extension | `src-tauri/gen/apple/` + `project.yml` | Second Share Extension target "Save bookmark" |

## Data model

Schema v1 lives in `crates/bookmarks/migrations/0001_init.sql`; this is a summary. Created with all columns,
so later phases only fill them in. Migrations use `PRAGMA user_version`. Each phone migrates its own
database on open, so there is never a server job that migrates every user's database. A database
with a newer version than the app knows (e.g. synced from a phone with a newer app) is refused with
"update the app" instead of being opened.

- **No user id anywhere.** One database belongs to one user ([why](#why-one-database-per-user)).
- **IDs** are text UUIDv5 derived from content (see Decisions), not integers.
- **Times** are integers, milliseconds since the Unix epoch.
- **Tables are `STRICT`**, so a wrong type is an error, not silently stored.
- **Only data that should follow the user** goes in this database. Device-only things (thumbnail
  files, "previews on this phone" settings) live outside it, so sync never carries a path or
  setting that only makes sense on one device.

```text
bookmark
  id                  text primary key       UUIDv5 of the cleaned URL at first save
  url                 text not null unique   cleaned URL (the identity of a bookmark)
  original_url        text                   as shared, with tracking (never shown by default)
  title               text                   from shared text, preview, or user
  note                text                   user's own note
  shared_text         text                   the full text that came with the share
  provider            text                   provider id from rules/ (e.g. "youtube"), if matched
  site_name           text                   phase 3: og:site_name or domain
  description         text                   phase 3: og:description / meta description
  image_url           text                   phase 3: og:image (cleaned)
  embed_kind          text                   phase 5: "youtube" | "x" | … (null = no embed)
  embed_url           text                   phase 5: URL the embed loads from
  preview_status      text not null          "none" | "pending" | "done" | "failed"
  preview_fetched_at  integer
  rules_revision      text not null          rules revision used to clean `url`
  tags_text           text not null          tag names joined by spaces (for search)
  created_at          integer not null
  updated_at          integer not null

tag
  id                  text primary key       UUIDv5 of the lowercased name
  name                text not null unique collate nocase
  created_at          integer not null

bookmark_tag
  bookmark_id         → bookmark.id on delete cascade
  tag_id              → tag.id on delete cascade
  source              text not null          "user" | "suggested" | "ai"
  created_at          integer not null
  primary key (bookmark_id, tag_id)

bookmark_search       full-text index on bookmark (ngram tokenizer)
  title, note, shared_text, description, tags_text

setting               not yet: added by a migration when the first synced preference exists
```

- **Thumbnails** (phase 3) are files named after the bookmark ID in the app data dir, not a column.
- **Tags:** names are trimmed, a leading `#` is dropped, and inner spaces collapse (`" #Rust  lang"`
  → `Rust lang`). Tags stay when no bookmark uses them any more (count 0); Settings → Tags lists every tag with its count, so the user decides what to delete. Renaming moves
  bookmarks to the tag with the new name (merging if it exists), because the ID follows the name.
- **Search:** every word of at least two letters must match somewhere (`rust talk` = rust AND
  talk), case-insensitive, also inside words (`ru` finds "Rust", and also "Trusted"). Results are
  ordered by relevance, then newest first.
- **Index upkeep:** `Store::optimize()` compacts the search index. The app calls it at most once a
  week on launch, and after imports (phase 7).

- **Content for tagging and search** = title + shared_text + description + note (+ article text
  later). Phase 1 already has title and shared_text; phase 3 adds description.
- **Duplicates:** `url` is unique. Saving an existing URL merges: keeps the older bookmark, adds new
  tags, keeps the existing title unless it was empty.
- **Rule updates:** when `rules_revision` differs from the current rules, re-clean `original_url`; if
  the new cleaned URL already exists, merge the two bookmarks (union of tags, keep the older one).

## Rust API (`crates/bookmarks`)

```text
crates/bookmarks/migrations/
  0001_init.sql one SQL file per schema version, compiled into the app

crates/bookmarks/src/
  lib.rs        crate docs: one database = one user
  db.rs         where data lives: Location::{Memory, File} → turso::Connection   (sync.rs joins here)
  migrate.rs    runs migrations/NNNN_*.sql in order (PRAGMA user_version), refuses newer schemas
  ids.rs        content-derived UUIDv5 ids
  cleaner.rs    Cleaner trait: BundledRules, Rules(RuleSet) for downloaded rules
  clock.rs      Clock trait (tests use a fake clock)
  model.rs      Bookmark, NewBookmark, ListQuery, Page, tags… (serde, camelCase for the UI)
  store/        Store: save/get/update/delete, tags, list + search, row mapping, tests
```

```rust
pub struct Store { /* turso::Connection, Arc<dyn Cleaner>, Arc<dyn Clock> */ }

impl Store {
    pub async fn new(conn: Connection, cleaner: Arc<dyn Cleaner>, clock: Arc<dyn Clock>) -> Result<Store>;
    pub async fn open(location: &Location) -> Result<Store>;   // bundled rules + system clock
    pub async fn open_in_memory() -> Result<Store>;
    pub fn set_cleaner(&mut self, cleaner: Arc<dyn Cleaner>);  // newer rules downloaded

    pub async fn prepare(&self, text: &str) -> Result<Prepared>;       // links + existing bookmark
    pub async fn save(&mut self, new: NewBookmark) -> Result<Saved>;   // Saved { bookmark, created }
    pub async fn get(&self, id: &str) -> Result<Option<Bookmark>>;
    pub async fn find_by_url(&self, cleaned: &str) -> Result<Option<Bookmark>>;
    pub async fn list(&self, q: &ListQuery) -> Result<Page<Bookmark>>; // search, tags (AND), cursor
    pub async fn update(&mut self, id: &str, patch: BookmarkPatch) -> Result<Bookmark>;
    pub async fn delete(&mut self, id: &str) -> Result<()>;

    pub async fn set_tags(&mut self, id: &str, tags: &[TagInput]) -> Result<Vec<BookmarkTag>>;
    pub async fn tags(&self) -> Result<Vec<TagCount>>;                  // most used first
    pub async fn rename_tag(&mut self, from: &str, to: &str) -> Result<()>; // merges if `to` exists
    pub async fn delete_tag(&mut self, name: &str) -> Result<()>;

    pub async fn optimize(&mut self) -> Result<()>;                    // compact the search index

    // phase 6: reclean_all(&mut self) -> Result<RecleanReport>, using the current Cleaner
}
```

**Adding sync later** touches only `db.rs` (a `sync` module that opens the user's cloud database
and returns its connection, plus `push`/`pull`) and the app (which database to open after sign-in).
`Store` and every query stay the same. The app holds the current `Store` in its state and swaps it
on sign-in / sign-out.

`NewBookmark` takes the **shared text**; the store calls `link_cleaner` itself, so no caller can
store an uncleaned URL. If the text has several links, the first one is the bookmark and the rest
stay in `shared_text` (the save screen shows a picker when there is more than one).

## Tauri commands (`src-tauri/src/bookmarks.rs`)

| Command | Input | Output |
|---|---|---|
| `bookmark_prepare` | `text` | cleaned analysis, existing bookmark (if any), suggested tags |
| `bookmark_save` | `text`, `title?`, `note?`, `tags[]` | `{ bookmark, created }` |
| `bookmark_list` | `search?`, `tags[]`, `cursor?` | page of bookmarks |
| `bookmark_get` / `bookmark_update` / `bookmark_delete` | id (+ patch) | bookmark / – |
| `tag_list` / `tag_rename` / `tag_delete` | – / names | tags with counts |
| `tag_suggest` | `bookmark_id` or draft text | `[{ name, source, score }]` |
| `settings_get` / `settings_set` | key / value | – |
| `preview_fetch` (phase 3) | id | updated bookmark (runs in the background, emits an event) |

The database is opened once at startup in `app.path().app_data_dir()/bookmarks.db` and kept in
Tauri state behind a `Mutex`. With sync on (phase 8), it is opened in sync mode instead, with the
user's database URL and a fresh sign-in token.

## Share sheet entries

`get_shared` returns `{ text, intent: "share" | "save" }`; `App.tsx` opens the share screen or the
save screen accordingly.

| Platform | How "Save bookmark" appears |
|---|---|
| Android | An `activity-alias` of `MainActivity` labelled "Save bookmark" with its own `ACTION_SEND` intent-filter. The plugin reads the alias name from the intent's component to set `intent: "save"`. After saving, the activity finishes, so the user is back in the source app |
| iOS | A second Share Extension target, `SaveExtension` ("Save bookmark"), with the same App Group. It stores `{ text, intent: "save" }` and opens `cleanshare://save`. The existing extension writes `intent: "share"` |

Later option (iOS): a small SwiftUI save form inside the extension, so the user never leaves the
source app. That needs the database in the App Group container; it's left for after phase 2.

## Previews (phase 3)

Turned on by **Settings → Fetch link previews** (off by default). When on, saving a bookmark queues a
background fetch.

1. **oEmbed first** for providers that offer it without an account:
   - YouTube: `https://www.youtube.com/oembed?url=…&format=json` → title, channel, thumbnail.
   - X: `https://publish.twitter.com/oembed?url=…` → post text, author.
   - Instagram's official oEmbed needs a Facebook app token; without one we fall back to step 2,
     which often returns little because Instagram asks visitors to log in.
2. **Page metadata** otherwise: fetch the cleaned URL and read `og:title`, `og:description`,
   `og:image`, `og:site_name`, `twitter:*`, `<title>`, `<meta name="description">`.
3. **Thumbnail**: download `og:image` (cleaned), resize to ~400 px, store as a file in the app data
   dir; the list never loads images from the network.

Fetch rules: only `http(s)`, cleaned URL only, no cookies, a plain browser-like user agent, 10 s
timeout, stop reading after 1 MB of HTML / 5 MB of image, follow at most 5 redirects (cleaning each
redirect target), refuse private/local IP addresses. Failures set `preview_status = "failed"` with a
manual "Retry" in the detail screen.

Implementation: `reqwest` (rustls) + `scraper` for HTML, `image` for resizing, in
`crates/link-preview` with tests against saved HTML fixtures (no network in tests).

## Tagging

### Tier 1 – suggestions (phase 2, every phone, offline)

`crates/tag-suggest` with a `MatchSuggester`:
- **Existing tags** whose name (or a simple singular/plural form) appears in the content.
- **Provider tags**: a small built-in map, e.g. youtube → `video`, spotify → `music`,
  instagram/x/threads/tiktok → `social`. Shown only if the user has used the tag before or accepts it.
- **Co-occurrence**: tags often used together with already-picked tags, and tags used for other
  bookmarks from the same site.
- Ranked by score; top 5 shown as chips.

### Tier 2 – on-device AI (phase 4, newer phones)

`plugins/on-device-ai` exposes one command: `suggest_tags(content, existing_tags) -> [String]`.
- iOS 26+: Apple's Foundation Models framework, on Apple Intelligence devices (iPhone 15 Pro and
  newer). Prompt: pick from existing tags first, propose at most 2 new ones, lowercase.
- Android: Gemini Nano through ML Kit's GenAI APIs, on supported devices.
- Reports `available: false` elsewhere; the UI then hides the option.

### Tier 3 – cloud AI (phase 4, opt-in)

Settings → "AI tag suggestions (cloud)". Sends **title, description and domain** (never the
original URL, never notes unless the user allows it) to an LLM API.
- Options to decide in phase 4: the user's own API key (stored in Keychain / Keystore, no server for
  us) vs. a small proxy we run (easier for users, costs us, needs abuse limits).
- Same `suggest_tags` contract as tier 2, so the UI doesn't care which tier answered.

All tiers implement one trait, and results merge (dedupe by name, keep the best score, remember the
source so accepted tags store `source = "suggested" | "ai"`).

## Embeds (phase 5)

Detail screen → **Show embed** (only for bookmarks with `embed_kind`):
- YouTube: `iframe` to `https://www.youtube-nocookie.com/embed/<id>` (reduced cookies).
- X: X's embed script, after a one-line notice that X will see the view.
- Others: only the preview card.

The embed renders in a sandboxed `iframe` with no access to the app's commands; it's removed when
leaving the screen.

## UI

- **App shell**: bottom tabs **Bookmarks** and **Settings**; the share screen and save screen open
  over them when something is shared. (Today `App.tsx` only renders `SharedTextScreen`.)
- **SaveBookmarkScreen**: cleaned link, title, tag input + chips, note, Save.
- **BookmarksScreen**: search field, tag filter chips, list (infinite scroll), empty state that
  explains how to save from the share sheet.
- **BookmarkDetailScreen**: card, open/share/edit/delete, tags, later preview + embed.
- **TagsScreen** (Settings → Tags, phase 1): every tag with its bookmark count (unused tags show 0
  and stay), rename/merge, delete (also in use: removes it from those bookmarks, after a confirm).
- **SettingsScreen**: previews toggle, AI options, export/import, rules revision.

## Phases

Each phase ends with a commit series on `bookmarks` (or a branch stacked on it), tests green, and a
build installed on the iPhone and an Android device.

### Phase 0 – Groundwork
- [x] **Storage spike**: `turso` confirmed for local storage (see [results](#storage-spike-results)).
  Still open, for phase 8: two-device sync conflicts, attaching a local file to a new cloud
  database. Still open, for phase 1: running on real phones (only cross-compiled so far).
- [x] `crates/bookmarks`: schema v1, migrations, store, tags, search, index upkeep; 25 tests.
- [x] `link-cleaner`: `links()` returns each link with its cleaned form and provider.
- [x] App shell with tabs and simple screen state in `App.tsx`.
- [x] Share plugin: `intent` field in `SharedContent`, Rust + Kotlin + Swift; existing flow sends `"share"`.
- **Done when:** app opens on the Bookmarks tab (empty), sharing still opens the clean screen.

#### Storage spike results

Checked with `turso` 0.8.3 (12 Oct 2026) on macOS, plus release builds for `aarch64-apple-ios` and
`aarch64-linux-android`:

| Question | Result |
|---|---|
| Builds for iOS and Android | Yes. iOS needs the deployment target the app uses (15.0); Rust's default is too old and fails to link zstd (`___chkstk_darwin`). Tauri passes the app's target, so this only matters when building the crate by hand |
| `PRAGMA user_version`, foreign keys + cascade, `STRICT`, upsert, `RETURNING`, triggers, transactions | All work |
| FTS5 | Not available (`no such module: fts5`) |
| Turso full-text index | Works and follows edits, but needs `experimental_index_method(true)` when opening. `word*` prefix queries match nothing; the `ngram` tokenizer gives search-as-you-type. Per-column tokenizer syntax from the docs isn't supported yet; a table-wide `WITH (tokenizer = 'ngram')` is |
| Ranking (`fts_score`) | Higher = better match (some docs said lower). Two bugs: `ORDER BY fts_score(…) DESC, created_at DESC` next to `fts_match` sorts wrongly, and in the working outer-query form `fts_score` returns 0 for a `?` parameter. Worked around in `store/list.rs`; a test fails if either changes |
| Index upkeep | Turso never compacts the index; deletes leave tombstones. `OPTIMIZE INDEX` works (one index or all) |
| `MATCH` operator | Works, although the docs say only `fts_match()` does; we use `fts_match()` |
| Data survives closing and reopening a file | Yes, including the search index |
| Sync between two devices | Not tested yet (needs a Turso Cloud database). The sync engine replays changes by primary key, which is why IDs are explicit text keys |

### Phase 1 – Save and list (Android + iOS)
- [x] `Store::save/get/find_by_url/list/delete`, duplicate merge, unit tests.
- [x] Run the store on devices: Android emulator (API 34), iOS simulator, and a real iPhone 13 Pro
      (save, list, restart). Still to do: a real Android phone.
- [x] Tauri commands `bookmark_prepare`, `bookmark_save`, `bookmark_list`, `bookmark_get`, `bookmark_delete`
      (`src-tauri/src/bookmarks.rs`; the store lives in a swappable `Library` holder).
- [x] `tag_list`, `tag_rename`, `tag_delete` commands (for the tag dropdown and Settings → Tags).
- [x] Settings → Tags: every tag with how many bookmarks use it (unused tags show "Unused" and
      stay until deleted); rename inline (warns and merges into an existing name); delete, also
      for tags in use, after an inline confirm that says how many bookmarks it will be removed from.
- [ ] Android `activity-alias` "Save bookmark"; finish after save.
- [ ] If Android sharing shortcuts exist by then ([roadmap](ROADMAP.md#share-sheet-position)),
      add "Save bookmark" as one.
- [ ] iOS `SaveExtension` target in `project.yml`, App Group handoff, `cleanshare://save`.
- [x] Interim: "Save bookmark" button on the share screen, and the Bookmarks tab lists saved
      bookmarks (tap opens the cleaned link).
- [x] Save screen: cleaned link, title (suggested from the shared text), tags (chips; dropdown of
      existing tags, most used first, filtered as you type; "Add 'xyz'" for new ones), note. Opens
      from the share screen's Save button, or directly for a "save" share. If the link is already
      saved, the same screen edits it (`bookmark_update`, `bookmark_set_tags`).
- [ ] Detail screen (open, share, edit, delete with the confirm sheet).
- **Done when:** a YouTube, Instagram and X link can be saved from each app on both phones, appear
  cleaned in the list, and saving the same link twice shows the existing bookmark.

### Phase 2 – Tags, search, suggestions (tier 1)
- [x] Store: tags, filter by tags (AND), search-as-you-type (ngram), rename/merge/delete tags.
- [ ] Tag input with autocomplete; tag filter and search UI.
- [ ] `crates/tag-suggest` `MatchSuggester` + tests; `tag_suggest` command; chips on the save screen.
- **Done when:** search finds by any word in title/note/tags; suggestions appear for repeat sites.

### Phase 3 – Preview cards
- [ ] Settings screen + `previews_enabled`.
- [ ] `crates/link-preview`: oEmbed (YouTube, X), Open Graph parsing, limits, fixtures tests.
- [ ] Background fetch queue, thumbnail cache, `preview_status`, retry.
- [ ] Cards in list and detail; description added to FTS and to tag suggestions.
- **Done when:** with previews on, YouTube and article links show title + thumbnail offline after
  saving; with previews off, no request is made (checked with a proxy).

### Phase 4 – AI tagging
- [ ] `plugins/on-device-ai` (Swift Foundation Models, Kotlin ML Kit GenAI), availability check.
- [ ] Decide cloud option (own key vs. proxy); implement behind the same trait; opt-in setting.
- [ ] Suggestions show their source; accepted tags store it.
- **Done when:** on a supported phone, AI suggestions appear within ~2 s, offline; on others the
  option is hidden or cloud is offered.

### Phase 5 – Embeds
- [ ] `embed_kind` / `embed_url` from providers; "Show embed" for YouTube and X; sandboxed iframe.

### Phase 6 – Organize
- [ ] Edit screen; `reclean_all` on rules updates with a report. (Tag management moved to
      phase 1: Settings → Tags.)

### Phase 7 – Import / export
- [ ] Export JSON (full) and Netscape HTML; import browser HTML, Pocket and Raindrop exports (cleaned).

### Phase 8 – Accounts and paid sync
- [ ] Pick auth provider (Clerk or Auth0); Sign in with Apple and Google.
- [ ] Turso org JWKS set up; JWT template scoped to one database per user.
- [ ] `backend/`: master DB, `POST /db` (look up or create), billing webhooks, `DELETE /account`.
- [ ] App: sign in, open the database in sync mode, push after writes, pull on open and on a timer.
- [ ] First sync from an existing local-only database (upload or attach, per the spike).
- [ ] App Store and Play subscriptions; read-only grace period when a subscription ends.
- [ ] Account deletion in the app.
- **Done when:** sign in on phone A, save bookmarks, uninstall, reinstall, sign in: all bookmarks are
  back. Phone B shows the same bookmarks, offline edits on both merge.

## Testing

| Layer | How |
|---|---|
| `crates/bookmarks` | In-memory database + one file test: save/merge/dedupe, same IDs on two "devices", tags, search, paging, migrations, newer-schema refusal (`cargo test` in `crates/bookmarks`) |
| Sync | Two local databases against a test Turso database: offline edits on both, push/pull, compare |
| Backend | Function tests with a test master DB: create once, look up again, delete removes the DB |
| `crates/link-preview` | Saved HTML/oEmbed fixtures; limit and redirect handling with a local test server |
| `crates/tag-suggest` | Table tests: content + existing tags → expected ranking |
| Share entries | Manual checklist per platform (YouTube, Instagram, X, Safari/Chrome) |
| Privacy | With previews and AI off, run a save flow through a proxy and confirm zero requests |

## Privacy

- Bookmarks stay on the device unless the user turns on sync. No analytics; no account without sync.
- With sync, bookmarks are stored in the user's own Turso Cloud database. It is encrypted in transit
  but **not end-to-end**: Turso (and we, via the admin key) could read it. The privacy policy and
  the sync sign-up screen say so.
- Network use, each opt-in and listed in Settings and the privacy policy:
  - sync (the user's database in Turso Cloud, sign-in via Clerk/Auth0, our backend function)
  - rules download (see `docs/ROADMAP.md`)
  - link previews (contacts the saved site / its oEmbed service)
  - cloud AI tagging (sends title, description, domain to the chosen provider)
  - embeds (the embedded site sees the view, only when tapped)

## Accounts and sync

Planned for phase 8, opt-in and paid. The free app stays local-only with no account.

### Two modes

| Mode | Where bookmarks live | Account | Survives uninstall |
|---|---|---|---|
| **Local** (free, default) | One database file on the phone | None | Only through OS backups (see below) |
| **Sync** (paid) | Local file + the user's own database in Turso Cloud | Yes | Yes: sign in again and everything downloads |

In both modes reads and writes go to the local file, so the app works the same offline. Sync adds
`push()` (send local changes) and `pull()` (fetch remote changes) on top.

### Why one database per user

SQLite (and so libSQL/Turso) has no row-level security. With one shared database, any key that
opens it opens everyone's bookmarks, so phones couldn't connect directly: we'd need our own API
server adding `WHERE user_id = ?` to every query, any mistake leaks data, and we'd lose Turso's
offline sync.

With **one database per user**, each phone gets a token that only opens its own database, Turso
enforces the isolation, and sync runs straight from the phone. Turso is designed for this pattern
and per-database cost is low (see [Cost](#cost)).

### Master database

A small database only the backend can access. Phones never connect to it, so the shared-database
problem above doesn't apply.

```text
account
  id             text primary key    user id from the auth provider
  db_name        text not null       e.g. "u-<userid>" (the user's Turso database)
  plan           text not null       "free" | "sync" | "grace"
  plan_until     timestamp           end of the paid period / grace period
  created_at     timestamp not null
  deleted_at     timestamp
```

Naming the database after the user id (`u-<userid>`) means the app could find it without a lookup;
the table is still the source of truth for billing state and deletion.

### Keys: nothing long-lived on the phone

- The Turso **admin key** (creates and deletes databases) lives only in the backend.
- The phone gets a **sign-in token (JWT)** from Clerk/Auth0 at login. Turso is set up with the
  provider's JWKS endpoint and a JWT template scoped to the user's own database, so that token is
  the database key. During Turso's beta only Clerk and Auth0 were supported as providers; check
  again before choosing.
- Tokens expire and the auth library refreshes them. On uninstall the token is gone, which doesn't
  matter: logging in again issues a new one.

### Sign in (first time, after reinstall, or on a new device)

1. Sign in with Clerk/Auth0 (Sign in with Apple / Google).
2. The app gets a JWT from the provider.
3. The app calls the backend `POST /db` with the JWT. The backend checks it, looks up `account`
   and returns the database URL. For a new paid user it creates the database first.
4. The app opens the local file in sync mode with that URL and the JWT. If the local file is empty
   (fresh install, new device), the first open downloads the whole database.
5. From then on: push after writes, pull on open and periodically.
6. Sign out: stop syncing and offer to delete the local copy.

**Switching from local to sync:** the existing local file must reach the new cloud database. The
spike finds out whether it can be attached directly or must be uploaded once when the user
subscribes.

### Backend (`backend/`, one serverless function)

| Endpoint | Does |
|---|---|
| `POST /db` | Verify JWT → look up or create the user's database → return its URL |
| `POST /billing/apple`, `POST /billing/google` | App Store / Play subscription events → update `plan` |
| `DELETE /account` | Delete the user's Turso database and mark the `account` row deleted |

- **Subscription ends:** cloud database becomes read-only for a grace period (e.g. 30 days), then is
  deleted. The local copy stays on the phone in local mode.
- **Account deletion** is available inside the app (Apple requires it for apps with accounts).

### Local-only users and uninstall

Without sync there is no cloud copy. To limit data loss:
- **Android Auto Backup** is on (our manifest doesn't disable `allowBackup`), so app files back up to
  the user's Google account. Check the database file is included and stays under the 25 MB limit.
- **iOS device backups** (iCloud or computer) include the app's data folder, so restoring a phone
  restores bookmarks. Deleting and reinstalling the app does not.
- **Export / import** (phase 7) as a manual backup.
- **Settings note:** "Your bookmarks are only on this phone. Turn on sync to keep them across
  reinstalls and devices."

### Cost

Turso Cloud plans as of October 2026 ([pricing](https://turso.tech/pricing)):

| Plan | Price / month | Databases | Storage | Rows written | Sync traffic |
|---|---|---|---|---|---|
| Free | $0 | 100 | 5 GB | 10M | 3 GB |
| Developer | $4.99 | Unlimited | 9 GB (+$0.75/GB) | 25M (+$1/M) | 10 GB (+$0.35/GB) |
| Scaler | $24.92 | Unlimited | 24 GB (+$0.50/GB) | 100M (+$0.80/M) | 24 GB (+$0.25/GB) |

Free is enough for development; Developer covers early paid users. Bookmarks are small (text plus
a few tags), so storage per user stays in the kilobytes to low megabytes. Thumbnails stay on the
phone and are not synced. Third-party sources quote other figures, so check the official page
before setting a sync price.

### Alternatives considered

| Option | Why not (for now) |
|---|---|
| One shared Turso database + our own API | We'd enforce isolation in every query, and lose direct offline sync |
| Export / import via iCloud Drive or Google Drive | No backend, but manual, no live sync |
| Platform sync (CloudKit on iOS, Drive app data on Android) | No own backend, but no cross-platform sync |
| Own backend with end-to-end encryption | Most private, most work; could replace Turso Cloud later |
| CRDT-based sync (e.g. Automerge) | Good merging, still needs a relay and a different storage model |
| Toasty ORM on Turso | Released driver is on turso 0.7 (`main` has 0.8.3, unreleased) and search stays raw SQL; see [ADR 0001](adr/0001-bookmarks-storage-turso-crate-vs-toasty.md) |

## Risks

| Risk | Mitigation |
|---|---|
| Instagram/X pages return little metadata without login | oEmbed where possible; shared text as fallback; user edits title |
| A cleaning rule breaks a link | `original_url` kept; detail screen can offer "open original" in a debug menu |
| On-device AI only on newer phones | Tier 1 works everywhere; cloud is optional |
| iOS can't return to the source app after saving | Later: in-extension save form (DB in App Group container) |
| Embeds bring trackers | Tap to load, nocookie domains, notice for X |
| Schema changes over time | `user_version` migrations, tested from every past version |
| `turso` crate or its sync is still young | Pinned to an exact version; everything goes through `Store` and `db.rs`, so `libsql` stays a possible fallback; local mode never depends on the cloud |
| Full-text index is experimental in Turso | Enabled in one place (`db.rs`); tests cover matching and ranking order; workarounds for 0.8.3 bugs are commented in `store/list.rs`; if it breaks, `Store::list` can fall back to `LIKE` (bookmarks are small) |
| Turso docs and library disagree (score direction, prefix `*`, per-column tokenizers) | Behaviour is pinned by tests, not by the docs; re-run them on every Turso upgrade |
| Sync conflicts between devices | Content-derived IDs make same-link saves one row; `updated_at` per row; test with two devices in phase 8 |
| Same link saved offline on two phones after a rules change gives two IDs for one URL | Rare; `reclean_all` (phase 6) merges duplicates by URL |
| Turso Cloud can read synced data | Stated on the sync screen and in the privacy policy; end-to-end encryption as a later option |
| Turso pricing or terms change | One database per user can be exported as plain SQLite files and moved elsewhere |
| Local-only user uninstalls | OS backups, export, and a clear note in Settings |

## Open questions

- Save form inside the iOS extension (never leave the source app) – worth doing right after phase 2?
- Cloud AI: user's own key or a proxy we host?
- Should "Clean & Share" also offer "Save too"?
- Article text extraction ("reader mode") for better tags and search – phase 3 or later?
- Separate app icon / name for bookmarks if it's ever split out?
- Paid sync in the first release, or after phases 1–3?
- Clerk or Auth0? (Pricing, Sign in with Apple support, Tauri/mobile SDKs.)
- Where does the backend function run (Cloudflare Workers, Vercel, Fly), and is the master DB a
  Turso database too?
- Sync price, and the grace period after a subscription ends?

## References

- Turso: [pricing](https://turso.tech/pricing) ·
  [Developer plan](https://turso.tech/blog/turso-cloud-debuts-the-new-developer-plan) ·
  [offline sync beta](https://turso.tech/blog/turso-offline-sync-public-beta)
- Rust: [SDK](https://docs.turso.tech/sdk/rust) ·
  [reference (sync, push/pull)](https://docs.turso.tech/sdk/rust/reference.md)
- Search: [Turso FTS](https://docs.turso.tech/sql-reference/functions/fts) (newer than 0.8.3 in places) ·
  [Beyond FTS5](https://turso.tech/blog/beyond-fts5)
- Auth: [authorization](https://docs.turso.tech/sdk/authorization) ·
  [JWKS (Clerk, Auth0)](https://docs.turso.tech/sdk/authorization/jwks)
- Toasty: [ADR 0001](adr/0001-bookmarks-storage-turso-crate-vs-toasty.md) ·
  [Toasty now fully supports Turso](https://turso.tech/blog/toasty-now-fully-supports-turso) ·
  [Toasty](https://github.com/tokio-rs/toasty)
- Background: [Expo + Turso offline-first](https://expo.dev/blog/build-offline-first-mobile-apps) ·
  [Turso guide 2026](https://www.oflight.co.jp/en/columns/turso-edge-sqlite-libsql-2026) ·
  [comparedge pricing](https://comparedge.com/tools/turso/pricing) (third-party; figures differ from Turso's page)
