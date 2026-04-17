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
