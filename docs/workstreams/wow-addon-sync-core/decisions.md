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

The first stable top-level reusable API surface is `core::app::StableAppServices`.
CLI and future desktop code should enter the stable installation, addon, bundle,
external-package, and backup flows through that shared app boundary instead of treating domain
install helpers as frontend-facing public API.
`ExtendedAppServices` remains the broader app root for less-stable addon-index, addon-lock, and related
extension flows that should not be part of the first stable contract wave.

### Consequences

- one shared `AppRuntime` becomes the canonical place for host-platform, install-scan, provider,
  backup, and bundle-output policy
- `core::install::{scan_installations, inspect_installation, resolve_installation}` can move to
  crate-internal support status while the stable installation contract lives under
  `core::app::StableAppServices`
- callers that truly need addon-index or addon-lock behavior can opt into `ExtendedAppServices`
  explicitly instead of inheriting those stability assumptions by default
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
- `ExtendedAppServices` should expose an explicit code boundary for the stable service set instead of
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
- `TaskProgressCode`
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

## ADR-040: Stable Task Progress Must Stay Human-Readable And Machine-Readable

### Status

Accepted on 2026-04-21

### Decision

The stable task contract must keep the existing human-readable `message` field for CLI and logs,
but it must also expose structured progress data that a future GUI can consume without parsing
strings.

`TaskRun` now carries one generated `task_id`, and `TaskProgressEvent` may carry:

- `task_id`
- `code`
- `current`
- `total`
- `bytes_current`
- `bytes_total`
- `bytes_per_second`

Task ids are generated inside the shared `core::task` wrapper layer rather than by each business
operation.
Task-specific execution loops should emit structured step progress through shared helpers instead
of inventing ad hoc event payloads at each call site.

### Consequences

- CLI keeps its current readable progress messages instead of switching to raw machine codes
- future `egui` work can group callback or collected-progress streams by `task_id`
- step-oriented work such as addon directory mutation, backup restore, bundle apply execution, and
  metadata-only lock actions can expose deterministic `current/total` counts without string parsing
- byte-oriented download progress can be added incrementally later without changing the app-facing
  event shape again

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
available from `ExtendedAppServices` and `StableAppServices`.
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

## ADR-034: ExtendedAppServices Exposes Explicit Extension Entry Points

### Status

Accepted on 2026-04-19

### Decision

`ExtendedAppServices` should not remain only a service factory, but it also should not silently become
the stable API surface through implicit compatibility behavior.

The stable installation/addon/bundle/external-package/backup contract belongs on
`StableAppServices`. `ExtendedAppServices` should compose that boundary explicitly and add the
less-stable addon-index, addon-lock, and bundle-addon-lock entrypoints on top.

When callers need both surfaces, they should cross the boundary intentionally through an explicit
stable bridge instead of relying on `Deref`-style implicit forwarding.

### Consequences

- stable CLI and future `egui` callers converge on `StableAppServices` as the default reusable
  contract instead of picking up addon-index/addon-lock stability by accident
- `ExtendedAppServices` remains available as the broader extension root when advanced reproducibility or
  curation workflows are explicitly needed

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

## ADR-038: Bundle Export Paths Must Not Depend on Caller Working Directory

### Status

Accepted on 2026-04-19

### Decision

Bundle export and bundle sidecar resolution should not silently depend on `std::env::current_dir()`
for portable behavior.

Specifically:

- default bundle output paths should resolve from explicit product inputs such as
  `manifest_base_dir`, runtime output defaults, or installation-derived base directories
- relative explicit output paths should resolve against the same explicit base rules
- relative addon-index references embedded through bundle metadata should require an explicit
  `manifest_base_dir` instead of falling back to ambient process state

### Consequences

- CLI, GUI, and tests now share one deterministic bundle-export contract instead of inheriting
  whatever working directory the caller process happened to use
- portable bundle metadata no longer smuggles relative sidecar resolution through ambient `cwd`
- the remaining portability hardening can focus on true archive metadata and case-folding edges
  instead of preserving process-global path assumptions in bundle export

## ADR-039: Default Public Plans Are Logical and Conservative

### Status

Accepted on 2026-04-21

### Decision

Default public dry-run plans such as `bundle plan` and `external-package plan` should stay logical
and conservative rather than reading archive bytes or previewing rewrites to compute exact
`skip` versus `replace` outcomes for existing targets.

When the logical planner reaches an entry whose destination already exists and no deterministic
`add` or `preserve` rule already applies, the public plan should report that entry as a replace
candidate.
Exact identical-file detection remains part of prepare/apply paths, where deeper source reads,
rewrite preview, and byte comparison are justified immediately before execution.

### Consequences

- public plan latency no longer scales with archive-byte reads or rewrite-aware content comparison
  for existing targets
- future GUI previews can use the default plan contract as a fast intent view without depending on
  accidental deep-compare behavior
- public plan counts become intentionally conservative for existing targets: some entries planned as
  `replace` may later resolve to `skip` during prepare/apply
- exact execution-time skipping still remains available where it matters operationally, so apply
  behavior does not regress just because public plan became cheaper

## ADR-041: Provider Download Byte Progress Reuses the Stable Task Event Shape

### Status

Accepted on 2026-04-21

### Decision

Provider-backed archive downloads should not introduce a second frontend-facing progress channel or
a provider-specific task payload.
Low-level HTTP downloads may use internal observers, but app-facing progress must continue to flow
through `TaskProgressEvent` using `TaskProgressCode::DownloadArchive` plus the existing
`bytes_current`, `bytes_total`, and `bytes_per_second` fields.

This applies at least to:

- addon install and update
- addon-index install and update
- addon-lock source preparation during apply

### Consequences

- CLI and future `egui` callers keep one task-stream contract for both step progress and transfer
  progress
- provider and HTTP observer details stay internal and can evolve without breaking the app boundary
- future download-capable flows extend the same event shape instead of adding a parallel callback
  or DTO surface

## ADR-042: Existing Local Addon State Enters Tracking through Explicit Snapshot Adoption

### Status

Accepted on 2026-04-22

### Decision

When a WoW installation already contains usable addon directories but has no HearthSync tracked
registry yet, the bootstrap path should be explicit addon adoption rather than ambient folder
scanning, fake remote-source reconstruction, or a new self-referential local-directory source
kind.

Specifically:

- the operator must explicitly choose which untracked addon directories to adopt
- multi-addon grouping requires an explicit tracked `package_id`
- HearthSync snapshots those directories into a real local archive and records that archive as the
  tracked `local_archive` source
- the bootstrap flow must not pretend to know the original CurseForge/GitHub/HTTP identity when
  the local machine no longer has that information

### Consequences

- the current addon source model remains honest and reusable for CLI plus future GUI work
- manual local installs can still enter tracked state and immediately unlock curator scaffold or
  suggestion workflows
- follow-up “upgrade this adopted snapshot to a real curated/provider source” flows remain possible
  later without overloading the initial bootstrap step with guesswork

## ADR-043: Source Relink Upgrades Tracking Truth Without Reinstalling AddOn Files

Accepted on 2026-04-22

### Decision

When an operator already has one tracked addon package and later learns the package's real source,
HearthSync should support an explicit source relink step instead of forcing reinstall just to
rewrite registry identity.

The first implementation is deliberately conservative:

- `addon relink` targets exactly one tracked package selected by tracked package id or addon
  directory name
- HearthSync prepares the candidate source and requires the prepared addon-directory set to match
  the currently tracked package exactly
- on success, HearthSync rewrites only the tracked registry source and leaves live AddOns files
  untouched
- generic relink clears stored package metadata rather than preserving possibly stale index/source
  details that no longer truthfully describe the new source

### Consequences

- adopted local snapshots now have a clean upgrade path to real provider or archive sources
- relink stays low-risk because it does not mutate live addon content and fails closed on addon-set
  mismatches
- curator/index-aware attach remains a separate follow-up because generic relink does not yet
  repopulate curated metadata

## ADR-044: Curated Index Relink May Attach Metadata Without Reinstalling Matching Sources

Accepted on 2026-04-22

### Decision

Once generic source relink exists, curator attach should build on the same relink model instead of
falling back to reinstall semantics.

Specifically:

- `addon index relink` resolves one package from a curated addon index and targets exactly one
  tracked package, either by explicit operator choice or by the existing tracked-package matching
  heuristics
- the resolved source must still expose the exact same addon-directory set as the tracked package
- when the resolved source differs, HearthSync rewrites both registry source and curated metadata
- when the resolved source is already the same, HearthSync may still proceed if curated metadata
  would change; metadata attach alone is a valid outcome
- if both source and curated metadata already match, the operation fails as a no-op instead of
  pretending new work happened

### Consequences

- operators can promote adopted or manually tracked packages into curated index identity without
  reinstalling unchanged AddOns content
- generic relink remains the lower-level "source only" primitive, while curator relink becomes the
  truthful path for attaching index metadata
- exact addon-directory parity remains the shared safety rule across both relink paths

## ADR-045: Bulk Curator Attach Stays Reviewable And Fail-Closed

Accepted on 2026-04-22

### Decision

Once single-package curator relink exists, the higher-level multi-package workflow should batch the
same truthful attach model instead of inventing partial reinstall or partial registry-write
semantics.

Specifically:

- `addon index attach` selects one package or the whole curated index and reuses the existing
  suggestion/preflight matching order to map each index package onto at most one tracked package
- each matched package still prepares the resolved source and must prove exact addon-directory
  parity before it becomes attachable
- the operation returns a structured review result for ready packages, already-attached packages,
  blocked packages, and skipped unsupported-flavor packages
- dry runs may report mixed ready and blocked states, but non-dry execution remains fail-closed:
  if any selected package is blocked, HearthSync writes no registry changes at all
- when every selected package is ready, HearthSync rewrites tracked registry source plus curated
  metadata in one batch and still leaves live AddOns content untouched

### Consequences

- operators can bootstrap many locally tracked packages into curated identity without repeating
  one-package commands
- CLI and future GUI work get one stable review surface for bulk curator attach instead of parsing
  ad hoc errors from repeated single-package relink calls
- exact addon-directory parity remains the shared safety rule across single-package relink and
  bulk attach
- future partial-apply behavior, if it ever exists, must be an explicit higher-level decision
  rather than the default semantics of the first bulk attach flow

## ADR-046: Managed Addon State Defaults To App Data And Keeps Sidecar Portable

Accepted on 2026-04-22

### Decision

HearthSync still needs persisted addon state for tracked registry truth, lock reproducibility,
mutable update policy, and adopted snapshot archives, but that state should not default to writing
inside the live WoW client tree.

Specifically:

- managed addon state is now resolved through one runtime-owned backend abstraction
- the default backend is platform app-data keyed by installation identity
- sidecar `.hearthsync` remains supported as an explicit portable backend instead of the default
- the first migration scope covers addon registry, addon lock, addon policy, and adopted snapshot
  archives only; bundle sidecar metadata is intentionally unchanged for now

### Consequences

- default desktop operation no longer creates managed state files under `Interface/AddOns`
- scan-only and "just inspect the client" workflows stay compatible with NewBeeBox-style
  expectations that the tool should recognize installs without first mutating the client tree
- managed addon flows still keep the persisted truth they need for source identity, curator
  attachment, policy, and reproducibility
- the product can expose backend choice as one runtime-level control instead of scattering
  feature-specific path overrides; the CLI now does this through global
  `--addon-state-storage app-data|sidecar`
- bundle unpack sidecar metadata can stay an explicit portable artifact without forcing bundle
  export or bundle addon-lock shortcut flows to ignore the configured managed-state backend
- future GUI work can surface backend choice explicitly without re-deriving path rules per feature
- runtime diagnostics can now also project exact managed addon state paths when the caller supplies
  an installation context, which keeps CLI troubleshooting and future GUI settings views on the
  same runtime-owned path-resolution contract
- the canonical app-data root now uses application-only `ProjectDirs` naming to avoid the
  duplicated Windows `.../hearthsync/hearthsync/data/...` segment
- because HearthSync is still pre-release, obsolete managed-state path compatibility was deleted
  instead of being preserved behind legacy fallback branches or migration-state diagnostics

## ADR-047: Runtime Settings Persist Through An App-Owned Backend

Accepted on 2026-04-22

### Decision

Cross-invocation runtime defaults should not live only in one-shot CLI flags, and they should not
default to writing hidden product settings into the WoW client tree.

Specifically:

- persisted runtime settings are owned by `core::app`, not by CLI-only bootstrap code
- the shared backend stores selected runtime overrides under app-data `settings/runtime.toml`
- the first persisted fields are `addon_state_storage`, `addon_cache_dir`, and
  `http_no_validator_cache_policy`
- the first operator-facing mutation surface is the explicit CLI namespace
  `settings inspect|set|reset`
- runtime assembly merges values in one order: built-in defaults, then persisted settings, then
  one-shot CLI flags for the current invocation

### Consequences

- CLI and future GUI work now have one shared persistence contract for runtime defaults instead of
  parallel settings stores
- cache policy persistence no longer depends on retyping global flags on every invocation
- addon-state backend choice can persist without making sidecar `.hearthsync` the desktop default
- future GUI settings work should layer on the same app-owned backend rather than inventing a
  second persistence path
- invalid persisted settings fail closed during runtime assembly or settings inspection until the
  operator fixes or resets them

## ADR-048: Pre-GUI Hardening Stays In The Core Workstream

Accepted on 2026-04-28

### Decision

The 2026-04-28 architecture-hardening findings should remain inside
`docs/workstreams/wow-addon-sync-core` instead of becoming a new top-level workstream.

Specifically:

- addon update transaction completion belongs to the shared mutation engine
- ignored-policy ordering belongs to shared addon-index planning
- config-owned DTO work belongs to the stable app boundary
- provider decomposition belongs to the addon acquisition core
- live task progress and cancellation belong to the app task contract
- clippy cleanup is a repo-wide quality gate, but the meaningful boundary pressure is concentrated
  in the core

### Consequences

- the core workstream remains the architecture source of truth for CLI and future `egui` callers
- the new review is tracked as `review-2026-04-28-architecture-hardening.md`
- `todo.md` and `milestones.md` carry the follow-up work as `R9` / `M9`
- because HearthSync is still pre-release, this hardening phase may delete obsolete transition code
  and reshape app contracts instead of preserving compatibility for current internal call sites

## ADR-049: Local Archive Paths Resolve At The App Boundary

Accepted on 2026-04-28

### Decision

Relative local addon archive paths are frontend input, not persisted core state.

Specifically:

- CLI runtime assembly records the invocation directory as `AppRuntime`'s absolute relative-path
  base.
- App addon install and relink requests resolve relative local zip sources against that runtime
  base before projecting into domain requests.
- Persisted tracked-registry `local_archive` sources, addon-lock `local_archive` source refs, and
  explicit addon-lock source overrides must already be absolute before core planning or source
  materialization uses them.
- Sidecar addon-lock source-index entries remain lock-relative because they are portable bundle
  metadata with an explicit lock-file base, not ambient caller input.

### Consequences

- CLI keeps the expected `--source ./addon.zip` behavior without making core logic depend on
  process cwd.
- Future `egui` callers can choose a file-dialog directory or another explicit base instead of
  inheriting whatever directory launched the process.
- Hand-edited registry or lock files with relative local archive sources fail closed before update
  or lock apply work can read from the wrong directory.

## ADR-050: Addon Index File Paths Resolve At The App Boundary

Accepted on 2026-04-28

### Decision

Addon index file paths are frontend input and must resolve before addon-index core logic reads or
writes them.

Specifically:

- Relative app-level addon-index file paths resolve against `AppRuntime`'s absolute relative-path
  base.
- This applies to inspect, validate, suggest, scaffold, attach, install, update, and relink
  requests.
- Local archive paths declared inside an addon index stay relative to the index file location.

### Consequences

- CLI keeps the expected `--file ./addons.toml` behavior through runtime assembly rather than
  implicit core cwd.
- Future GUI callers can choose a file-dialog base or pass absolute paths without depending on the
  process launch directory.
- Curated indexes retain portable sidecar archive references because the archive source base is the
  resolved index file path, not the caller's cwd.

## ADR-051: Bundle And External Package Inputs Resolve At The App Boundary

Accepted on 2026-04-28

### Decision

Bundle archive inputs and external/config package source inputs are frontend file selections and
must resolve before stable app services call bundle or external-package core code.

Specifically:

- Relative bundle archive paths resolve against `AppRuntime`'s absolute relative-path base before
  inspect, plan, unpack, embedded addon-lock plan, or embedded addon-lock apply work reads them.
- Relative external-package and config source paths resolve against the same runtime base before
  analysis, temporary bundle creation, plan, or apply work scans the source.
- Relative bundle manifest base directories resolve at the app boundary because they are the
  explicit base for manifest sidecar inputs such as addon indexes.
- Bundle output paths keep the existing bundle-domain rule: relative output references resolve
  against the manifest/installation-derived output base, not the app relative-path base.

### Consequences

- CLI keeps the expected `--bundle ./ui.bundle.zip` and `--source ./AuthorPack` behavior via
  runtime assembly instead of ambient process cwd inside core services.
- Future GUI callers can pass relative file-dialog results with an explicit base, or absolute
  paths with no runtime base dependency.
- Export destination semantics stay separate from read-input semantics, avoiding accidental
  changes to bundle output placement.

## ADR-052: Lock And Backup Selection Paths Resolve At The App Boundary

Accepted on 2026-04-28

### Decision

Addon-lock file selections and backup restore/list selections are frontend input and must resolve
before app services call addon-lock or backup core code.

Specifically:

- Relative addon-lock diff paths, verify/plan/apply lock paths, and explicit source override
  archive paths resolve against `AppRuntime`'s absolute relative-path base.
- Relative backup list directories, restore directories, and restore archive paths resolve against
  the same runtime base before backup catalog or restore selection logic runs.
- Backup output directories remain a separate output-path decision because the same option shape is
  shared across addon, addon-index, addon-lock, bundle, external-package, and backup creation
  flows.

### Consequences

- CLI keeps the expected `addon-lock diff ./left.toml ./right.toml` and `backup restore
  --archive ./backup.zip` behavior through runtime assembly rather than core cwd.
- Future GUI callers can make lock/backup file-dialog bases explicit instead of relying on the
  process launch directory.
- The remaining backup-output cleanup can be done consistently across every mutation flow instead
  of changing only one caller family.

## ADR-053: Output Paths Resolve At The App Boundary

Accepted on 2026-04-28

### Decision

App-level output selections are frontend output choices and must resolve before app services call
domain code.

Specifically:

- Relative mutation backup output directories resolve against `AppRuntime`'s absolute
  relative-path base before addon, addon-index, addon-lock, bundle, external-package, or config
  apply work begins.
- Relative backup creation output directories resolve against the same runtime base before backup
  domain code creates archives.
- Relative addon adoption archive output paths and external-package bundle output directories also
  resolve at the app boundary.
- Bundle pack output paths keep the existing bundle-domain rule: relative paths resolve against
  the manifest/installation-derived output base, not the app relative-path base.

### Consequences

- CLI keeps expected `--backup-output ./backups`, `backup create --output ./backups`, adoption
  archive, and author-package bundle output behavior without letting core services infer process
  cwd.
- Future `egui` callers can attach output choices to an explicit file-dialog or project base.
- Bundle export remains intentionally different because it already has a domain-owned portable
  placement model tied to the selected manifest and installation.

## ADR-054: Installation Paths Resolve At The App Boundary

Accepted on 2026-04-28

### Decision

Installation path selections and configured installation scan roots are frontend inputs and must
resolve before install-discovery core code probes the filesystem.

Specifically:

- Relative inspect and resolve installation paths resolve against `AppRuntime`'s absolute
  relative-path base before installation classification runs.
- Relative configured installation scan roots resolve against the same runtime base before scan
  code checks whether roots exist.
- Relative installation paths without a runtime base fail closed at the app boundary instead of
  falling through to process-cwd filesystem probes.

### Consequences

- CLI keeps expected `--install ./World of Warcraft` behavior through runtime assembly rather than
  implicit core cwd.
- Future `egui` callers can resolve file-dialog selections against an explicit base.
- Installation discovery now follows the same boundary rule as addon, bundle, backup, and output
  selections.

## ADR-055: CLI Sidecar Files Resolve Through The Runtime Base

Accepted on 2026-04-28

### Decision

CLI-only sidecar files that are read before app requests are invoked must still resolve through the
same runtime relative-path base.

Specifically:

- Relative bundle manifest files used by `bundle pack` resolve before the CLI loads the manifest
  and derives the manifest base directory.
- Relative manifest validation files resolve before the CLI loads and validates them.
- Relative apply mapping files resolve before the CLI loads mapping overrides for bundle,
  external-package, or config apply/plan commands.

### Consequences

- CLI convenience file loading no longer reintroduces ambient process-cwd behavior after the app
  boundary was hardened.
- Bundle manifest base directories derived by CLI are absolute when they enter app requests.
- Future GUI code remains free to load these sidecar documents through its own file-dialog model,
  while CLI keeps one deterministic relative-path contract.

## ADR-056: Addon Cache Runtime Paths Are Absolute After The Boundary

Accepted on 2026-04-28

### Decision

Addon download cache directories are runtime filesystem policy and must not remain relative after
CLI or settings boundaries.

Specifically:

- Relative CLI `--addon-cache-dir` values resolve against the CLI runtime relative-path base before
  the default addon provider is constructed.
- Relative `settings set --addon-cache-dir` values resolve against `AppRuntime`'s absolute
  relative-path base before they are persisted.
- Persisted addon cache directories must be absolute when loaded; relative persisted values fail
  closed instead of drifting with a later invocation directory.

### Consequences

- Provider cache reads, repairs, purges, and materialization no longer depend on process cwd.
- Runtime diagnostics and capabilities report stable cache paths.
- Future GUI settings screens can store file-dialog selections as absolute paths while still
  accepting relative form input when they provide an explicit runtime base.

## ADR-057: Runtime Builder Owns Path Normalization

Accepted on 2026-04-28

### Decision

Runtime construction should have one fallible build step that validates and normalizes
runtime-owned filesystem paths before services receive an `AppRuntime`.

Specifically:

- `AppRuntimeBuilder` collects host platform, relative-path base, provider options, addon-state
  storage, helper policy, scan roots, and default output directories.
- Builder `build()` validates that the relative-path base is absolute when present.
- Builder `build()` resolves relative addon cache directories, installation scan roots, default
  backup directories, and default bundle output directories against that base.
- Direct default-provider construction through addon provider options is fallible, so relative
  cache paths cannot bypass the runtime base by being applied before the base exists.

### Consequences

- CLI and future GUI runtime assembly can express all runtime policy first and then build once.
- Runtime diagnostics for builder-created runtimes report normalized path policy.
- Runtime-owned paths now have one normalized representation after `build()`.

## ADR-058: Runtime Policy Is Immutable After Build

Accepted on 2026-04-28

### Decision

`AppRuntime` must be a read-only runtime policy snapshot after construction.

Specifically:

- `install_scan_roots`, `relative_path_base`, `default_backup_dir`, and
  `default_bundle_output_dir` can only be set on `AppRuntimeBuilder`.
- `host_platform`, `addon_state_storage_kind`, and `external_helper_policy` can only be set on
  `AppRuntimeBuilder`.
- `AppRuntimeBuilder::build()` remains the only place that can validate the runtime base and
  normalize relative runtime-owned paths.
- `AppRuntime` keeps read-only accessors plus request-time input/output resolution helpers.

### Consequences

- Future CLI and GUI assembly cannot accidentally bypass builder normalization by modifying a
  built runtime.
- Runtime diagnostics are more trustworthy because runtime policy is already normalized and cannot
  be replaced with unresolved or inconsistent values.
- Tests and fixtures now exercise the same fallible runtime construction path as production for
  path-bearing and policy-bearing settings.

## ADR-059: Resolved Installation DTOs Validate Before Core Projection

Accepted on 2026-04-28

### Decision

App-owned resolved installation values must be validated before they are projected back into core
installation structs.

Specifically:

- `ResolvedInstallationValue::into_domain()` is fallible.
- Every filesystem path inside a resolved installation DTO must be absolute before app services
  pass it to addon, bundle, backup, config, or policy core code.
- CLI callers normally receive these DTOs from `resolve_installation`, while future GUI callers can
  still deserialize or construct them, but app services fail closed if the DTO contains relative
  paths.

### Consequences

- A frontend cannot bypass runtime input-path resolution by constructing a relative
  `ResolvedInstallationValue`.
- Core services continue to receive deterministic installation trees instead of inheriting ambient
  process cwd from app-layer DTOs.
- Request projection methods now surface invalid installation DTOs as app validation errors before
  any filesystem mutation is planned.

## ADR-060: Bundle Pack Output Paths Resolve Before Core Projection

Accepted on 2026-04-28

### Decision

App-owned bundle pack requests must resolve explicit output paths before they are projected into
core bundle packing requests.

Specifically:

- Absolute bundle pack output paths pass through unchanged.
- Relative bundle pack output paths keep the existing bundle placement rule:
  - when a manifest base directory is present, they resolve under that manifest base;
  - otherwise, they resolve under the selected installation product root's parent directory, or the
    product root itself when it has no parent.
- Runtime default bundle output directories still normalize in `AppRuntimeBuilder` and remain
  absolute when injected into app requests.

### Consequences

- Future GUI callers cannot send an explicit relative bundle output path into core packing code.
- CLI behavior stays stable for `--output exports` next to a manifest loaded from a subdirectory.
- Core packing keeps its deterministic default placement behavior, but the stable app boundary no
  longer relies on core to interpret frontend-provided relative output paths.

## ADR-061: Addon Provider Options Validate Before Provider Construction

Accepted on 2026-04-29

### Decision

App-owned addon provider options must be validated before `AppRuntimeBuilder` constructs the
default addon provider.

Specifically:

- `AddonProviderRetryPolicyValue::into_domain()` is fallible.
- `AddonProviderOptionsValue::into_domain()` is fallible because it owns nested retry-policy
  and HTTP no-validator cache-policy projection.
- `max_attempts` must be greater than zero. The app/runtime boundary rejects zero instead of
  relying on the provider's internal `max(1)` execution guard.
- `HttpNoValidatorCachePolicyValue::ReuseWithinWindow` must use a positive `max_age_secs`.
  `AlwaysRefresh` remains the explicit no-reuse policy.

### Consequences

- Future GUI callers cannot build a runtime whose displayed retry policy says zero attempts while
  the provider silently performs one attempt.
- Future GUI callers and hand-edited persisted settings cannot express a zero-second no-validator
  reuse window with ambiguous behavior; they must choose a positive window or `AlwaysRefresh`.
- Provider internals can keep defensive guards, but frontend/runtime configuration semantics stay
  explicit and validation-driven.
- Runtime construction remains the single fallible step for path-bearing and provider-policy
  settings.
