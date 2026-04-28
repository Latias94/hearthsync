# WoW Addon Sync Core Review - 2026-04-21 Config Sync Follow-up

## Summary

This review re-checks the current HearthSync code after the task-progress and download-byte-progress
slice, using the NewBeeBox research as a product reference and the current repository state as the
source of truth.

Judgment: do not create a new workstream.
The remaining gaps still belong to `wow-addon-sync-core` because they are corrections to the shared
core contract for addon acquisition, configuration migration, and frontend-facing task execution.

## Inputs

- `targets/newbeebox_new/research/02-addon-wtf-flow-notes.md`
- `targets/newbeebox_new/research/03-hearthsync-current-code-review.md`
- `targets/newbeebox_new/research/04-hearthsync-implementation-roadmap.md`
- `src/core/app/stable.rs`
- `src/core/app/config.rs`
- `src/core/lua_patch/text.rs`
- `src/core/lua_patch/bytes.rs`
- `src/core/addon/provider/mod.rs`
- `src/core/addon/lock.rs`
- `src/core/addon/policy/mod.rs`
- `src/core/addon/execution.rs`
- `src/core/addon/index/operations.rs`
- `src/cli/args.rs`
- `src/cli/args/config.rs`
- `src/cli/addon_policy.rs`

## Findings Already Closed Since Older Gap Notes

The older NewBeeBox comparison notes contained several findings that are no longer current:

- account-root WTF files such as `config-cache.wtf`, `bindings-cache.wtf`, and `AddOns.txt` are now
  modeled in the current bundle and external-package paths
- mutable-source freshness and task progress are no longer only design wishes; they already landed
  in the current provider and task layers
- byte-oriented transfer progress now exists on the stable task event contract through
  `TaskProgressCode::DownloadArchive`
- the text Lua rewrite path now uses unique placeholders plus regression coverage, so the previous
  placeholder-collision bug is no longer current
- config sync now has first-class `config inspect|plan|apply` product vocabulary in both CLI and
  stable app surfaces instead of only external-package phrasing
- addon policy now has its own persisted file and first-class app/CLI surface, and local cache
  reuse now has a sidecar-plus-rehash integrity floor; the remaining gaps in those areas are about
  behavioral coverage, not missing state models
- cache and addon-state runtime defaults now also have an app-owned persistent settings backend
  plus a first CLI mutation surface, so cross-invocation cache policy no longer depends only on
  one-shot global flags
- regular provider-backed addon update now also consumes `release_channel` and
  `allow_prerelease` for floating GitHub and CurseForge source resolution; the remaining addon
  policy gap is now mostly dependency execution plus indexed-update scope

The next refactor pass should therefore focus only on the gaps that still remain in the current
codebase.

## Current Findings

### F1 - The stable app task surface had been too wide, and the first shrink pass is now in place

The triplet pattern was real, but it is no longer the current stable-boundary shape.
`StableAppServices` and `ExtendedAppServices` now expose long-running operations through one
task-shaped return value: `TaskRun<T>`.

The CLI now stays as a thin convenience layer on top of that contract by rendering `run.result`
instead of forcing the stable or extended app boundary to keep separate direct,
`*_collecting_progress`, and `*_with_callbacks` variants for the same operation.

Callback streaming still exists below the stable boundary on service/task-support helpers, which
keeps internal reuse and lower-level test coverage possible without re-expanding the public app
surface immediately.

Impact:

- the stable app boundary is already cheaper to evolve than before
- future `egui` work can consume one long-running app result shape directly
- the remaining task-execution cleanup is now internal: decide whether callback streaming should
  stay as lower-level service plumbing or later become one explicit public task-context abstraction

Recommended direction:

- keep `TaskRun<T>` as the stable long-running app result contract
- keep CLI rendering as a thin convenience wrapper over `run.result`
- if a future public streaming API is still needed, add one explicit task-context abstraction
  instead of reviving per-operation triplets

### F3 - Cached download reuse now validates local integrity, provider-side validators, and a first shared transport-level validator path

The provider no longer treats immutable cache entries as valid only because the archive file
exists. Current reuse now writes a local integrity sidecar and re-hashes the cached archive before
reuse under `src/core/addon/provider/mod.rs`.

Resolved GitHub release assets and resolved CurseForge files now also contribute remote validator
metadata such as published digest/hash, size, and modified time into cache reuse decisions.

`AddonLockPackage` still has room for source integrity metadata through `source_sha256`, but the
transport-layer story is now only partially open rather than absent. Generic `http(s)` archives no
longer have to refresh unconditionally when the remote server exposes reusable transport
validators: the provider HTTP layer now has one shared conditional GET path for `ETag` /
`Last-Modified`, and cached arbitrary-URL archives can now be reused through
`If-None-Match` / `If-Modified-Since` when those validators still match the local integrity
sidecar:

- `src/core/addon/lock.rs:37`
- `src/core/addon/lock.rs:47`
- `src/core/addon/lock.rs:53`
- `src/core/addon/lock.rs:54`

Impact:

- corrupted or locally modified cache files now trigger a refresh instead of silent reuse
- GitHub/CurseForge immutable cache reuse now has a stronger "matches remote intent" contract than
  the previous sidecar-only model
- arbitrary `http(s)` archives can now reuse cache when transport validators remain stable
- arbitrary `http(s)` archives without reusable transport validators now also have an explicit
  short-lived freshness contract instead of unconditional refresh
- explicit `addon cache repair` is no longer purely local cleanup; it can now remote-verify
  validator-backed cache entries and refresh or prune them when the current contract says they are
  stale
- provider cache policy is now also operator-facing: runtime/CLI can configure cache root and the
  no-validator HTTP freshness behavior directly, and the stable runtime diagnostics surface reports
  the configured policy rather than hiding it inside provider internals
- cache policy persistence now also has one explicit app-owned backend: `core::app` stores
  selected runtime overrides under app-data `settings/runtime.toml`, the CLI exposes
  `settings inspect|set|reset`, and runtime assembly merges persisted settings before one-shot CLI
  overrides
- future GUI cache management would still only have a partial validity state to display

Recommended direction:

- keep the local sidecar-plus-rehash floor
- keep using provider metadata validators where the source API exposes them
- keep the shared transport-level validator path for `ETag` / `Last-Modified` style archive
  probing on arbitrary remote URLs
- keep the explicit no-validator freshness policy bounded and fail closed when older sidecars do
  not have the fetch-time field needed to evaluate that policy
- keep the new operator-facing `addon cache purge|repair` surface as the first explicit cache-state
  maintenance contract instead of hiding corruption cleanup behind implicit re-download behavior
- keep remote repair best-effort and explicit rather than turning every cache-touch path into a
  background network validation pass
- keep the explicit app-owned runtime settings backend as the only persistence path for cache and
  addon-state defaults, and let future GUI work reuse that backend instead of inventing a second
  settings store
- keep the new conditional GET path as the canonical transport-validator flow instead of reverting
  to a separate pre-download probe path
- treat source-archive integrity as operator-facing state instead of an internal assumption

### F5 - Addon reproducibility and mutable policy are now split, and source-resolution coverage is broader, but execution is still partial

`AddonLockPackage` is strong as a reproducible artifact:

- `src/core/addon/lock.rs:37`
- `src/core/addon/lock.rs:47`
- `src/core/addon/lock.rs:53`
- `src/core/addon/lock.rs:54`
- `src/core/addon/lock.rs:57`
- `src/core/addon/lock.rs:58`

That gap now has a concrete persistence and product seam:

- mutable addon preferences live in `src/core/addon/policy/mod.rs`
- storage now resolves through the runtime-owned addon state layout, defaulting to platform
  app-data keyed by installation identity while sidecar `.hearthsync` remains an explicit portable
  backend
- stable app callers now use first-class addon-policy request/result shapes
- CLI callers now use `addon policy inspect`, `addon policy set`, and `addon policy remove`

Current execution now consumes a bounded subset of that policy:

- bulk `addon update` and bulk `addon index update` skip tracked packages marked with
  `ignored = true`
- explicit named addon update overrides `ignored` and still runs
- provider-backed addon update applies basic pins (`pin.file_id` for CurseForge, `pin.version` as
  a GitHub tag override) while preserving the tracked `package_id`
- regular provider-backed addon update now also forwards `release_channel` and
  `allow_prerelease` into provider resolution for floating sources

Current provider semantics are now explicit:

- floating GitHub releases treat `allow_prerelease = true` or `release_channel = beta|alpha` as
  prerelease-eligible selection, while explicit GitHub `tag` pins remain authoritative
- floating CurseForge mods now map `stable|beta|alpha` onto release-type filtering, while explicit
  `file_id` pins remain authoritative
- addon-index update intentionally keeps curated index source declarations authoritative and today
  now consumes `ignored` plus the first explicit dependency-installation slice, but not broader
  source-resolution override preferences

The remaining gap is narrower still:

- regular addon update now installs missing required CurseForge dependencies when
  `install_dependencies = true`, while unsupported source kinds fail explicitly instead of acting
  like a silent no-op
- addon-index update now does the same for missing required dependencies, but it resolves them from
  the curated index source and still does not consume user-controlled pin or
  release-channel/prerelease overrides
- provider-side dependency resolution now also returns an explicit strategy value instead of a bare
  dependency-source list, so the current `missing required only` semantics are represented in the
  contract rather than only in comments and call-site expectations
- provider-side dependency support now also has an explicit capability contract, so unsupported
  sources are rejected from a declared capability boundary instead of only through deeper
  provider-resolution error paths
- that dependency capability now also projects through app-owned source values, so frontend callers
  can inspect source-level dependency support from addon/index/lock results before invoking update
  flows
- addon and addon-index app services now also preflight `install_dependencies = true` against
  provider dependency capability before they enter domain update execution, so unsupported sources
  fail earlier than the previous prepare-stage-only validation path
- addon-index matching now also uses provider-level source-family identity in both preflight and
  domain update flows, so index-package id drift and GitHub asset-name drift no longer force a
  fallback when the tracked source still identifies the same underlying package family
- addon-index matching now also accepts unique exact display-name continuity as a later fallback,
  so source-family migration can still preflight when the curated package name remains stable
  across tracked package id, stored metadata package name, addon directory name, or addon title
- addon-index schema now also supports exact author-declared `match_package_ids` hints, so curated
  source-family migrations can explicitly bridge known historical tracked package ids without
  adding new fuzzy runtime matching rules
- addon-index preflight matching remains intentionally conservative only when the curated source
  family itself changes, the index also omits exact `match_package_ids`, stable addon-directory
  hints, and exact unique display-name continuity, so unsupported dependency-install policy can
  still fall back to the existing domain validation path in that narrower no-explicit-bridge case
- that narrower fallback no longer fails as an opaque unsupported-source error, though: the domain
  path now explains that app preflight could not determine the tracked-package mapping from stable
  identity hints alone and points curators toward exact `match_package_ids`, stable
  `addon_directories`, or unique exact package-name continuity as the bridge options
- `addon index inspect` now also exposes exact-identity-hint coverage directly, so operators do
  not need to infer curation risk only from raw package TOML fields when reviewing an index
- that inspect result now also carries structured warning objects for packages missing any exact
  identity hint, so future GUI or automation work can bind on a stable warning code instead of
  re-parsing CLI-oriented summary text
- `addon index validate` now also promotes those warnings into an explicit validation surface, so
  curation workflows can use a non-zero CLI exit or a structured validation result instead of
  treating inspect output as an implicit policy gate
- dependency execution is still intentionally narrow: no dependency upgrades yet, no broader
  cross-provider dependency model yet

Impact:

- the lock is now strong for replay while policy has its own persisted home
- future GUI addon UX no longer needs to invent a parallel addon-preference file format
- the remaining product gap is no longer basic release selection; it is the narrower question of
  dependency execution semantics plus whether indexed update should ever consume more mutable user
  source-resolution policy than `ignored`

Recommended direction:

- keep `lock.toml` reproducible and content-oriented
- keep evolving the separate addon policy/profile layer for mutable user choices
- keep release-channel and prerelease-aware behavior on the regular addon-update/provider path
  where floating source selection actually happens
- keep the first dependency-installation slice narrow and explicit: missing required CurseForge
  dependencies during regular addon update and addon-index update, with indexed update resolving
  from curated source authority and unsupported source kinds surfaced as validation errors instead
  of silently ignored policy
- decide separately whether dependency handling should later expand into dependency upgrades,
  additional providers, or indexed-update policy
- decide separately whether indexed update should ever expose user-controlled source-resolution
  preferences beyond `ignored` plus dependency-install enablement

### F6 - Encoding strategy is still narrower than the reference product

The byte rewrite path currently supports UTF-8 and Latin-1 replacement encodings:

- `src/core/lua_patch/bytes.rs:23`
- `src/core/lua_patch/bytes.rs:48`

That is safer than UTF-8-only rewriting, but it is still narrower than the reference-product
signals around explicit encoding-aware Lua migration.

The current code now also has a first sanitized real-world fixture floor instead of only tiny
inline snippets:

- a more realistic UTF-8 `Details.lua` sample with Chinese text
- a more realistic Latin-1 `Pawn.lua` sample with extended characters
- a more realistic UTF-8 `Clique.lua` sample with Chinese character/server profile keys and
  `spec*_profileKey` fields
- a more realistic UTF-8 `BagSync.lua` sample with realm-and-character keyed account data
- a more realistic UTF-8 `AddOnSkins.lua` sample with `profiles` plus `profileKeys` keyed by
  `角色 - 服务器`
- a more realistic UTF-8 `ElvUI.lua` sample with mixed `角色 - 服务器` profile keys,
  nested realm/character maps, and no-space `角色-服务器` combined identity keys
- a more realistic UTF-8 `NewBeeBox.lua` sample with no-space `服务器-角色` combined identity keys,
  plus separate `name` / `realmName` fields without rewriting `Player-...` GUID values

Impact:

- localized rewrite confidence is better than before and no longer entirely snippet-driven
- the current rewrite model is now more conservative for unknown files: generic `playerName` /
  `realm` markers alone no longer enable identity rewrite, so multi-character account files such as
  `Syndicator.lua` fail closed unless they have an explicit known-file rule
- chat-history and cache payloads such as `WIM.lua` are also explicitly outside the current
  automatic rewrite surface; nested `服务器 -> 角色 -> 会话历史` data stays fail-closed by default
- `Rarity.lua` is also narrower now: `profileKeys` still rewrite, but account-wide statistics no
  longer use the identity whitelist because real payloads contain many same-server characters and
  broad `playerName` / `server` replacement would mis-target unrelated rows
- fixture breadth now also covers a more realistic UTF-8 `RurutiaSuite.lua` sample plus a more
  realistic UTF-8 `NDui_Bags.lua` sample with a long single-line `profileKeys` payload, so the
  generic profile-key path is now checked against author text, mixed simplified/traditional
  character variants, legacy suffixed profile keys, and dense one-line account maps without adding
  a broader identity rule
- the UTF-8 `profileKeys` path is now narrower too: profile-style rewrites are scoped to direct
  `profileKeys` entries, direct `profiles` keys, and `*profileKey` field values instead of whole-
  document exact-string replacement, and `Clique.lua` now also rides an explicit known-file
  identity rule because its real payload keeps `char` tables keyed by `角色 - 服务器`
- the UTF-8 known-file identity path is now narrower too: explicit identity field values such as
  `playerName`, `realm`, `server`, `character`, `LastPlayerFullName`, `LastRealm`, `guildrealm`,
  `realmKey`, `rwsKey`, and paired `name + realmName` no longer rely on whole-document quoted-
  string replacement, so real `Details.lua`-style `lastPlayerName` text now stays untouched while
  real `NewBeeBox.lua` player records still rewrite correctly
- the UTF-8 identity-key path is narrower again: exact identity-shaped Lua keys now rewrite only in
  known containers such as root table-key records, `profileKeys`, `profiles`, `char`, `faction`,
  `worldBoss`, `searchHistoryList`, `Toons`, `value`, `currentrealm`, and `totals`, plus
  root/faction `服务器 -> 角色` maps, so arbitrary nested cache keys that merely equal
  `角色 - 服务器`, `角色-服务器`, or `服务器-角色` now stay untouched
- the previous architectural byte-fallback risk is now narrower: the non-UTF-8 path can also use
  Lua-structure-scoped rewrites for profile keys, explicit identity fields, paired `name` /
  `realmName`, known identity-key containers, and realm-character maps, and valid UTF-8 payloads
  no longer fall through into raw-byte rewriting after a scoped text miss
- new addon-specific identity-key containers still require explicit evidence before they should
  join the shared allowlist
- fixture breadth is still too narrow to claim broad Chinese-region or addon-wide migration safety

Recommended direction:

- keep collecting sanitized real-world SavedVariables samples across more addons and payload shapes
- keep raw-byte preservation as the default when rewrite confidence is weak
- decide whether a broader explicit encoding policy is needed before desktop rollout

## Recommended Next Slice Order

1. decide whether dependency handling should expand beyond missing required CurseForge installs into
   dependency upgrades, additional providers, or indexed-update policy
2. keep expanding sanitized SavedVariables fixture breadth before broad config-migration claims
3. decide whether the new transport-level conditional GET path, explicit no-validator freshness
   policy, and first remote validator-driven `addon cache repair` slice should grow into a richer
   operator-facing cache policy surface
4. decide whether indexed update should ever consume mutable source-resolution preferences beyond
   `ignored` without weakening curated index authority
5. only then decide whether lower-level callback task plumbing should become one explicit public
   task-context surface instead of remaining below the stable app boundary

## Workstream Decision

Do not add a third workstream for configuration sync or NewBeeBox parity.
The remaining work is still one `wow-addon-sync-core` refactor sequence:

- core task/app contract cleanup
- configuration-sync ergonomics
- addon policy modeling
- cache integrity hardening
- real-world rewrite safety
