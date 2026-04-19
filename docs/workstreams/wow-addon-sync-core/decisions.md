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
- `StableAppServices` should expose explicit direct and task entrypoints for that stable service set
  so the GUI-stable boundary is a real callable contract, not just a container of lower-level
  services

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
- `BundleManifest`, addon metadata, and install/platform enums can be cleaned up in smaller,
  independent slices because apply-strategy input is no longer one of the larger domain leaks on
  the first-wave stable service set

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
  ownership and any remaining shared enum ownership instead of carrying a duplicate metadata DTO at
  the stable addon boundary

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
- remaining `R3` cleanup can now focus on service behavior ownership and runtime policy injection
  instead of carrying another broad request/result DTO migration after the manifest value tree
  becomes app-owned

## ADR-026: Stable App Contracts Use App-Owned Platform and Flavor Values

### Status

Accepted on 2026-04-19

### Decision

Stable frontend-facing `core::app` contracts should use shared app-owned value enums for platform
and flavor identity:

- `HostPlatformValue`
- `WowFlavorValue`

This applies at least to:

- `AppRuntime` host-platform policy
- installation resolve and inspect requests
- resolved installation results
- bundle or external-package source metadata exposed through app contracts

Frontend callers should not need install-domain `HostPlatform` or `WowFlavor` enums just to
express host defaults, selected installation flavor, or source compatibility metadata.

### Consequences

- CLI and future `egui` code now build platform and flavor input entirely against the stable
  app-owned boundary instead of mixing app DTOs with install-domain enums
- runtime-owned host-platform policy no longer leaks install-domain enums through `AppRuntime`
- this decision narrows later cleanup so the remaining `R3` work can focus on service behavior and
  runtime policy ownership instead of keeping `HostPlatform` and `WowFlavor` as ambient boundary
  leaks

## ADR-027: Runtime-Backed Default Injection Belongs to App Request Contracts

### Status

Accepted on 2026-04-19

### Decision

When a frontend-facing app operation depends on runtime defaults such as backup destination,
bundle output destination, or external-package source platform, that default injection should live
on the app request contract rather than being reimplemented as scattered private normalization
helpers in each service wrapper.

This applies at least to:

- addon, addon-index, and addon-lock mutation requests that can create backups
- backup create/list/restore requests
- bundle pack/apply requests
- external-package bundle/create/plan/apply requests

### Consequences

- shared runtime defaults are now applied consistently across the app boundary, including addon,
  addon-index, and addon-lock mutation flows that previously did not honor the default backup
  directory
- service wrappers become thinner in the right direction: they call app-owned request normalization
  instead of each carrying fragmented path/default patching logic
- remaining `R3` behavior cleanup can focus on orchestration ownership and capability boundaries
  instead of repeating simple runtime default injection in multiple services

## ADR-028: Default Addon-Provider Configuration Uses App-Owned Runtime Values

### Status

Accepted on 2026-04-19

### Decision

The stable `core::app::AppRuntime` boundary should not require provider-domain option structs when
the caller only wants to configure default addon acquisition behavior.

Default-provider runtime configuration now uses app-owned values:

- `AddonProviderOptionsValue`
- `AddonProviderRetryPolicyValue`

Custom provider injection remains available for tests or advanced embedding, but the runtime should
make it explicit whether it is using configurable default-provider options or a fully injected
provider implementation.

### Consequences

- cache-dir and retry-policy configuration now stays inside the app-owned runtime contract instead
  of leaking `AddonProviderOptions` through the frontend boundary
- future CLI or `egui` configuration UIs can bind to stable app-owned runtime values without
  learning provider-domain structs
- any future helper-capability work should follow the same pattern: runtime-owned app values first,
  planner or provider internals second

## ADR-029: Custom Addon-Provider Injection Is Not a Stable Frontend Contract

### Status

Accepted on 2026-04-19

### Decision

`core::app::AppRuntime` may still need a seam for injecting a fake or specialized addon provider
inside the crate, especially for app-service tests.
That seam should not remain part of the public frontend-facing runtime contract.

The public app boundary now exposes provider configuration only through app-owned runtime values,
while direct custom provider injection is restricted to crate-internal composition.

### Consequences

- CLI and future `egui` callers configure addon acquisition through stable runtime values instead of
  domain provider traits
- fake-provider and specialized-provider wiring still exists for app-layer tests without forcing the
  same trait seam into the public app API
- future optional helper capabilities should follow the same rule: frontend-facing contracts expose
  stable app-owned capability settings, while low-level injected implementations stay internal

## ADR-030: Stable Task Contracts Are Published Through `core::app`

### Status

Accepted on 2026-04-19

### Decision

Long-running app-service contracts should not require frontend callers to import the lower-level
`core::task` module directly just to collect progress, stream callbacks, or satisfy task-related
trait bounds.

The stable task contract is now published through `core::app`:

- `CancellationToken`
- `TaskProgressSink`
- `TaskRun`
- `TaskProgressEvent`
- `TaskKind`
- `TaskPhase`

### Consequences

- CLI and future `egui` code can stay on the `core::app` import surface for runtime, services,
  requests, results, and task progress contracts
- `core::task` remains reusable internal infrastructure, but it is no longer the frontend-facing
  path for stable app-service progress behavior
- future task-behavior refactors can keep the same app-facing import surface even if internal task
  plumbing changes underneath

## ADR-031: Helper Strategy Is Runtime Capability State, Not Planner State

### Status

Accepted on 2026-04-19

### Decision

The selected helper strategy belongs to runtime capability state.
It should not be stored on bundle-domain planning DTOs just because plan results currently expose it
to frontend callers.

Bundle and external-package domain plans now carry only bundle semantics and logical preview data.
`core::app::AppRuntime` owns helper strategy state, and app-layer plan results project that state
back into frontend-facing response payloads.

### Consequences

- bundle-domain planning no longer fabricates helper capability state with a hardcoded
  `NativeRust` value
- future helper backends can be introduced by extending runtime-owned app contracts instead of
  teaching the planner about frontend capability reporting first
- public plan results still expose helper strategy, but that field now reflects app runtime state
  rather than a planner implementation detail

## ADR-032: App Response Projection Does Not Publish Public Domain Conversion Traits

### Status

Accepted on 2026-04-19

### Decision

Stable `core::app` result types may still need to project domain outputs into frontend-facing DTOs,
but that projection should remain an internal app-layer detail.

When the app boundary needs domain-to-app response mapping, it should prefer crate-internal factory
methods such as `from_domain(...)` over public `impl From<DomainType> for AppResultType>`.

### Consequences

- frontend callers are less likely to treat domain DTOs as part of the stable app contract just
  because a convenient public conversion trait exists
- app result types can keep changing their internal projection path without exposing more domain
  coupling through trait resolution
- the response boundary becomes more consistent with the rest of the `core::app` refactor: app
  contracts stay public, domain conversion seams stay internal
- the same rule now applies across bundle, external-package, installation, addon, backup,
  addon-index, addon-lock, and bundle-addon-lock response DTOs, so frontend callers see one
  consistent projection boundary instead of a mixed trait surface

## ADR-033: Runtime Capability State Uses an App-Owned Snapshot Contract

### Status

Accepted on 2026-04-19

### Decision

Optional provider and helper behavior should be observable through one app-owned runtime
capability contract instead of a mix of ad hoc getters, optional provider-option fields, or
planner-projected details.

`core::app::AppRuntime` now exposes `AppRuntimeCapabilitiesValue`, and the same snapshot is also
available from `HearthSyncApp` and `StableAppServices`.
That snapshot owns:

- provider mode: configured default provider options vs. internal custom provider
- helper capability reporting owned by app runtime, not planner or service-local state

### Consequences

- frontend callers can inspect runtime capability state without inferring meaning from `Option`
  fields such as missing provider options
- provider/helper capability reporting now has one stable app boundary that future CLI and `egui`
  work can share
- future non-native helper integration can extend one runtime-owned capability contract instead of
  surfacing first through planner DTOs or service-local composition details

## ADR-034: HearthSyncApp Exposes Direct Frontend Entry Points

### Status

Accepted on 2026-04-19

### Decision

`HearthSyncApp` should not remain only a service factory.

The frontend root may still expose raw service accessors for advanced composition and specialized
task entrypoints, but common direct app operations should also be callable from `HearthSyncApp`
itself.

That direct entry surface now covers the primary installation, addon, bundle, external-package, and
backup flows, plus the current CLI-facing addon-index and addon-lock direct operations.
For the stable long-running flows, `HearthSyncApp` should also forward collecting-progress and
callback entrypoints so a frontend can stay on the app root while consuming task behavior.

### Consequences

- CLI callers can stay on one app-root import surface instead of manually coordinating
  `installations()` plus another service for routine flows
- future `egui` code can start from `HearthSyncApp` as a credible frontend root and only drop down
  to raw services when it actually needs task-specific or advanced composition seams
- service accessors remain available, but they are now a secondary composition API instead of the
  only practical entry path

## ADR-035: App Inputs Own Installation Policy and Thin Domain Projection

### Status

Accepted on 2026-04-19

### Decision

If a frontend-facing app service only needs to apply runtime installation policy or translate a
stable app-owned input into a small set of domain arguments, that normalization should live on
`AppRuntime` or the app request contract instead of remaining inside the service wrapper.

This applies at least to:

- installation scan policy such as configured scan roots plus host-platform selection
- installation inspect and resolve host injection
- thin installation-targeted read or plan inputs such as addon inventory, addon-lock inspect/write/
  verify/plan, and bundle preview planning

### Consequences

- `InstallationService` is closer to a real boundary caller and no longer decides scan-root
  branching or host injection itself
- thin app services no longer keep repeating `request.installation.into()` glue when that step is
  part of the stable app input contract rather than service-specific behavior
- future frontend contract cleanup can focus on real policy ownership gaps instead of preserving
  service-local argument reshaping that belongs to runtime or request helpers

## ADR-036: Runtime-Backed Mutation Requests Own Normalized Domain Projection

### Status

Accepted on 2026-04-19

### Decision

When an app-facing mutation request depends on runtime defaults before it can become a domain
request, that request contract should expose one normalized projection helper that performs both:

- runtime-backed default injection
- final projection into the domain mutation request

App services should not keep coordinating this as a repeated two-step protocol such as
`request.apply_runtime_defaults(&self.runtime).into()`.

### Consequences

- addon, addon-index, addon-lock, backup, bundle, and external-package mutation services now read
  more clearly as boundary callers because request normalization and domain projection live on the
  request contract itself
- future request evolution can change defaulting or projection details in one place without touching
  every service wrapper that executes the mutation
- the remaining `core::app` cleanup can focus on meaningful policy or orchestration seams instead of
  preserving duplicated request-normalization choreography in service bodies

## ADR-037: External-Helper Capability State Is Explicitly Separate from Active Strategy

### Status

Accepted on 2026-04-19

### Decision

The app runtime capability contract should not use one `helper_strategy` field to represent all of
the following at once:

- whether the frontend wants to allow an external helper
- whether such a helper is currently available
- which strategy is actually active for the current runtime

`AppRuntimeCapabilitiesValue` now exposes an explicit `external_helper` snapshot with policy and
availability, while bundle and external-package plan/apply results continue to report the active
`helper_strategy`.

### Consequences

- future optional helper backends can be integrated without overloading one enum with both desired
  policy and actual execution state
- frontend callers can distinguish “prefer external helper, but none is available” from “never
  requested an external helper” while still reading the currently active `helper_strategy`
- helper-assisted paths remain optional accelerators instead of becoming ambient planner or service
  assumptions before a concrete helper backend exists
