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
- `src/core/lua_patch/text.rs`
- `src/core/lua_patch/bytes.rs`
- `src/core/addon/provider/mod.rs`
- `src/core/addon/lock.rs`
- `src/cli/args.rs`
- `src/cli/args/external_package.rs`
- `src/cli/external_package/request.rs`

## Findings Already Closed Since Older Gap Notes

The older NewBeeBox comparison notes contained several findings that are no longer current:

- account-root WTF files such as `config-cache.wtf`, `bindings-cache.wtf`, and `AddOns.txt` are now
  modeled in the current bundle and external-package paths
- mutable-source freshness and task progress are no longer only design wishes; they already landed
  in the current provider and task layers
- byte-oriented transfer progress now exists on the stable task event contract through
  `TaskProgressCode::DownloadArchive`

The next refactor pass should therefore focus only on the gaps that still remain in the current
codebase.

## Current Findings

### F1 - The stable app task surface is still too wide

`src/core/app/stable.rs` still grows by triplets for every long-running operation:

- direct call
- `*_collecting_progress`
- `*_with_callbacks`

Representative lines:

- `src/core/app/stable.rs:87`
- `src/core/app/stable.rs:94`
- `src/core/app/stable.rs:115`
- `src/core/app/stable.rs:122`
- `src/core/app/stable.rs:185`
- `src/core/app/stable.rs:234`
- `src/core/app/stable.rs:262`
- `src/core/app/stable.rs:298`
- `src/core/app/stable.rs:327`

Impact:

- each new capability multiplies stable API surface area
- future `egui` work will likely use task-aware execution almost everywhere anyway
- the stable boundary is still more expensive to evolve than it should be

Recommended direction:

- keep `TaskRun<T>` and callback streaming
- introduce one task-context style invocation path underneath the stable app boundary
- let direct CLI calls remain thin convenience wrappers instead of the primary design axis

### F2 - Text Lua rewrite still has a placeholder-collision bug

`src/core/lua_patch/text.rs:76` rewrites text through staged placeholders, but the placeholder
generation at `src/core/lua_patch/text.rs:91` does not check whether the original content already
contains the same placeholder literal.

The byte path already does this correctly:

- `src/core/lua_patch/bytes.rs:83`
- `src/core/lua_patch/bytes.rs:95`

Impact:

- if a real SavedVariables file already contains `__HEARTHSYNC_REWRITE_<n>__`, the text path can
  rewrite unrelated content
- the text and byte rewrite paths no longer share the same safety floor

Recommended direction:

- give the text path the same unique-placeholder loop as the byte path
- add regression coverage for pre-existing placeholder literals in user content

### F3 - Cached download reuse still trusts file presence instead of integrity

The provider now distinguishes mutable from immutable remote references correctly, but cache reuse
for immutable artifacts is still only presence-based:

- `src/core/addon/provider/mod.rs:534`
- `src/core/addon/provider/mod.rs:557`
- `src/core/addon/provider/mod.rs:562`
- `src/core/addon/provider/mod.rs:564`

`AddonLockPackage` already has room for source integrity metadata through `source_sha256`, but that
field is not currently used to validate an existing cached archive before reuse:

- `src/core/addon/lock.rs:37`
- `src/core/addon/lock.rs:47`
- `src/core/addon/lock.rs:53`
- `src/core/addon/lock.rs:54`

Impact:

- a corrupted local cache file can still be reused indefinitely
- immutable-source caching is freshness-aware, but not integrity-aware
- future GUI cache management would have no trustworthy validity state to display

Recommended direction:

- add cache sidecar metadata and validation for size, hash, or provider validators when available
- expose a cache purge/repair operation
- treat source-archive integrity as operator-facing state instead of an internal assumption

### F4 - Config sync is still product-visible only through `external-package` and bundle wording

Top-level CLI commands currently expose `ExternalPackage`, not a first-class config namespace:

- `src/cli/args.rs:50`
- `src/cli/args/external_package.rs:8`

The current external-package wrapper is semantically correct, but the user-facing request-building
still starts from `CreateExternalPackageBundleAppRequest` and author-package bundle defaults:

- `src/cli/external_package.rs:22`
- `src/cli/external_package.rs:27`
- `src/cli/external_package.rs:46`
- `src/cli/external_package/request.rs:17`
- `src/cli/external_package/request.rs:21`
- `src/cli/external_package/request.rs:57`

Impact:

- the core engine already supports configuration migration semantics, but CLI ergonomics still
  frame that work mostly as package import
- future GUI information architecture would have to invent a clearer config surface later instead
  of reusing one explicit app/CLI concept now
- users who think in terms of "sync settings" rather than "import bundle-like package" do not yet
  get a first-class entrypoint

Recommended direction:

- add a `config` namespace as a thin wrapper over the same external-package and bundle planning
  engine
- keep one core planner and one apply model; only improve product vocabulary and request shaping

### F5 - Addon reproducibility exists, but policy and preference state is still missing

`AddonLockPackage` is strong as a reproducible artifact:

- `src/core/addon/lock.rs:37`
- `src/core/addon/lock.rs:47`
- `src/core/addon/lock.rs:53`
- `src/core/addon/lock.rs:54`
- `src/core/addon/lock.rs:57`
- `src/core/addon/lock.rs:58`

But the current addon model still has no separate persisted layer for operator choices such as:

- ignore this addon
- pin a specific remote file or release channel
- allow or forbid pre-release updates
- declare dependency-install policy

Impact:

- the lock is strong for replay and verification, but weak as an update-policy store
- future GUI addon UX would either need to overload the lock or invent parallel state ad hoc
- NewBeeBox-style practical update preferences still have no home in the current product model

Recommended direction:

- keep `lock.toml` reproducible and content-oriented
- add a separate addon policy/profile layer for mutable user choices

### F6 - Encoding strategy is still narrower than the reference product

The byte rewrite path currently supports UTF-8 and Latin-1 replacement encodings:

- `src/core/lua_patch/bytes.rs:23`
- `src/core/lua_patch/bytes.rs:48`

That is safer than UTF-8-only rewriting, but it is still narrower than the reference-product
signals around explicit encoding-aware Lua migration.

Impact:

- the current rewrite model may still be too optimistic for some real-world SavedVariables payloads
- Chinese-region migration confidence is not yet backed by fixture coverage

Recommended direction:

- collect sanitized real-world SavedVariables samples
- keep raw-byte preservation as the default when rewrite confidence is weak
- decide whether a broader explicit encoding policy is needed before desktop rollout

## Recommended Next Slice Order

1. fix the text-path placeholder collision in `lua_patch`
2. add a thin first-class `config` product surface over the existing planning engine
3. split addon policy/preferences from the reproducible lock model
4. make cached artifact reuse integrity-aware instead of file-presence-aware
5. add sanitized real-world SavedVariables coverage before claiming robust config migration
6. only then shrink the stable app task surface into a smaller invocation model

## Workstream Decision

Do not add a third workstream for configuration sync or NewBeeBox parity.
The remaining work is still one `wow-addon-sync-core` refactor sequence:

- core task/app contract cleanup
- configuration-sync ergonomics
- addon policy modeling
- cache integrity hardening
- real-world rewrite safety
