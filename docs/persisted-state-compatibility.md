# Persisted State Compatibility

Date: 2026-05-06

Status: pre-1.0 policy for the CLI technical preview.

This document defines how HearthSync treats files that can outlive one CLI process or move between
machines.

## State Surfaces

| Surface | Default Location | Owner | Shareable | Schema |
| --- | --- | --- | --- | --- |
| Runtime settings | Platform app-data `settings/runtime.toml` | User | No | Unversioned typed settings |
| Addon registry | App-data or sidecar managed addon state | HearthSync | No | `schema_version = 1` |
| Addon lock | App-data, sidecar, or bundle metadata | HearthSync generated | Yes | `schema_version = 1` |
| Addon policy | App-data or sidecar managed addon state | User | Optional | `schema_version = 1` |
| Addon index | User, repository, guild, or maintainer TOML | User/community | Yes | `schema_version = 1` |
| Catalog governance overlay | Repository JSON under `catalog/` | Repository | Yes | `schema_version = 1` |
| Bundle manifest and metadata | Bundle zip | User/community | Yes | Versioned bundle contracts |
| Bundle addon source sidecar | Bundle metadata `sources.toml` | HearthSync generated | Personal migration only | `schema_version = 1` |

The addon registry is managed state, not a portable user-authored contract. The addon lock and addon
index are the product-facing reproducibility contracts.

## Compatibility Rules

HearthSync is pre-1.0, so compatibility can still change, but changes must be deliberate:

- Existing `schema_version = 1` files should remain readable throughout the CLI technical preview
  unless a documented migration replaces them.
- Unsupported future or past `schema_version` values must fail closed before mutating a live WoW
  installation.
- Required field additions, field renames, enum meaning changes, path semantics changes, or source
  reference meaning changes require a schema bump or an explicit migration.
- Optional additive fields may stay on the current schema only when older current-schema readers can
  ignore them without losing safety or reproducibility.
- Running an older HearthSync binary against files written by a newer binary is not guaranteed. If
  a future field carries data that must survive older rewrites, bump the schema instead of relying
  on unknown-field passthrough.
- Runtime settings are intentionally unversioned for now because the file is small, local, and fully
  user-resettable. If settings gain nested provider credentials, account identity, or migration
  sensitive values, add a versioned settings envelope.
- Lockfiles should record exact reproducible state. Shared community indexes should record source
  identities and governance metadata, not transient release snapshots.

## Migration Expectations

When introducing a new persisted-state schema version:

1. Add fixture files for the oldest supported input version and the new canonical output version.
2. Add read-only migration tests that load old fixtures without touching a WoW installation.
3. Add write tests that verify newly written files use the current schema version and canonical
   ordering.
4. Add negative tests for unsupported schema versions and invalid contracts.
5. Document whether the migration is automatic on read, automatic on write, or command-gated.
6. Keep backup behavior ahead of any migration that rewrites user-owned state.

For consumer-facing beta, the release gate is stricter: the project should support migration from
the previous public minor release's persisted state, or explicitly block startup with a clear manual
recovery path.

## Current Test Expectations

The current codebase should keep tests around these boundaries:

- registry, lock, policy, index, governance, and source-sidecar files reject unsupported
  `schema_version` values;
- generated lock and registry state validate portable addon directory names before writes;
- lock apply/plan fails before mutation when the desired lock or sidecar source index is invalid;
- runtime settings reject invalid enum values, invalid paths, and invalid cache policies;
- catalog validation remains read-only and can be run without creating managed addon state.

These tests are the minimum bar before changing any schema-bearing type.
