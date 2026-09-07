# `.waffle` v4 — Document Model: Identity, Git-Aware Sources, Scoped References

Status: **PHASE 1 IN PROGRESS** (spec 2026-09-07). Plan of record for the
file-format changes that assemblies, KiCad board links, multi-document
assemblies, derived 2D drawings, and model-facing tooling (MCP) all depend on.

Supersedes the "future tab kinds" note in `docs/FILE_FORMAT.md` §5.3 and the
assembly-file-format milestone (M6) of `projects/10-assemblies/PLAN.md`. The v3
description in `docs/FILE_FORMAT.md` stays authoritative for everything this
spec does not change.

---

## 1. Goal

A `.waffle` document can:

1. **Be identified** independently of where it is stored (`document.id`).
2. **Link to external content** — another `.waffle` document, a STEP file, a
   KiCad board — through a `sources` table whose locators are **git-aware**: a
   link names a remote, a path, and a ref that is either **pinned to a
   commit** or **floating on a branch or tag tip**, and records what commit
   and content hash were actually resolved last time. A fork or clone of a
   repo of parts keeps working because intra-repo links are relative.
3. **Reference geometry across scopes** — a face of instance N of part X in
   document Y — through a `scope` on geometry references (reserved in v4.0,
   consumed by the assembly and drawing tab kinds).
4. **Carry tab kinds an older reader does not know** without refusing the file
   or dropping the tab on resave (opaque preservation), so adding `Assembly`
   and `Drawing` later is **not** a wire-breaking change.
5. **Be authored and edited by tools** (an MCP server, a script, a model)
   without those tools having to fabricate derived data or lose annotations:
   a published JSON Schema, unknown-field preservation at the structural
   levels, a provenance table, and profile addressing by sketch entity ids.

User-visible behavior in Phase 1 (this spec's implementation scope): files
save as v4 through **one** writer (Rust), old files (v1–v3) open unchanged,
imported STEP payloads move into the `sources` table, the document gets a
stable id, and the app refuses nothing it accepted before. Assemblies,
drawings, and KiCad import are **designed here and implemented in later
phases** (§9); Phase 1 lands the substrate they need.

---

## 2. Parameters (the v4 data model)

All lengths meters, angles degrees, timestamps RFC 3339 UTC, ids lowercase
hyphenated UUIDs unless stated. Additive rules of v3 (§13 of the v3 spec)
still apply: new optional fields are serde-defaulted; new **enum variants**
of `Operation`, constraint, selector, or `PlaneDefinition` still require a
`MIN_READER_VERSION` bump. New **tab kinds** and **source kinds/locators** do
not (§2.5, §2.3).

### 2.1 Envelope

```json
{
  "format": "waffle-iron",
  "version": 4,
  "min_reader_version": 4,
  "document": {
    "id": "6f1c2a4e-…",
    "name": "Bracket",
    "created": "2026-09-07T18:00:00.000Z",
    "modified": "2026-09-07T18:04:12.331Z",
    "display_unit": "mm"
  },
  "sources": [ …SourceEntry ],
  "tabs": [ …Tab ],
  "active_tab": "9068ef01-…"
}
```

| Field | Type | Req/default | Rule |
|---|---|---|---|
| `format` | `"waffle-iron"` | ✔ | unchanged |
| `version` | u32 | ✔ | `4` |
| `min_reader_version` | u32 | ✔ | `4` for Phase-1 writers |
| `document.id` | UUID | writer ✔ / reader defaults | **Stable identity of the document.** Survives rename, move between storage providers, fork. Readers that find it absent (a hand-written file) mint one and warn; the next save persists it. Migration v3→v4 mints it. |
| `document.name/created/modified/display_unit` | | as v3 | unchanged |
| `sources` | SourceEntry[] | default `[]` | §2.3 |
| `tabs` | Tab[] | ✔ | `Tab.id` **must be a UUID string for new tabs**; readers keep accepting legacy free-form ids (`"default"`) and migration v3→v4 rewrites non-UUID ids to fresh UUIDs (rewriting `active_tab` too). The Rust type stays `String`. |
| `active_tab` | string | ✔ | as v3 |
| *any other key* | | | **preserved** on load and re-emitted on save (§2.6) |

### 2.2 Tab

```json
{ "id": "…", "name": "Part 1", "kind": { "type": "Part", "features": {…}, "preview_mesh": null } }
```

Unchanged shape. Known `kind.type` values in v4.0: `Part`. Reserved (§9):
`Assembly`, `Drawing`. **Unknown `kind.type` is preserved opaquely** (§2.5).
Unknown keys on `Tab` are preserved (§2.6).

### 2.3 SourceEntry — external content the document depends on

```json
{
  "id": "3b9e…",
  "name": "bracket.waffle",
  "kind": { "type": "Waffle" },
  "locator": {
    "type": "Git",
    "remote": "https://github.com/acme/parts",
    "path": "brackets/bracket.waffle",
    "ref": { "type": "Branch", "name": "main" },
    "host": "github"
  },
  "resolved": { "commit": "9fceb02a…40 hex…", "at": "2026-09-07T18:00:00Z" },
  "content_hash": "git-blob-sha1:2aae6c35c94fcfb415dbe95f408b9ce91ee846ed",
  "pack": false,
  "embed": null,
  "fetched_at": "2026-09-07T18:00:00Z"
}
```

| Field | Type | Req/default | Meaning |
|---|---|---|---|
| `id` | UUID | ✔ | Referenced by features (`source_id`), assembly instances, drawing views, and `scope.source_id`. Stable for the life of the document; **relinking** (pointing the entry at a new locator) keeps the id so dependents survive. |
| `name` | string | ✔ | Display name / diagnostics (usually the file's basename). |
| `kind` | SourceKind | ✔ | Tagged `type`: `Waffle`, `Step`, `KicadPcb`, `Mesh`, or **any other string, preserved opaquely** (a v4.0 reader shows "unsupported source kind" and keeps the entry). |
| `locator` | Locator | ✔ | Where the content lives (§2.4). |
| `resolved` | `{commit: string, at: timestamp}` \| null | default null | For git locators: the commit that was actually loaded last time. For a `Commit` ref it equals the ref; for `Branch`/`Tag` it is the tip at last sync. Absent for non-git locators. |
| `content_hash` | string \| null | default null | Algorithm-prefixed hash of the **exact bytes** last loaded. Phase 1 algorithm: `git-blob-sha1:<40 hex>` = SHA-1 of `"blob " + byte_len + "\0" + bytes`, i.e. the git blob id, which GitHub/GitLab/Gitea report for free and which is computable offline. Additional algorithms (`sha256:`) may be added; readers ignore prefixes they do not know. |
| `pack` | bool | default: `true` when `locator.type == "Embedded"`, else `false` | Writer policy: emit `embed` for this entry. A document with every referenced source packed is self-contained ("pack and go"). |
| `embed` | `{encoding: "deflate-base64", blob: string}` \| null | default null | Cached content bytes at `content_hash`. Readers use it without any network. Decoders reject unknown encodings loudly (same contract as the v3 STEP blob, `step_import::decode_step_blob`) and **cap inflation** (§6). |
| `fetched_at` | timestamp \| null | default null | When `embed`/`content_hash` were last refreshed from the locator. |
| *any other key* | | | preserved (§2.6) |

**Content resolution order** for a source id, at rebuild: (1) `embed` if
present and its hash matches `content_hash` (or `content_hash` is null);
(2) content the host supplied at runtime for that id (the app fetched it
through the locator — bridge message `ProvideSource`); (3) a legacy in-feature
payload (v3 `ImportedBody.blob`, still accepted). Otherwise the dependent
feature fails loudly with `SourceUnavailable { source_id, locator }`; the
document still loads.

### 2.4 Locator — git-aware addressing

Tagged `type`. Unknown types are preserved opaquely and reported as
unresolvable.

| Variant | Fields | Semantics |
|---|---|---|
| `Git` | `remote` (string), `path` (string, `/`-separated, no leading `/`), `ref` (GitRef), `host` (`"github"`\|`"gitlab"`\|`"gitea"`\|`"generic"`\|absent) | Content of `path` in the repository at `remote`, at `ref`. `remote` is the HTTPS clone URL **without** a trailing `.git` (normalized on write; readers accept either). `host` selects the API adapter (§7.2); absent ⇒ inferred from the remote's hostname, falling back to `generic`. |
| `Relative` | `path` (string, POSIX-relative, may contain `..`) | Resolved against the **document's own location**: if the document was opened from a git locator `{remote, path: P, ref}`, the target is `{remote, path: normalize(dirname(P)/path), ref}` with the **same ref** — so a clone, a fork, or a branch of a repo of parts keeps its intra-repo links. If the document was opened from a local folder (desktop host), it resolves on the filesystem. If the document has no location (browser-local, unsaved), a `Relative` link is unresolvable until the document is saved somewhere. |
| `Url` | `url` (string, `https://`) | Plain fetch. No ref resolution; `content_hash` is the only staleness signal. |
| `Local` | `provider` (string, e.g. `"local"`), `doc_id` (string) | A document in one of this browser's storage providers. **Not shareable**: any writer producing a file for download or for a git provider must refuse to emit a `Local` locator without `pack: true` (§6). |
| `Embedded` | — | The `embed` **is** the source (a file that came from a file picker or paste and has no origin). `pack` defaults to `true`. |

`GitRef` (tagged `type`):

| Variant | Fields | Pinned? |
|---|---|---|
| `Commit` | `sha` (40 hex; 64 hex accepted for SHA-256 repos) | **Yes.** Reproducible forever; "update" is a no-op. |
| `Branch` | `name` | No. `resolved.commit` is the tip at last sync; the app can show "tip moved" and offer update or pin. |
| `Tag` | `name` | Treated like `Branch` (tags can move); UIs may label it "release". |

**Pin and update semantics.** "Pin" rewrites `ref` to `Commit{resolved.commit}`.
"Update to tip" re-resolves the ref, fetches content at the **resolved commit**
(never at the moving ref name, so `resolved.commit` and `content_hash` always
agree), and rewrites `resolved`, `content_hash`, `fetched_at`, and `embed` if
`pack`. Both are explicit user (or tool) actions; a rebuild never fetches.

### 2.5 Unknown tab kinds and source kinds are preserved, not rejected

`Tab.kind` deserializes as *known kind or opaque JSON value*: a v4 reader that
meets `{"type": "Assembly", …}` before it implements assemblies keeps the tab
(shown as read-only "unsupported in this version"), keeps its JSON verbatim,
and re-emits it byte-for-byte on save. Same for `SourceEntry.kind` and
`SourceEntry.locator`. Consequence: **adding a tab kind, source kind, or
locator kind is NOT a `MIN_READER_VERSION` bump.** Adding an `Operation`
variant inside a Part still is (unchanged from v3) until the same opaque
treatment lands for features (§9, Phase 1b).

### 2.6 Unknown fields are preserved at structural levels

Envelope, `document`, `Tab`, `SourceEntry`, and `FeatureTree` carry a
flattened catch-all map: unknown keys survive load → save. Convention for
tool-added metadata: prefix with `x-` (`"x-agent-notes"`, `"x-kicad-refdes"`)
so a future official field cannot collide. Not (yet) preserved inside
`Feature`, `GeomRef`, sketch entities, or operation params — those are dense
Rust structs with ~70–120 literal construction sites each; tooling must not
stash data there.

### 2.7 Provenance

`FeatureTree.provenance: { "<feature_uuid>": Provenance }` (defaulted, omitted
when empty, garbage-collected on feature delete like `body_names`):

```json
{ "origin": { "type": "Agent", "name": "claude-fable-5-1" }, "at": "2026-09-07T18:02:00Z" }
```

`origin` ∈ `User` \| `Agent{name}` \| `Import{source_id}` \| `Derived{source_id, rule}`.
`Derived` marks features regenerated from a source (a KiCad board outline):
the UI shows them read-only and "re-sync" may replace them.

### 2.8 Scoped geometry references (reserved in v4.0)

```json
"scope": { "source_id": "3b9e…", "tab_id": "…", "instance_path": ["…", "…"] }
```

Reserved field on `GeomRef`, absent ⇒ local (the current tab). `source_id`
absent ⇒ same document; `tab_id` absent ⇒ same tab; `instance_path` = chain
of assembly-instance ids from the referencing assembly down to the owning
part instance. **Not implemented in Phase 1** (no consumer; 117 literal sites
would change). Lands with the first consumer (assembly mates), together with
the `MIN_READER_VERSION` bump that consumer needs anyway for its Part-side
use (in-context sketch projection). Assembly and Drawing tabs that carry
scoped refs are opaque to v4.0 readers per §2.5, so nothing is lost meanwhile.

### 2.9 Agent-friendly profile addressing

`ExtrudeParams` / `RevolveParams` gain `profile_entity_ids: Option<Vec<u32>>`
(defaulted). When present, the profile is the solved loop whose entity-id set
equals this set (order-insensitive); `profile_index` is ignored. A writer that
has not run the solver can now say "the loop made of entities 3,4,5,6" —
`Region.profile_entity_ids` already uses this identity. Resolution failure
(no such loop, or two loops with the same set) is a loud per-feature error.

### 2.10 Sketch `solve_status` becomes optional

`#[serde(default)]` with a new variant `Unsolved` as the default. A sketch
without the field parses; the engine's rebuild solves it and replaces the
status (today it recomputes positions/profiles when empty; the status must
follow). New variant ⇒ part of the v4 bump.

### 2.11 ImportedBody in v4

| Field | v3 | v4 |
|---|---|---|
| `file_name` | ✔ | ✔ (display) |
| `blob_encoding`, `blob` | ✔ required | optional, **legacy read path only**; v4 writers do not emit them |
| `source_id` | — | UUID of the `sources` entry holding the STEP content |
| `translation_m`, `rotation_deg`, `scale` | | unchanged |

Migration v3→v4 (§3): for each `ImportedBody` with a blob, create a source
`{kind: Step, locator: Embedded, pack: true, embed: {deflate-base64, blob},
content_hash: git-blob-sha1(decoded text), name: file_name}` and set
`source_id`; clear `blob`/`blob_encoding`. Two features with byte-identical
payloads share one source (dedup by hash).

The single-tree API (`load_project`/`save_project`, used by the assay
generators and by `KernelV2`-facing tests) keeps working: `load_project`
inlines each referenced source's embed back into the feature as a legacy
blob, and `save_project` lifts blobs out into sources. Round-trip is
byte-stable for the STEP text.

---

## 3. Branch table — reader and writer behavior

| Input | Reader behavior | Writer behavior |
|---|---|---|
| v1 file | migrate v1→v2→v3→v4 (scale, wrap in tab, mint document id, UUID tab id) | n/a |
| v2 file | v2→v3→v4 | n/a |
| v3 file, UUID tab ids | mint `document.id`, `sources: []`, lift ImportedBody blobs into sources | n/a |
| v3 file, `"default"` tab id | as above plus tab id → fresh UUID, `active_tab` rewritten | n/a |
| v4 file, known kinds | load | emit v4, `min_reader_version: 4` |
| v4 file, unknown `Tab.kind.type` | tab preserved opaque, flagged unsupported; not rebuildable; other tabs work | re-emit verbatim |
| v4 file, unknown `SourceEntry.kind`/`locator.type` | entry preserved; dependents fail `SourceUnavailable` | re-emit verbatim |
| v4 file, unknown keys at §2.6 levels | preserved | re-emitted |
| v4 file, unknown keys elsewhere | dropped (documented) | — |
| `document.id` missing | mint + warning | always emit |
| `version > 4` or `min_reader_version > 4` | `FutureVersion` (unchanged) | — |
| `sources[].embed` present, hash matches / hash null | use embed | keep |
| `sources[].embed` present, hash mismatch | ignore embed, warn `EmbedHashMismatch`, fall through to host-provided content | keep (do not "repair" silently) |
| source referenced, no embed, host provided content | use provided | write `embed` iff `pack` |
| source referenced, nothing available | dependent feature error `SourceUnavailable`; document loads | keep entry |
| `Local` locator, save to download / git provider | — | refuse unless `pack: true` (`LoadError::UnshareableSource`) |
| `Relative` locator, document has no location | unresolvable until saved somewhere | keep |
| `ImportStep` (file picker) | — | new source `{Step, Embedded, pack: true}` + feature with `source_id` |
| `profile_entity_ids` set | resolve by entity-id set; ignore `profile_index` | — |
| `profile_entity_ids` set, 0 or ≥2 matching loops | per-feature error | — |
| `solve_status` absent | `Unsolved`; rebuild solves | writers emit the solved status |

---

## 4. Invariants

1. **Identity is content-independent.** `document.id` is unchanged by save,
   rename, provider move, or tab edits. Only "Save as copy"/fork mints a new one.
2. **Round trip is lossless at structural levels.** For any v4 file,
   `save(load(file))` preserves every key at envelope/document/tab/source/
   feature-tree level, including unknown ones, and preserves unknown tab kinds
   byte-for-byte (after JSON canonicalization).
3. **Backward compatibility.** Every v1–v3 file that loaded before loads
   after, and rebuilds to the same topology (the 312-case assay corpus and the
   `format_tests` v1→v3 chains extend to v4).
4. **Resolved commit and content agree.** After any fetch, `content_hash` is
   the hash of bytes taken at `resolved.commit`, never at a moving ref name.
5. **Pinned means reproducible.** A `Commit` ref plus its `content_hash` fully
   determines the content; "update" never changes a pinned source.
6. **Relative links are location-invariant.** For a repo cloned/forked to a
   new remote, every `Relative` link resolves to the same relative path in the
   new remote at the document's own ref.
7. **One writer.** Every production save path (autosave, Ctrl+S, download,
   provider sync) produces its bytes in `file_format::save_document_verified`.
   No JavaScript code composes the envelope (the home page's empty-document
   template is the one exception: it is a constant, and the loader normalizes
   it — invariant 3 pins it).
8. **Verified save.** A file the loader would refuse is never emitted
   (v3 contract, extended to `save_document`).
9. **Hash algorithm is self-describing.** `content_hash` always carries its
   algorithm prefix; an unknown prefix is treated as "no hash", never as a
   mismatch.

---

## 5. Oracles

- `format_tests.rs`: v4 round trip (single tab, two tabs, with sources);
  unknown tab kind preserved byte-identical; unknown envelope/tab/source keys
  preserved; v3 → v4 migration mints `document.id`, rewrites `"default"`,
  lifts a STEP blob into a source with the expected git-blob SHA-1 (computed
  against a known vector: `git-blob-sha1` of the empty string is
  `e69de29bb2d1d6434b8b29ae775ad8c2e48c5391`; of `"hello\n"` is
  `ce013625030ba8dba906f756967f9e9ca394464a`); `load_project` inlines the
  blob back and `save_project` lifts it out (STEP text byte-identical);
  `Local` locator without pack refused on shareable save; hash mismatch falls
  through with a warning; `profile_entity_ids` resolves the annulus outer
  loop of a two-circle sketch and errors on a duplicated set; `solve_status`
  absent parses and rebuilds.
- Assay corpus load: `crates/test-harness` corpus smoke (312 v3 files through
  `load_project`) stays green — the backward-compat pin.
- `document-format-seam.spec.js`: written envelope has `version: 4`,
  `min_reader_version: 4`, a UUID `document.id` that is **identical** across
  two consecutive saves and across a reload; `sources` present; `created`
  still latched; too-new file still refused.
- JSON Schema golden: `docs/schema/waffle-v4.schema.json` is generated from
  the Rust types (`cargo test -p file-format --features json-schema
  schema_is_current`); the test fails when the committed schema is stale.
  Every fixture under `app/tests/gui/fixtures/*.waffle`, the root
  `*.waffle` samples, and the assay corpus validate against the schema
  (a v3 file validates against the v3-compatible subset: the test migrates
  first, then validates the v4 output).
- Rebuild parity: `err.waffle`, `minihexa.waffle`, `step_extrude.waffle`
  (the three root samples with region/import payloads) produce the same body
  count, face count and volume before and after migration.

---

## 6. Failure modes

| Condition | Behavior |
|---|---|
| Embed inflates beyond the cap (Phase 1 cap: 256 MiB decoded) | `EmbedTooLarge` — feature error, not file rejection. Closes the deflate-bomb gap noted in v3 §14.8 for shared files. |
| Unknown `embed.encoding` | loud per-source error (v3 contract) |
| `content_hash` unknown algorithm prefix | treated as absent |
| `content_hash` mismatch | warn, ignore embed, fall through (§3) |
| Git ref name with `..`, leading `-`, control chars; path with `..` segments or leading `/` | `InvalidLocator` at load; entry preserved, unresolvable |
| Remote not HTTPS | `InvalidLocator` (SSH remotes are recorded as `generic` + unresolvable in the browser host; a desktop host may resolve them) |
| Source cycle (A links B links A) | detected at resolution time by the host (visited set per document id + tab); the second visit is `SourceCycle`; never loops |
| `Local` locator leaving the browser | refused at save (§3) |
| Two sources with the same `id` | `ParseError` |
| `active_tab` naming an unknown-kind tab | loads; UI opens the first known tab and flags |

---

## 7. Storage, sharing, and personal access (design)

This section is design guidance for the app's storage layer; it is not part
of the file bytes, but the locator design above is shaped by it.

### 7.1 Where files live

| Tier | Today | v4 direction |
|---|---|---|
| Browser-local | IndexedDB `documents` store, 8-char provider ids | Stays the default for personal work. Adopt `document.id` as the record key (provider id becomes an alias during a one-time migration). Add a **content cache** store keyed by `content_hash` for fetched sources. Not linkable from elsewhere. |
| Personal git repo | GitHub provider: one repo, `<slug>.waffle` at root + `.waffle-index.json` | Generalize to a `GitProvider` with host adapters (§7.2). Allow **folders**; the index maps `document.id` → path. A document saved here has a location `{remote, path, ref: Branch(default)}` — which is exactly what makes it linkable and what `Relative` links resolve against. |
| Someone else's repo / a public repo | share link `?src=<raw url>` (currently emitted but **unhandled** by the app — dead) | "Open from link" accepts a `Git` or `Url` locator, creates a **linked** local document that is read-only until "fork to my storage" (copy, new `document.id`, all `Relative` links rewritten to absolute `Git` locators at the pinned commit). |
| Offline / attachment | — | "Pack" = set `pack: true` on every source, embedding content; the resulting single file opens anywhere with no network. |
| Desktop (future host) | — | `Relative` and `Git` locators resolve against the filesystem and local git; the same file is valid. |

### 7.2 Host adapters (browser host)

All adapters implement: `resolve_ref(remote, ref) → commit`,
`fetch_blob(remote, commit, path) → (bytes, blob_sha)`, and, for write-back,
`put_file(remote, branch, path, bytes, parent_blob_sha)`.

| Host | resolve_ref | fetch_blob | blob sha source |
|---|---|---|---|
| GitHub | `GET /repos/{o}/{r}/commits/{ref}` → `sha` | `GET /repos/{o}/{r}/contents/{path}?ref={commit}` (or raw.githubusercontent.com for public) | contents API `sha` |
| GitLab | `GET /projects/{id}/repository/branches/{name}` → `commit.id` (tags: `/repository/tags/{name}`) | `GET /projects/{id}/repository/files/{path}/raw?ref={commit}` | files API `blob_id` |
| Gitea / Forgejo | `GET /repos/{o}/{r}/branches/{name}` → `commit.id` | `GET /repos/{o}/{r}/raw/{path}?ref={commit}` | contents API `sha` |
| generic | none (ref stays unresolved; `Url`-like fetch of `remote` + path is not attempted) | — | computed locally |

Tokens are **per host** (`hosts: [{host_url, token}]`), replacing today's
single GitHub token. Public repos need no token. The existing device-flow
proxy pattern extends to GitLab/Gitea OAuth or PAT entry.

### 7.3 Write-back and permissions

- A linked source is **editable in place** only when its locator is
  `Git{ref: Branch}` in a repo where the user's token has write access; edits
  commit to that branch, and the linking document's `resolved.commit` is
  updated on the next sync.
- `Commit`- and `Tag`-pinned sources are read-only by construction.
- "Update to tip" and "Pin" are explicit actions with a visible diff of
  `resolved.commit` (old → new).

### 7.4 What a share link is

A share link is a locator, not a copy: `https://<app>/open?remote=<https clone url>&path=<path>&ref=<branch|tag|commit>`. For a public repo this opens
without login. For a private repo the app asks for a token for that host.
Opening always creates a linked, read-only local document (§7.1), so a
recipient cannot accidentally overwrite the sender's file, and "fork" is the
only path to editing.

---

## 8. Research basis

No single published technique; the design composes established practice:

- **Content-addressed identity + git blob ids**: git's object model (SHA-1 of
  `"blob <len>\0"` + bytes) — used so hashes computed offline match what
  every git host reports, avoiding a second hash scheme.
- **Pinned vs floating references**: Cargo/Go module semantics (lockfile
  commit vs branch tip); "resolved" recorded next to the request, as in
  `Cargo.lock`'s `source = "git+…#<commit>"`.
- **Recipe-not-geometry + opaque preservation**: Onshape's document/element
  model (documents contain elements of several kinds; unknown element kinds
  are not fatal); the v3 spec's own §13 forward-compat analysis.
- **Assembly linking**: FreeCAD `App::Link` (absolute vs relative, pinned
  external documents) and SolidWorks pack-and-go — the `pack` flag and
  `Relative` locator follow those precedents.
- **Mate connectors and instance paths**: `projects/10-assemblies/INTERFACES.md`
  (this repo) and Onshape's mate-connector model (`instance_path` mirrors
  Onshape's occurrence path).
- **Persistent naming across scopes**: `docs/PERSISTENT-NAMING.md`; the scope
  field composes with the existing anchor/selector/policy, it does not replace
  them.
- **KiCad**: the `.kicad_pcb` S-expression format (footprint `uuid`,
  `Reference` property, `at`, `layer`, `model` with `offset`/`scale`/`rotate`,
  `Edge.Cuts` outline, stackup thickness) and `kicad-cli pcb export step`.

### 8a. Analytical vs approximate

Not applicable: no surface-surface intersection is introduced. Derived 2D
drawing views (§9) will need kernel projection, silhouette and planar-section
capability; that spec must carry its own §7a.

---

## 9. Phases

| Phase | Content | Wire impact |
|---|---|---|
| **1 (this spec)** | `document.id`; `sources` table + git-aware locators; unknown tab/source kinds preserved; unknown keys preserved (§2.6); provenance table; ImportedBody → sources with dedup; `profile_entity_ids`; `solve_status` default; single Rust writer via bridge `SaveDocument`; engine source store + `ProvideSource`; inflation cap; JSON Schema golden; `docs/FILE_FORMAT.md` v4 section | v4, `MIN_READER_VERSION` 4 |
| 1b | Opaque preservation of unknown `Operation` variants (feature kept, rebuild error, re-emitted) so future ops stop bumping the reader floor | none |
| 2 | App storage: `document.id` as storage key; `GitProvider` with GitHub/GitLab/Gitea adapters; per-host tokens; content cache; open-from-link (fixes the dead `?src=`); pack/unpack; pin/update UI | none (uses Phase-1 fields) |
| 3 | `Assembly` tab kind: instances `{id, name, source: {source_id?, tab_id}, transform: {translation_m, rotation_quat}, external_key?, parameter_overrides?}`, mate connectors `{id, name, geom_ref(scoped), frame}`, mates (Fastened first), persisted solved placements as derived hints; `scope` on `GeomRef` lands here | new tab kind (no bump); `scope` field (bump for Part-side use) |
| 3b | KiCad: `KicadPcb` source kind; derived board sketch/extrude (`Derived` provenance); one instance per footprint keyed by footprint UUID; component models as `Step` sources resolved through a KiCad path-variable table; mounting holes → connectors | none beyond Phase 3 |
| 4 | `Drawing` tab kind: sheet, views `{source(scoped), projection kind, direction/up, placement, scale, style}`, annotations `{kind, refs(scoped), value, placement}`; kernel projection/HLR/section as a separate FIP spec | new tab kind (no bump) |

Phase-1 increments (each an atomic commit, tests first):

1. Spec (this file).
2. `file-format`: v4 types (`SourceEntry`, `Locator`, `GitRef`, `SourceKind`,
   `Resolved`, `TabKindWire`), envelope, extra maps, migration v3→v4, hash,
   loader v1–v4, `save_document_verified`; single-tree shims. RED→GREEN
   `format_tests`. Corpus still loads.
3. `feature-engine`: `ImportedBodyParams.source_id` + optional blob;
   `Engine.sources` store; rebuild resolution order; `provenance` table;
   `profile_entity_ids`; `SolveStatus::Unsolved` default.
4. `wasm-bridge`: `SaveDocument`, `ProvideSource`; `LoadProject` via
   `load_document` + source registration; `ImportStep` creates a source;
   `EngineState.sources`.
5. App: `buildDocumentJson` → `SaveDocument`; adopt `document.id`; home
   template v4; `format.js` = 4; always route document open through the
   engine loader; seam spec updated; WASM rebuilt in the same commit.
6. Schema: `json-schema` cargo feature (schemars) across `waffle-types`,
   `feature-engine`, `file-format`; golden `docs/schema/waffle-v4.schema.json`;
   fixture validation test.
7. Docs: `docs/FILE_FORMAT.md` v4 section; `projects/09-file-format/PLAN.md`;
   `projects/10-assemblies/PLAN.md` M6 pointer.
