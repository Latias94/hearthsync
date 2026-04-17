# WoW Addon Sync Core Design

## Status

Active on 2026-04-16.

This workstream is now the architecture source of truth for reusable core design.
The CLI workstream remains active, but it should focus on command surface, user-facing output,
and delivery sequencing on top of this core.

## Problem Statement

`hearthsync` is no longer only a command-line experiment.
It is becoming a cross-platform synchronization product that must support:

- direct CLI use
- a future `egui` desktop frontend
- addon package download and update
- portable UI bundle export and apply
- third-party UI package import and normalization
- safe migration of `WTF`, fonts, and interface assets

The current codebase already contains substantial core logic, but the main binary still owns the
crate root and long-running operations do not yet expose reusable task boundaries.
If more features are added without changing those boundaries, future desktop work will inherit
CLI-shaped assumptions instead of consuming a real application core.

## Goals

- Define a reusable Rust core that can be consumed by CLI and future desktop code.
- Keep product semantics centered on WoW installation structure instead of raw file copying.
- Introduce clear boundaries for tasks, progress, cancellation, provider access, and transaction safety.
- Normalize first-party bundles and third-party UI packages through one core planning model.
- Preserve the current safety posture while making future refactors cheaper.

## Non-Goals

- Immediate migration to a multi-crate workspace.
- Full async conversion in the first refactor slice.
- Desktop UI implementation in this workstream.
- Deep compatibility with every third-party launcher format on day one.

## Current Architecture Snapshot

The repository already has useful core subdomains:

- `install`: local WoW installation discovery and inspection
- `backup`: backup, list, and restore workflows
- `bundle`: pack, inspect, plan, unpack, and addon-lock sidecar flows
- `addon`: provider-backed install, update, remove, lock, and index flows
- `lua_patch`: targeted rewrite rules for selected `SavedVariables`
- `manifest`: first-party bundle manifest schema

Strengths in the current code:

- explicit resource-group planning for bundle apply
- single backup and rollback boundary for mutating bundle and addon-lock operations
- derived addon lock with content hashing
- tests covering current sync semantics

Current architectural constraints:

- the package only exposes a binary-shaped entrypoint by default
- long-running operations do not yet share a stable task contract
- provider networking is still direct and blocking inside core
- backup restore is useful but not yet modeled as a transaction-safe restore pipeline
- bundle planning now has an internal logical-planning and execution-preparation split, but public plan generation still reads source bytes and previews rewrites to keep today’s preview semantics
- public bundle apply operations have started shedding execution-only detail; rewrite-related fields such as per-entry `rewrite_applied`, per-entry `rewrite_count`, and summary-level `files_to_rewrite` no longer leak out of internal execution preparation
- direct external-package plan and apply now use a native normalized-source path, but bundle planning still keeps too much execution-shaped preparation internally
- `core::app` facades are still mostly thin forwarders and do not yet own provider or runtime policy injection
- account and character discovery still rely mostly on directory layout instead of richer role artifacts

## Target Architecture

The target product shape is a layered application core:

```text
CLI / Desktop
  -> app services / tasks
    -> domain modules
      -> provider ports + filesystem / archive adapters
```

Short-term repository layout:

```text
src/
  lib.rs
  main.rs
  cli/
  core/
```

Longer-term logical layout:

```text
core/
  app/
    task/
    services/
  domain/
    install/
    addon/
    bundle/
    backup/
    manifest/
    lua_patch/
  infra/
    provider/
    archive/
    fs/
```

The code does not need to move to separate crates immediately.
The first requirement is to make those boundaries real in the library API and in the request/result model.

## Core Boundaries

### Library Boundary

The package should expose reusable core modules through `lib.rs`.
The binary should become a thin shell that only parses CLI arguments and renders output.

### Application Services

The reusable entrypoints should move toward service-shaped operations instead of direct command handlers.

Examples:

- `inspect_installation`
- `plan_bundle_apply`
- `execute_bundle_apply`
- `plan_addon_lock_sync`
- `execute_addon_lock_sync`
- `analyze_external_ui_package`

The next step is to make these services real orchestration boundaries:

- caller-visible request and result types should stop leaking transport-specific defaults
- provider/cache/retry policy should be configured at the service boundary
- CLI and future desktop code should not need to know whether an operation came from a zip archive, a normalized external package, or a provider download

### Domain Modules

Domain logic should remain responsible for WoW semantics:

- installation flavors
- resource groups
- account and character identity
- addon package identity and lock semantics
- bundle manifest and apply policy semantics

### Infrastructure Ports

The following dependencies should sit behind explicit ports before any async conversion:

- addon providers
- archive readers and writers
- local filesystem mutation helpers
- optional external helper integration

## Task Model

The future desktop frontend needs a stable task contract even if the initial implementation remains synchronous.

Each long-running operation should converge on:

- task input
- progress events
- warnings
- cancellation checks
- final structured result

Initial task candidates:

- addon install
- addon update
- addon lock apply
- bundle pack
- bundle apply
- third-party UI package analysis and import plan generation
- backup restore

The CLI may execute tasks synchronously and print progress directly.
The future desktop frontend may run the same tasks on worker threads.

## Provider Abstraction

Current provider code is already modeled around source references, which is a good base.
The next refactor step is to move acquisition behind traits and request structs.

Required capabilities:

- resolve addon source metadata
- download or materialize an archive
- search supported catalogs
- report provider-specific warnings and attribution

The first implementation may still use blocking `reqwest`, but call sites should depend on a port rather than a concrete HTTP client.

## Third-Party Bundle Import Model

The product goal is broader than first-party `manifest.toml` bundles.
Users also need to import author-distributed setup packages that may contain:

- `Interface/AddOns`
- `WTF`
- `Fonts`
- interface assets such as materials or textures

The core should therefore add an analyzer layer:

1. inspect external archive or folder layout
2. classify discovered resources into product resource groups
3. produce a normalized import model
4. generate a temporary internal manifest or apply plan
5. reuse the same planner and executor used by first-party bundles

This keeps the system honest:

- one planning model
- one execution model
- one safety model

The current temporary first-party bundle bridge remains useful for explicit export workflows,
but it should not stay on the critical path for direct `external-package plan` or
`external-package apply`.
Those direct flows should operate on normalized source entries without first repacking them
into another archive.

## Safety and Transaction Model

Safety remains a core product feature, not a CLI convenience.

Mutating operations should converge on:

1. inspect and validate target
2. build explicit plan
3. create backup checkpoint
4. stage or materialize inputs
5. execute deterministic operations
6. verify or summarize result
7. rollback from one clear boundary on failure

`backup restore` should eventually follow the same philosophy instead of only clearing destination groups and replaying files.

## Cross-Platform Strategy

Cross-platform support should remain explicit.

Primary supported targets:

- Windows
- macOS

Key concerns:

- case sensitivity differences
- path normalization
- filename encoding in archives
- flavor compatibility
- account and character remapping
- author packages that were produced on a different platform than the target

## Current Refactor Slice

The bounded 2026-04-17 refactor slice removed the most expensive redundant path without changing
product semantics:

1. keep explicit bundle creation for users who want a reusable first-party archive
2. remove the mandatory temporary-bundle bridge from direct external-package plan and apply
3. introduce a reusable source-entry preparation boundary shared by bundle archives and normalized external packages
4. keep current CLI behavior and safety guarantees intact while deleting the now-redundant path

The next slice should now move bundle planning toward a purer logical plan that does not need
execution-time byte work during public plan generation.
This still avoids a full provider-runtime rewrite or async conversion.
Its purpose is to keep deleting execution-shaped planning work so later architecture cleanup starts
from a smaller, cleaner core.

## Open Questions

- Should the first reusable task API live under `core::app` or directly under `core`?
- When third-party packages contain both first-party metadata and raw folders, which source should win?
- Should backup restore gain verification before replacement, or a full staged swap pipeline?
- At what point is a workspace split justified instead of keeping one crate with stronger internal boundaries?
