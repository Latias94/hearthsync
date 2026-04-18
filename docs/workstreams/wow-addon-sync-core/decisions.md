# WoW Addon Sync Core Decisions

## ADR-001: Reusable Core Architecture Lives in the Core Workstream

### Status

Accepted on 2026-04-16

### Decision

Reusable architecture ownership moves from the CLI workstream to `wow-addon-sync-core`.

### Consequences

- CLI documentation should reference the core workstream for shared architecture decisions.
- Future frontend planning should build on this workstream instead of adding another parallel source of truth.

## ADR-002: The First Refactor Slice Prioritizes Boundaries over Behavior Changes

### Status

Accepted on 2026-04-16

### Decision

The first fearless refactor slice should expose a reusable library boundary and document target contracts before changing provider runtime or destructive operation semantics.

### Consequences

- lower conflict risk with ongoing feature work
- safer sequencing for later refactors

## ADR-003: CLI and Desktop Must Consume the Same Core Contracts

### Status

Accepted on 2026-04-16

### Decision

The CLI is a shell, not the product architecture owner.
Future desktop work must call the same service or task contracts as the CLI.

### Consequences

- output rendering stays in CLI-facing code
- domain logic and long-running operation orchestration move toward reusable services

## ADR-004: Provider Access Sits Behind Ports before Full Async Conversion

### Status

Accepted on 2026-04-16

### Decision

Provider-backed addon acquisition should move behind traits or ports before the project considers a full async runtime conversion.

### Consequences

- current blocking HTTP can remain temporarily
- later async migration becomes a contained infrastructure change instead of a domain rewrite

## ADR-005: Third-Party UI Packages Are Normalized into the Same Planning Model

### Status

Accepted on 2026-04-16

### Decision

Author-provided setup packages should be analyzed and normalized into the same internal resource-group plan used by first-party bundles.

### Consequences

- one plan model
- one execution model
- one safety model

## ADR-006: Restore Safety Is a Core Concern

### Status

Accepted on 2026-04-16

### Decision

Backup restore should eventually follow the same transaction-oriented safety philosophy as bundle apply and addon-lock apply.

### Consequences

- restore work belongs in core architecture planning, not only in CLI polish
- validation and staging improvements are not optional cleanup

## ADR-007: Backup Restore Uses Prevalidation plus Transactional Rollback

### Status

Accepted on 2026-04-17

### Decision

Backup restore should validate archive metadata and restore destinations before destructive replacement, then create an internal checkpoint of the current target state so a failed restore can roll back to the pre-restore filesystem state.

### Consequences

- restore no longer trusts archive layout only after clearing target groups
- future GUI recovery flows can surface one explicit restore task with deterministic failure semantics
- restore safety now aligns more closely with bundle apply and addon-lock apply rollback philosophy

## ADR-008: Direct External-Package Plan and Apply Must Not Require Temporary Bundle Repacking

### Status

Accepted on 2026-04-17

### Decision

The temporary first-party bundle bridge may remain available for explicit export workflows, but
direct `external-package plan` and `external-package apply` must not depend on repacking the
normalized source package into a temporary bundle archive first.

### Consequences

- normalized source entries need a first-class planning and execution path
- temporary bundle creation becomes optional instead of mandatory on the direct sync path
- redundant staging and archive I/O can be deleted from the hot path

## ADR-009: Planning Boundaries Should Be Logical, Not Execution-Shaped

### Status

Accepted on 2026-04-17

### Decision

Planning should converge on source classification, target comparison, and logical operation
selection. Execution-time byte materialization and file rewrite staging should not remain on the
critical path of public planning APIs unless no smaller boundary is practical.

### Consequences

- future plan APIs stay responsive enough for dry-run and GUI preview use
- execution preparation data should stay internal
- source-specific execution helpers may still exist, but they should not define the public plan boundary

## ADR-010: Real-World Import Compatibility Hardening Stays inside the Existing Core Workstream

### Status

Accepted on 2026-04-17

### Decision

Do not create a new parallel workstream for the current real-world addon and WTF compatibility
cleanup. Record it as a bounded refactor slice inside `wow-addon-sync-core`, with CLI notes
capturing only user-facing consequences.

### Consequences

- reusable architecture still has one source of truth
- the refactor can delete duplicated compatibility code instead of creating another planning track
- CLI documentation should describe delivery impact, not own the architecture decision

## ADR-011: Frontends Enter the Product through `core::app`, not Domain Install Helpers

### Status

Accepted on 2026-04-17

### Decision

The first stable top-level reusable API surface is `core::app::HearthSyncApp`.
CLI and future desktop code should obtain `InstallationService`, `BundleService`,
`ExternalPackageService`, `AddonService`, `AddonIndexService`, `AddonLockService`, and
`BackupService` from that shared app entrypoint instead of treating domain install helpers as
frontend-facing public API.

### Consequences

- one shared `AppRuntime` becomes the canonical place for host-platform, install-scan, provider,
  backup, and bundle-output policy
- `core::install::{scan_installations, inspect_installation, resolve_installation}` can move to
  crate-internal support status while `core::app::InstallationService` owns the frontend contract
- future `egui` work can start from one explicit application root instead of a loose set of
  unrelated façades

## ADR-012: Portable Inputs Must Not Depend on Caller Working Directory or Case-Sensitive Targets

### Status

Accepted on 2026-04-17

### Decision

Normalized external-package inputs and addon-index local archive references must remain portable
across the supported Windows and macOS product targets. Case-only path variants that would collide
on case-insensitive targets are invalid, and addon-index relative local archive paths must resolve
relative to the index file instead of the caller's process working directory.

### Consequences

- external-package normalization cannot rely on exact-string duplicate checks alone
- reusable addon index workflows cannot route local archive portability through ambient process state
- future GUI and CLI callers share one deterministic contract for portable author packages and
  portable curated addon catalogs

## ADR-013: Common WTF Targets Must Be Resolved Conservatively

### Status

Accepted on 2026-04-17

### Decision

Local account discovery and common-WTF target selection must prefer false negatives over silent
mis-targeting. Raw directory shape alone is not enough evidence that a directory is a real local
account or character, and the planner must not auto-select a common-WTF target account only because
one discovered account directory exists.

### Consequences

- local account discovery should require account-level or character-level artifacts such as
  `SavedVariables` or real character-root files instead of trusting any nested directory layout
- common-WTF application should require explicit account selection unless mappings already resolve a
  unique target account
- future GUI flows can surface “needs explicit account selection” as a deterministic planning state
  instead of silently writing common settings into the wrong account tree

## ADR-014: App Task Entry Points Must Reuse One Wrapper Contract

### Status

Accepted on 2026-04-18

### Decision

`core::app` services that expose direct, collected-progress, and callback-based task entrypoints
should reuse one shared wrapper helper instead of each façade manually reconstructing default
`NeverCancel`, noop progress sinks, or callback plumbing.

### Consequences

- direct app-service calls and progress-aware app-service calls now route through the same task path
  where a task contract already exists
- future GUI work can depend on one consistent cancellation and progress entry shape instead of
  service-specific wrapper behavior
- the next refactor slices should target real service-contract semantics rather than preserving
  duplicated façade plumbing

## ADR-015: App Read and Plan APIs Use Owned Request Contracts

### Status

Accepted on 2026-04-18

### Decision

`core::app` read-oriented and planning-oriented service APIs should accept explicit owned request
structs instead of exposing service-specific mixes of borrowed paths, optional references, and
positional parameters.

### Consequences

- CLI and future GUI code depend on one stable app-facing input shape per operation
- runtime policy injection and app-level defaults can evolve without leaking more positional
  arguments into frontend callers
- the app boundary moves closer to a real service contract and further away from a thin forwarding
  façade over domain helpers

## ADR-016: Read-Only App Outputs Use App-Owned DTOs

### Status

Accepted on 2026-04-18

### Decision

Read-only `core::app` outputs that frontends consume directly should prefer app-owned result DTOs
over returning raw domain aggregate structs.

### Consequences

- future GUI work can bind to stable app-facing output shapes without learning internal domain
  aggregate boundaries
- CLI rendering can depend on app-owned summary counts and labels instead of reconstructing them
  from nested domain payloads
- execution and mutation results may still transition in later slices, but the read-only surface now
  has a clearer ownership boundary

## ADR-017: App Mutation APIs Use Owned Request Contracts

### Status

Accepted on 2026-04-18

### Decision

`core::app` mutation-oriented service APIs should accept app-owned request structs instead of
exposing raw domain request types directly to CLI or future desktop callers.

### Consequences

- runtime-owned default injection and normalization stay inside the app boundary instead of leaking
  through frontend code that constructs domain requests manually
- CLI and future GUI code can depend on one stable app-facing input shape for addon, addon index,
  addon lock, backup, bundle, and external-package mutations
- further contract cleanup can now focus on which shared value objects should remain cross-layer
  types and which ones still need app-owned wrappers, rather than first untangling raw domain
  request leakage

## ADR-018: Author-Package Import Defaults Are Explicit per Resource Group, not Merge-First

### Status

Accepted on 2026-04-18

### Decision

Direct external author-package import should default to one explicit resource-group profile:

- `create_backup = true`
- `addons = mirror`
- `wtf_common = share`
- `wtf_characters = replace_selected`
- `fonts = mirror`
- `interface_assets = mirror`

This profile must be shared by core manifest creation and CLI override composition.
If the caller overrides only some groups, unspecified groups inherit this shared profile instead of
falling back to `merge`.

### Consequences

- default author-package import behaves like a real setup sync path instead of a merge-only prototype
- stale addons, fonts, and interface assets are removed by default when the author package owns
  those groups
- common `WTF` remains conservative by default, while character `WTF` remains explicit and
  target-scoped
- CLI policy flags no longer have the surprising side effect of resetting other groups back to old
  merge semantics

## ADR-019: Transition Code Should Be Deleted Promptly after the Cleaner Path Lands

### Status

Accepted on 2026-04-18

### Decision

When a new path fully replaces an older bridge or transitional helper, the project should delete
the old path in the same bounded refactor stream instead of preserving long-lived dual-track logic
for comfort.

### Consequences

- workstream planning should prefer small, test-backed slices that end with deletion of obsolete code
- direct external-package import, planner cleanup, and future app-contract work should not keep
  redundant bridges once the new contract is verified
- the repository stays easier to reason about because architecture documents describe one intended
  path instead of a permanent stack of historical fallback layers

## ADR-020: Public Plan Payloads Stay Logical and Exclude Execution State

### Status

Accepted on 2026-04-18

### Decision

Public dry-run payloads such as `BundleApplyPlan` and `ExternalPackageApplyPlan` should expose only
logical preview data:

- group policies, target selection, and character mappings
- logical operations with action, scope, normalized archive identity, and destination
- plan summaries, helper strategy, manifest, and external-package normalization analysis

They should not expose execution-only state such as rewrite vectors, source-entry maps, prepared
apply-source details, temporary staging paths, byte-materialization flags, or rollback bookkeeping.
Execution-specific payloads must be projected from the resolved logical preview only at the apply
boundary.

### Consequences

- CLI and future GUI dry-run flows can depend on one stable preview contract without learning apply
  internals
- planner refactors can keep deleting execution-shaped helpers without changing public plan models
- if execution later needs more source-specific data, that data must remain internal to prepared
  apply types instead of leaking into plan serialization

## ADR-021: GUI Stability Starts with a Smaller App Service Set

### Status

Accepted on 2026-04-18

### Decision

The first GUI-stable `core::app` contract should not promise every current app service at once.
The initial stable service set is:

- `InstallationService`
- `AddonService`
- `BundleService`
- `ExternalPackageService`
- `BackupService`

`AddonIndexService` and `AddonLockService` remain available app services, but they are not part of
the first-wave GUI-stable contract yet.

### Consequences

- future `egui` work has one explicit stable entry set for installation, addon, bundle,
  external-package, and backup flows
- advanced curation and reproducibility flows can continue evolving without forcing premature
  stability promises on addon-index and addon-lock contracts
- `HearthSyncApp` should expose an explicit code boundary for the stable service set instead of
  relying on documentation alone

## ADR-022: App Service Requests Share an App-Owned Resolved Installation Value

### Status

Accepted on 2026-04-18

### Decision

`InstallationService::resolve` should return an app-owned resolved installation value, and app
service requests that target a specific WoW installation should accept that same value instead of
exposing domain `DetectedFlavorInstallation` directly.

### Consequences

- stable frontend callers can resolve an installation once and reuse the same app-owned value
  across addon, bundle, external-package, backup, addon-index, and addon-lock requests
- request-side contract cleanup now has one shared value object boundary instead of many service-
  specific leaks of the domain install model
- further R3 cleanup can focus on remaining domain-owned request payloads such as manifests,
  apply defaults, mapping inputs, and addon metadata

## ADR-023: Stable Bundle and External-Package Strategy Inputs Use App-Owned Values

### Status

Accepted on 2026-04-18

### Decision

Stable frontend-facing bundle and external-package requests should use shared app-owned value
objects for apply strategy input:

- target-account and character mapping input uses `BundleApplyMappingsValue`
- author-package default policy overrides use `BundleApplyDefaultsValue`

Frontend callers should not need domain `BundleApplyMappings` or manifest `ApplyDefaults` just to
drive stable `BundleService` or `ExternalPackageService` flows.

### Consequences

- CLI and future `egui` code can build apply-strategy input against the app boundary instead of
  learning bundle or manifest-domain structs
- the shared author-package default profile is now reachable from the app boundary, not only from
  domain-level helpers
- `BundleManifest`, addon metadata, and remaining install/platform enums are still part of the
  next request/result cleanup slices, but apply-strategy input is no longer one of the larger
  domain leaks on the first-wave stable service set

## ADR-024: Stable Addon Metadata Uses One App-Owned Value

### Status

Accepted on 2026-04-18

### Decision

Stable addon-facing request and result contracts should use one shared app-owned addon package
metadata value instead of exposing domain `AddonPackageMetadata` directly.

This applies at least to:

- `InstallAddonAppRequest.metadata`
- tracked-package metadata returned from addon inventory, update, and remove result payloads

### Consequences

- stable addon callers can pass curated metadata through the app boundary without learning the
  addon domain model
- addon inventory and mutation results now return the same app-owned metadata shape that addon
  install requests accept
- remaining `R3` request/result cleanup can focus on larger domain leaks such as manifest
  ownership and shared platform/flavor enums instead of carrying a duplicate metadata DTO at the
  stable addon boundary

## ADR-025: Stable Full Manifest Payloads Use One App-Owned Value Tree

### Status

Accepted on 2026-04-18

### Decision

Stable app-facing full manifest payloads should use one shared app-owned manifest value tree
instead of exposing domain `BundleManifest` directly on requests while returning a separate
response-only manifest DTO.

This applies at least to:

- `PackBundleAppRequest.manifest`
- bundle creation results that return a full manifest
- bundle apply / external-package plan / external-package apply results that return a full manifest

Inspection-oriented resource summaries may still keep separate result DTOs when they intentionally
add derived counts or other preview-only convenience fields.

### Consequences

- stable callers now submit and receive the same app-owned full-manifest shape across bundle and
  external-package flows
- `core::app` no longer needs a split between request-side domain `BundleManifest` and response-
  side `BundleManifestResult` for the same logical payload
- remaining `R3` cleanup can now focus on the smaller enum/value leaks such as `WowFlavor`,
  `HostPlatform`, and `CharacterMappingMode`, plus any thin-forwarder behavior that still lives in
  service wrappers
