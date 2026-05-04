# Product Focus Architecture Review - 2026-05-04

## Scope

This review re-centers HearthSync on the current product goal:

- open-source addon and configuration sync
- cross-platform Windows/macOS operation
- multi-source addon download and update
- reproducible addon state across machines
- shareable configuration packages with explicit account, realm, and character mapping

NewBeeBox compatibility is treated as an external-package compatibility adapter and research input,
not as a product experience target.

## Summary

The current architecture is directionally correct. The project does not need a big-bang rewrite.

The install/update backend, bundle planner/executor, Lua rewrite engine, app service boundary,
runtime path policy, backup, rollback, and task-progress contracts are already useful foundations.

The main refactor need is narrower:

1. Multi-source addon acquisition needs a focused provider-ecosystem refactor.
2. Configuration sharing needs product hardening around privacy, sensitive WTF data, and naming.
3. CLI-owned input composition should move further into `core::app` before `egui` depends on it.
4. Cross-machine sharing should be documented around `lock.toml` plus source sidecars, not app-data
   registry state.

## Addon Provider Findings

Current strengths:

- The addon mutation backend is mostly source-agnostic after a source becomes a prepared archive.
- `package_prep` already gives a good archive-to-prepared-package boundary.
- The HTTP client and cache modules are testable.
- CurseForge and GitHub have provider-specific validation modules.
- Registry, lock, index, and bundle-addon-lock flows already share the same source identity values.

Current pressure:

- `AddonSourceRef` is a closed enum. Adding Wago, WoWInterface, or more provider families requires
  edits across parsing, materialization, cache namespace, policy pinning, dependency support,
  registry, lock, index, and app DTO projection.
- `AddonProvider` currently mixes parsing, source materialization, search, dependency resolution,
  and cache maintenance.
- `search_addons_impl` is effectively a CurseForge search adapter, not a multi-catalog aggregator.
- Dependency resolution and provider pinning are provider-specific but still leak through generic
  policy plumbing.

Decision:

- Open a focused provider workstream for provider registry and capability refactoring.
- Do not rewrite addon install/update execution, package preparation, registry persistence, or
  lock planning as part of that first slice.

Related workstream:

- `docs/workstreams/wow-addon-provider-ecosystem/`

## Configuration Sharing Findings

Current strengths:

- `bundle` owns the controlled portable archive layout and apply semantics.
- `external_package` normalizes third-party or author packages into the same bundle resource model.
- `config` is a user-facing app/CLI facade over the shared external-package pipeline.
- `lua_patch` remains decoupled from bundle/external-package/config and consumes explicit character
  mappings.
- Account, realm, and character mapping is explicit and conservative.

Current pressure:

- Public sharing is not the same as personal migration. A bundle can currently expose source account
  names in normalized WTF paths and can include broad account-level SavedVariables.
- `config` can actually process addons, fonts, and interface assets through the shared package
  pipeline, so the name is useful for users but not precise as a technical boundary.
- `ResourceApplyPolicy` names such as `Share`, `Sync`, `Mirror`, and `ReplaceSelected` need clearer
  user-facing and app-facing explanations about cleanup scope.
- `WtfCommon` is a broad bucket. API and GUI consumers need the narrower `WtfScope` risk details
  surfaced clearly.

Decision:

- Keep the current bundle/external-package/config execution model.
- Add a product-hardening slice for shareable configuration packages instead of writing a second
  configuration engine.
- Treat NewBeeBox layout support as one optional compatibility layout under external-package import.

Recommended next slices:

1. Add privacy and sharing-mode metadata to manifests or app-facing package results.
2. Add a WTF sensitive-file policy for public sharing.
3. Surface `WtfScope` risk details in config/external-package analysis output.
4. Clarify `ResourceApplyPolicy` docs and UI copy.
5. Consider a future `setup-package` or `ui-package` product term if `config` becomes too narrow.

## App, CLI, and Cross-Platform Findings

Current strengths:

- `StableAppServices` is a reasonable first-wave frontend boundary.
- `ExtendedAppServices` separates addon-index and addon-lock flows from the first stable surface.
- App request/result DTOs are mostly frontend-owned and serializable.
- `AppLiveTask` and `TaskRun<T>` provide a usable progress/cancellation contract for GUI integration.
- Runtime owns provider configuration, host platform, path base, backup output, bundle output, and
  addon-state storage policy.

Current pressure:

- CLI still owns some reusable input composition:
  - mapping file plus CLI override merging
  - manifest file loading and validation for some commands
  - some apply-policy mapping glue
- Path policy is implemented in the right places but spread across install, runtime, archive path,
  addon layout, and bundle planning modules.
- `AppData` addon registry state is local machine state. It should not be presented as the primary
  cross-machine sharing format.

Decision:

- Keep `core::app` as the future `egui` entrypoint.
- Move remaining reusable CLI composition into app request/service helpers.
- Document the recommended sharing contract as lockfile plus source sidecars or provider-backed
  sources.
- Add a GUI task facade later only as a thin layer over the current task-progress contract.

## Refactor Classification

Recommended refactor type:

- fearless, bounded refactor
- not a big-bang rewrite
- no workspace split required right now

Keep stable:

- addon package preparation
- addon mutation execution and rollback
- bundle apply planner/executor
- external-package-to-bundle planning reuse
- Lua patch boundary
- app service roots
- runtime path resolution model

Refactor first:

- provider source registry and capability boundaries
- user-facing sharing/privacy semantics
- CLI-to-app input normalization
- sharing documentation around lock/source sidecars

## Immediate Recommendation

Start with the provider workstream because it directly serves the core product goal of open,
multi-source addon acquisition and has the highest future-change cost if left in enum-and-match
form.

The configuration-sharing hardening can proceed in the existing core workstream without blocking
the provider refactor, because it mainly tightens manifest/app semantics and analysis output rather
than replacing the execution pipeline.
