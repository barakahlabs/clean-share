# Bookmarks

Status: **draft**, to be expanded. Not implemented yet.

Save links shared from other apps, tag them, and find them later. Bookmarks live inside Clean Share,
so every saved link is cleaned of tracking first.

## Goals

- Save a link in two taps from any app's share sheet, without leaving that app for long.
- Tag links and find them again quickly (search, filter by tag).
- Store clean links only: no share tokens, no tracking parameters.
- Local-first: works offline, nothing leaves the device unless sync is explicitly turned on later.

## Non-goals (for now)

- Accounts, sync between devices, sharing collections with others (see [Later: sync](#later-sync)).
- Full-text archiving of page contents or offline reading.
- Browser extensions.

## Decisions so far

| Decision | Choice | Why |
|---|---|---|
| Separate app or combined | Combined into Clean Share | Same entry point (share sheet), cleaning is the first step of saving, one app to maintain |
| Storage | Local SQLite on the device | Offline, private, fits "collects nothing" |
| Link cleaning | Reuse `crates/link-cleaner` + `rules/` | Same rules as Clean & Share; dedupes links that differ only by tracking |
| Share sheet entries | Two: **Clean & Share** and **Save bookmark** | Each entry does one thing; users pick the intent up front |

The code stays split (core crates, share plugin, UI) so bookmarks could move into a separate app later
without rewriting it.

## User flows

### Save from another app

1. In Instagram / YouTube / X / a browser: **Share → Save bookmark**.
2. Clean Share opens a small save screen:
   - cleaned link (tracking shown in red, as in the share screen)
   - title (from the shared text if present, editable)
   - tag input with suggestions from existing tags
3. **Save** → back to the source app.

If the link is already saved, the screen shows the existing bookmark instead of creating a duplicate,
with the option to add tags.

### Browse and find

- List of bookmarks, newest first.
- Search across title, URL, tags and note.
- Filter by one or more tags.
- Tap a bookmark → open the link in the browser, share it (cleaned), edit, or delete.

### Organize

- Add, rename, merge and delete tags.
- Edit a bookmark's title, note and tags.

## Share sheet entries

| Platform | Clean & Share (exists) | Save bookmark (new) |
|---|---|---|
| Android | `ACTION_SEND` intent-filter on `MainActivity` (added by `plugins/share-target`) | Second entry: an `activity-alias` with its own label and intent-filter; the plugin reports which entry was used |
| iOS | `ShareExtension` target: saves text to the App Group, opens the app | Either a second Share Extension target ("Save bookmark") or a choice inside the existing extension. A second target is clearer in the share sheet |

The share plugin's `get_shared` result gains a field for the entry used, e.g.
`{ text, intent: "share" | "save" }`, and the app opens the matching screen.

## Data model (first version)

```text
bookmark
  id            integer, primary key
  url           text      cleaned URL (unique)
  original_url  text      as shared, kept for debugging rule issues (optional, may drop)
  title         text      nullable
  note          text      nullable
  source_app    text      nullable, e.g. "instagram" (provider id from rules/, or Android package)
  created_at    timestamp
  updated_at    timestamp

tag
  id            integer, primary key
  name          text      unique, case-insensitive

bookmark_tag
  bookmark_id   → bookmark.id
  tag_id        → tag.id
  primary key (bookmark_id, tag_id)
```

- `url` is the **cleaned** URL, so the same post shared by different people is one bookmark.
- When rules change (e.g. a new tracking parameter is found), a migration can re-clean saved URLs
  and merge duplicates.
- Search: SQLite FTS5 table over title, url, note and tag names.

## Where the code goes

| Piece | Location | Notes |
|---|---|---|
| Storage + queries | new crate `crates/bookmarks` (Rust, SQLite via `rusqlite`) | Pure Rust, unit-testable without the app |
| Tauri commands | `src-tauri/src/` | `save_bookmark`, `list_bookmarks`, `search`, `tag` operations |
| Share entry routing | `plugins/share-target` | Report which share entry was used |
| UI | `src/screens/` | `SaveBookmarkScreen`, `BookmarksScreen`, `BookmarkDetailScreen` |
| iOS extension | `src-tauri/gen/apple/` (+ `project.yml`) | Second extension target if chosen |

The database lives in the app's data directory. Exporting is a plain file the user controls.

## Privacy

- Bookmarks stay on the device. No analytics, no account.
- The only network request remains the optional rules download (see `docs/ROADMAP.md`).
- Optional later: fetching page titles or previews. This contacts the saved site, so it should be
  opt-in and explained.

## Import and export

- Export: JSON (full fidelity) and Netscape bookmark HTML (works with browsers).
- Import: browser HTML exports, Pocket / Raindrop exports. Imported links are cleaned too.

## Later: sync

Undecided. Options to evaluate when there's a real need:

| Option | Trade-off |
|---|---|
| Export / import file via iCloud Drive or Google Drive | No backend, manual |
| Platform sync (iCloud CloudKit on iOS, Drive app data on Android) | No own backend, but no cross-platform sync |
| Own backend with end-to-end encryption | Cross-platform, most work, needs accounts |
| CRDT-based local-first sync (e.g. Automerge) | Good offline/merge story, still needs a relay |

## Phases

1. **Save and list**: "Save bookmark" share entry on Android, SQLite storage, list screen, open/share/delete.
2. **Tags and search**: tag input with suggestions, tag filter, FTS search.
3. **iOS**: second share extension (or in-extension choice), App Group handoff.
4. **Organize**: edit, tag management (rename/merge), duplicate handling on rule changes.
5. **Import / export**.
6. **Sync** (only if needed).

## Open questions

- Keep `original_url` at all, or drop it once cleaning is trusted?
- Save screen inside the main app, or a lightweight in-extension UI on iOS so users never leave the
  source app?
- Fetch titles automatically (network request) or only use the shared text?
- Should "Clean & Share" also offer "Save too"?
- Separate app icon / app name for bookmarks if it's ever split out?
