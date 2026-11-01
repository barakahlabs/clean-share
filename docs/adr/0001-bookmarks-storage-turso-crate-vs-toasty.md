# ADR 0001: Bookmarks storage – plain `turso` crate, not Toasty (for now)

- **Status:** Accepted
- **Date:** 2026-11-01
- **Scope:** `crates/bookmarks` (see [`docs/BOOKMARKS.md`](../BOOKMARKS.md))
- **Revisit when:** the next Toasty release (0.12) ships. Its `main` branch already uses
  `turso` 0.8.3 (see [Revisit](#revisit))

## Context

Bookmarks are stored in one Turso database per user: a local file on the phone, optionally synced
to that user's own Turso Cloud database (paid sync, phase 8). The storage crate must:

- run inside a Tauri app on iOS and Android;
- support full-text search-as-you-type (ngram index) with relevance ranking;
- use our own schema: content-derived text IDs, `STRICT` tables, case-insensitive unique tag
  names, cascading deletes;
- migrate on each phone at open, and refuse a database written by a newer app version;
- later open the same database in sync mode without changing any query code.

`crates/bookmarks` is built on the plain [`turso`](https://crates.io/crates/turso) crate
(0.8.3), with hand-written SQL behind a `Store` type. 25 tests pin its behaviour, including two
workarounds for Turso 0.8.3 search bugs.

[Toasty](https://github.com/tokio-rs/toasty) is an async ORM from the Tokio project. Since
October 2026 it [fully supports Turso](https://turso.tech/blog/toasty-now-fully-supports-turso),
including sync. This ADR compares the two.

Versions compared: `turso` **0.8.3** (12 Oct 2026) and `toasty` / `toasty-driver-turso` **0.11.0**
(25 Sep 2026). Toasty facts were checked in the published 0.11.0 source, not only its docs. Toasty's
`main` branch (checked at `26b4df2`, 30 Oct 2026) is noted where it differs.

## Where they differ

| Feature | Plain `turso` 0.8.3 (current) | Toasty 0.11.0 | Matters for us? |
|---|---|---|---|
| **Turso engine version** | 0.8.3, the version our tests and workarounds were written against | Released 0.11.0 requires `turso ^0.7` (0.7.x only). `main` moved to `turso` 0.8 on 29 Sep ([#1268](https://github.com/tokio-rs/toasty/pull/1268)) and locks 0.8.3, our exact version, but that isn't released yet | **Yes, until 0.12 ships.** Today: downgrade and retest, or depend on an unreleased git commit |
| **Query style** | Hand-written SQL strings | Typed models (`#[derive(Model)]`), generated queries, `create!` / `query!` / `update!` macros | Some. Toasty removes boilerplate for simple queries |
| **Mapping rows to structs** | By hand (`store/row.rs`, ~90 lines) | Automatic for model queries | Some. Small saving at our size (~15 queries) |
| **Full-text search queries** | `fts_match` / `fts_score` in SQL, with our workarounds | Not in the query builder; only via raw SQL (`toasty::sql::query`) | **Yes.** Our main query stays raw SQL either way |
| **Raw SQL results** | Rows read into our structs | "Toasty does not … hydrate models from raw query results"; returns dynamic `Value` records | **Yes.** Search results would still be mapped by hand |
| **Schema definition** | Hand-written DDL in `migrate.rs` | Generated from models (`push_schema`, generated migration SQL) | Some. See the next three rows |
| **`STRICT` tables, `COLLATE NOCASE`, `ON DELETE CASCADE`** | Written directly in the DDL | No sign of them in the 0.11.0 source; only possible by hand-editing migration SQL | **Yes.** Our tag and delete rules depend on them |
| **Search index DDL (`USING fts … WITH (tokenizer = 'ngram')`)** | Written directly in the DDL | No model attribute; hand-editing migration SQL needed | **Yes** |
| **Migration tracking** | `PRAGMA user_version` | `__toasty_migrations` table (id, name, applied time) | No. Either works |
| **Refusing a newer schema** (synced from a phone with a newer app) | Built in: `Error::NewerSchema` | Not built in, as far as the source shows: unknown applied IDs are skipped, not rejected | **Yes, for sync.** We'd add it ourselves |
| **Runtime** | Any executor (Tokio only in our tests) | Tokio required | No. Tauri runs on Tokio |
| **Concurrent writes (MVCC)** | Not used | `concurrent_writes()` wraps transactions in `BEGIN CONCURRENT` | No. One app, one writer |
| **Remote over HTTP (serverless)** | Not used | `serverless` feature | No. We sync a local file instead |
| **Relations** | Joins written by hand | `#[has_many]`, `#[belongs_to]`, `Deferred` | Little. We have one join table |
| **Release pace** | 0.8.0 → 0.8.3 in two weeks, no API breaks seen | Five breaking `0.x` releases in five months (0.7 May → 0.11 Sep); the launch post expects "churn" | **Yes.** Repeated upgrade work |
| **Maturity** | Engine maintained by Turso | Launched April 2026 as "an early release … without much real-world usage (yet)" | Some |

## Where they are equal

- **Same database engine:** both run Turso's Rust engine in the app, so the database file is the same.
- **Local files and in-memory:** both open either one.
- **Sync with Turso Cloud:** both offer remote URL, auth token, a token callback, `push()` and `pull()`.
- **Sign-in tokens:** both accept a per-request token function, so our JWKS plan works with either.
- **Full-text index:** both can enable it (`fts` feature + `experimental_index_method`).
- **Raw SQL:** both can run any SQL with numbered parameters (`?1`, `?2`).
- **Migrations inside the app:** both apply migrations at runtime on each phone.
- **Async:** both are async throughout.
- **Transactions:** both support them.
- **Mobile:** both build on the same engine, which we cross-compiled for iOS and Android.
- **Multi-tenancy:** both open one database per user; neither needs a user ID in queries.

## Decision

Keep `crates/bookmarks` on the plain `turso` crate. Do not adopt Toasty now.

## What holds us back, and why

1. **Engine version (blocker for now).** The released `toasty-driver-turso` 0.11.0 requires
   `turso ^0.7`. Our search behaviour, the ngram tokenizer results, and the workarounds for two
   ranking bugs were all found and tested on 0.8.3. Moving to 0.7 means redoing that work on an
   older engine, and using both versions would put two database engines in the app. Tested on
   2026-11-01: our crate builds on `turso` 0.7.2, but 3 of 25 tests fail, all in search. Matching
   across the five indexed columns returns nothing for words that are in the data, while a
   single-column match and `LIKE` find them. So on 0.7 we'd have no working search. Toasty's
   `main` already uses 0.8.3, so this ends with the next release. Depending on an unreleased git
   commit in the meantime isn't worth it for a project that breaks its API every month.
2. **Our hardest code doesn't get easier.** The search query (ranking in an outer query, search
   text written into the SQL) is raw SQL in both. Toasty returns raw results as dynamic values,
   so we'd still map them by hand. Toasty helps most with simple CRUD, which is already short.
3. **Our schema rules sit outside Toasty's model.** `STRICT` tables, case-insensitive unique tag
   names, cascading deletes and the ngram index have no model attributes. They'd live in edited
   migration SQL, so the models would no longer be the full truth about the schema.
4. **The sync safety check isn't built in.** A phone must refuse a database migrated by a newer
   app on another phone. Our store does this; with Toasty we'd write it ourselves.
5. **Upgrade churn.** Breaking releases roughly every month mean regular porting work for a small
   team, on top of Turso's own upgrades.

None of these is permanent, and the costs of waiting are low.

## Consequences

- We keep writing SQL by hand and map rows in `store/row.rs`.
- Everything database-related stays inside `Store` and `db.rs`. Switching later replaces the
  inside of the store, not the app or the API.
- Our 25 tests are the contract: any future engine or ORM must pass them unchanged.

## Revisit

Look again when **any** of these happens:

- Toasty 0.12 (or any release with `turso` 0.8+) ships. This is expected soon: `main` has had
  `turso` 0.8.3 since 29 Sep 2026. Then port `Store` on a branch and run the tests. Reasons 2–5
  above still apply, so the port decides whether the saving outweighs them;
- Toasty adds model attributes for collation, cascading deletes, `STRICT`, or full-text indexes;
- our query count grows enough that row mapping becomes a real cost (e.g. phase 6 organize
  features, phase 7 import/export).

## References

- Toasty: [GitHub repository](https://github.com/tokio-rs/toasty) ·
  [release announcement, April 2026](https://tokio.rs/blog/2026-04-03-toasty-released) ·
  [0.10.0 README](https://docs.rs/crate/toasty/0.10.0/source/README.md) ·
  [roadmap](https://tokio-rs.github.io/toasty/nightly/dev/roadmap/index.html) ·
  [query constraints roadmap](https://tokio-rs.github.io/toasty/nightly/dev/roadmap/query-constraints.html) ·
  [source on docs.rs](https://docs.rs/toasty/latest/src/toasty/lib.rs.html)
- Toasty crates (0.11.0 source read for this ADR): [`toasty`](https://crates.io/crates/toasty) ·
  [`toasty-driver-turso`](https://crates.io/crates/toasty-driver-turso) (`Cargo.toml`: `turso`
  version `0.7`; `src/lib.rs`: sync builder, `__toasty_migrations`) ·
  [workspace `Cargo.toml` on `main`](https://github.com/tokio-rs/toasty/blob/main/Cargo.toml)
  (`turso = { version = "0.8" }`, `Cargo.lock`: 0.8.3) ·
  [#1268: update turso to 0.8](https://github.com/tokio-rs/toasty/pull/1268) · `toasty/src/sql.rs` (raw SQL,
  no hydration) · `toasty/src/migration/embed.rs` (`MigrationSet::apply`)
- Turso + Toasty: [Toasty now fully supports Turso, Oct 2026](https://turso.tech/blog/toasty-now-fully-supports-turso) ·
  [Turso docs: Toasty](https://docs.turso.tech/sdk/rust/orm/toasty.md) (still says local-only;
  older than the blog post)
- Turso crate: [`turso` on crates.io](https://crates.io/crates/turso) ·
  [Rust SDK](https://docs.turso.tech/sdk/rust) ·
  [Rust reference (sync)](https://docs.turso.tech/sdk/rust/reference.md)
- Turso full-text search: [docs](https://docs.turso.tech/sql-reference/functions/fts) ·
  [Beyond FTS5](https://turso.tech/blog/beyond-fts5)
- Our findings: [storage spike results](../BOOKMARKS.md#storage-spike-results) ·
  `crates/bookmarks/src/store/list.rs` (search workarounds) ·
  `crates/bookmarks/src/store/tests.rs` (`search_puts_the_best_match_first`)
