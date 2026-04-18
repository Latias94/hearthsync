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
