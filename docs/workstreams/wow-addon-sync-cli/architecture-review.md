# Architecture Review - Sync Semantics Hardening

Date: 2026-04-15

## Summary

The repository now has a runnable prototype, but the next product risk is not missing CLI surface area. The next risk is unclear synchronization semantics.

Before expanding into a frontend, HearthSync should stabilize the core contract for:

- what each resource group means during apply
- which files may be removed, replaced, merged, or preserved
- where transaction boundaries start and end
- how Lua rewrites are scoped and validated
- which APIs form the stable CLI/Core/frontend boundary

## Review Judgment

The external review is directionally correct:

- Bundle apply currently behaves like merge-copy for most resources, so stale files can remain.
- Manifest apply intent exists, but runtime behavior does not fully consume it.
- Bundle planning currently performs staging and rewrite preprocessing, so it is not a clean pure planning boundary.
- Addon lock apply chains multiple mutating subflows and does not yet have one transaction boundary.
- Lua rewriting is useful but too broad for real `SavedVariables` without an explicit file policy and encoding handling.
- Several modules are already large enough that future GUI integration would benefit from clearer core interfaces.

One item is partially outdated: bundle-local addon source archives now make lock-driven addon sync portable across machines without requiring source-machine local archive paths. This improves transport, but it does not solve the broader apply semantics and redistribution policy questions.

## NewBeeBox Research Alignment

`targets/newbeebox/RESEARCH.md` reinforces the same architectural direction:

- NewBeeBox models `share` and `sync` as different first-class WTF modes, not as one generic copy operation.
- The practical WTF root is `WTF/Account`, with separate account, server, and role scopes.
- Role detection is file-based through artifacts such as `AddOns.txt` and `config-cache.wtf`, not purely directory-name based.
- Migration combines copy, profile-key rewriting, role identity rewriting, and explicit remapping.
- Blacklists are part of the model, especially for files such as bindings and macros caches.
- Some Lua migration is byte-oriented, so UTF-8-only text replacement is insufficient.
- Deleting the target before copying is deliberate for some sync paths.
- Generic mod installation is represented as a long-running task with progress, cancellation, and uninstall metadata.

The strongest implication is that HearthSync should model behavior by resource semantics, not just paths. `WTF` needs account-root, account `SavedVariables`, role `SavedVariables`, role state, and cache-like file classes. Addons, fonts, interface assets, and WTF should not share one hidden merge-copy behavior.

## Architecture Decisions

### Decision 1: Core semantics before GUI

Do not start frontend work until bundle apply semantics, addon lock transaction behavior, and Lua rewrite policy are stable enough to expose as a long-lived core API.

### Decision 2: Resource groups need explicit apply policies

Every resource group should have a declared and enforced policy:

- `merge`: write missing and changed files, keep unrelated target files
- `mirror`: make the target group match the source group, deleting stale files in scope
- `replace_selected`: replace only explicitly selected files or directories
- `preserve`: preview only, never mutate

The previous manifest flags such as `replace_addons`, `replace_fonts`, and `merge_wtf` should stay removed in favor of a clearer policy enum. Keeping declared intent that is ignored by execution is not acceptable.

### Decision 3: Plan and execution must separate

The core should split bundle application into three boundaries:

1. `BundleReader`: validates bundle metadata and exposes a normalized archive index
2. `BundlePlanner`: compares source resource groups with a target snapshot and creates a logical plan
3. `BundleExecutor`: stages content, applies rewrites, mutates the target, and rolls back on failure

Temporary extraction is acceptable during planning only if it is treated as an implementation detail and never mutates target state. Rewrite output should be represented as planned transformations first, then materialized by the executor.

### Decision 4: Apply should be transaction-oriented

Bundle apply and addon lock apply should create one operation-level backup checkpoint before mutation, execute the planned operation deterministically, and roll back the whole operation on failure.

Subcommands may still report resource-group progress, but rollback should be reasoned about at the operation level, not as many nested partial backups.

### Decision 5: Lua rewrite must fail closed

Lua migration should move from broad string rewriting toward explicit opt-in rules:

- file include rules
- optional addon-specific rule groups
- previewable replacements
- encoding-aware read/write
- regression fixtures from real `SavedVariables`

Unknown Lua files should default to copy/merge without rewrite unless a rule explicitly includes them.

### Decision 6: Prefer task abstraction before full async conversion

HearthSync should support long-running operations, but the first refactor should not convert the whole core to `async`.

The right near-term boundary is a task model:

- progress events
- cancellation token
- dry-run and execution phases
- operation id
- stable result and error reporting

The CLI can run tasks synchronously. A future frontend can run the same blocking core task on a worker thread and receive progress events. Provider downloads can stay blocking behind this task boundary for now. A later async provider runtime can be introduced behind traits once cancellation, progress, and transaction semantics are stable.

This avoids spreading an async runtime through filesystem-heavy code before the domain model is correct.

## Refactor Sequence

### R1 - Manifest apply policy

- Replace or consume existing boolean apply flags.
- Define default policies per resource group.
- Include policy decisions in plan output and JSON.

### R2 - Bundle planner/executor split

- Introduce target snapshots for `AddOns`, `WTF`, `Fonts`, and interface assets.
- Remove target mutation from plan generation.
- Move rewrite materialization into execution.

### R3 - Resource-group sync semantics

- Implement `merge`, `mirror`, `replace_selected`, and `preserve`.
- Expose `share` as copy-missing-and-preserve-target and `sync` as delete-target-then-copy alias semantics.
- Add delete operations to plans where policies allow stale file cleanup.
- Make dangerous deletes visible in text and JSON previews.

### R4 - Transactional apply

- Use one backup checkpoint per bundle apply.
- Prepare all files before target mutation.
- Roll back the entire operation on failure.
- Apply the same transaction model to addon lock sync.

### R5 - Lua rewrite hardening

- Add file-level rewrite allowlists.
- Add encoding-aware file operations.
- Add representative real-world fixtures.

### R6 - Task model and stable core API for frontend reuse

- Expose pure request/result structs from core.
- Move CLI rendering out of core concerns.
- Add progress and cancellation hooks after the operation model is stable.
- Keep blocking filesystem and archive operations behind a task boundary before introducing an async runtime.
