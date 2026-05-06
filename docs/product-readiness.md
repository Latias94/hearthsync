# Product Readiness

Date: 2026-05-06

Status: CLI technical preview is usable for self-managed workflows; public product readiness is not
complete yet.

## Product Position

HearthSync is an open-source, cross-platform World of Warcraft addon and configuration sync tool.
The product should optimize for users who want transparent source attribution, reproducible addon
state, and shareable UI/config packages across Windows and macOS.

It should not try to clone NewBeeBox's user experience. NewBeeBox is useful as a compatibility
sample for real author-package layouts, but HearthSync's product identity is:

- open metadata and source references instead of hidden mirrors;
- multi-provider addon install/update with original upstream attribution;
- reproducible lock/index state for addon sync;
- safe config package inspect/plan/apply flows with backups and rollback;
- app-layer contracts that can be reused by a future `egui` UI.

## Current Product Grade

HearthSync is strong enough for a technical-preview CLI release aimed at developers, power users,
and the project maintainer.

It is not yet strong enough to present as a polished consumer addon manager. The core capabilities
exist, but product onboarding, documentation, live compatibility evidence, network diagnostics, and
release packaging still need work.

## Evidence Map

- Provider source compatibility:
  [Addon Provider Compatibility Matrix](workstreams/wow-addon-provider-ecosystem/provider-compatibility-matrix.md)
- Provider research and source-policy rationale:
  [New Provider Research](workstreams/wow-addon-provider-ecosystem/provider-research.md)
- Community catalog policy and validation:
  [HearthSync Catalog](../catalog/README.md)
- Community catalog contribution workflow:
  [Catalog Contribution Workflow](catalog-contribution.md)
- Scenario-based CLI recipes:
  [User Workflows](user-workflows.md)
- Persisted state and schema compatibility policy:
  [Persisted State Compatibility](persisted-state-compatibility.md)
- Release verification and binary packaging process:
  [Release Process](release.md)
- Config package compatibility hardening:
  [Config Package Compatibility Hardening](workstreams/wow-addon-sync-core/config-package-compatibility-hardening.md)
- Shareable config-package MVP audit:
  [Completion Audit](workstreams/wow-addon-sync-core/completion-audit-2026-05-05.md)

## What Is Ready

| Area | Readiness | Evidence |
| --- | --- | --- |
| Cross-platform path model | Technical preview ready | Runtime path normalization, Windows/macOS installation modeling, sidecar vs app-data addon state. |
| Addon install/update | Technical preview ready | Local archive, HTTP archive, GitHub Releases, CurseForge, Wago, and Tukui source refs with provider-owned materialization. |
| Addon reproducibility | Technical preview ready | Addon registry, addon lock, addon index, bundle addon sidecars, community catalog metadata. |
| Config package sync | Technical preview ready | Config inspect/export/plan/apply over the shared external-package and bundle pipeline. |
| Safety model | Technical preview ready | Dry-run planning, backups, rollback tests, fail-closed Lua rewrite rules, public-sharing risk gates. |
| Search | Early product ready | Built-in community catalog by default, provider-scoped live search when requested, in-process caching with configurable TTL. |
| Future GUI boundary | Directionally ready | Stable and extended app service roots, app-owned DTOs, progress/cancellation contracts. |

## What Is Not Ready

| Gap | Why It Matters | Release Gate |
| --- | --- | --- |
| First-run docs | Users need a safe path from install discovery to first addon/config operation. | Done for technical preview: README quick start covers runtime/inspect, catalog-backed addon install, and config package dry-run. |
| Product command recipes | The CLI has many pieces, but users need opinionated workflows. | First scenario guide exists in `docs/user-workflows.md`; keep expanding it with screenshots or GUI equivalents later. |
| Real author-package matrix | Synthetic tests are good but not enough for broad compatibility claims. | Done for technical preview: config package hardening tracks real-package rows, sanitized Lua reductions, and read-only probe flow. |
| Network/proxy guidance | CN users may hit GitHub/Wago/CurseForge/Tukui connectivity or API quota issues. | Done for technical preview: README documents proxy variables, provider token variables, cache TTL, runtime credential/proxy signals, and common failure symptoms. |
| Provider quota strategy | A local client cannot safely offer unlimited live global search without either local catalog metadata or provider tokens. | Keep default search catalog-backed; document when live provider calls happen and how to configure credentials. |
| Release packaging | Building from source is acceptable for contributors but not consumers. | Publish signed or checksummed binaries for Windows and macOS. |
| Upgrade/migration policy | Addon state, catalog schema, and config bundle schema will evolve. | Done for technical preview: `docs/persisted-state-compatibility.md` defines state surfaces, schema bump rules, and migration test expectations. |
| GUI UX | `egui` should not be started before core workflows and copy are stable enough. | Freeze the first-wave app service contract and product workflows before building screens. |

## Network And Provider Policy

HearthSync should assume network access is unreliable and provider limits are real.

Default behavior should avoid unnecessary upstream requests:

- use the in-tree community catalog for ordinary discovery;
- call live provider search only when a provider is explicitly requested;
- cache successful live provider search results in-process;
- cache downloaded provider artifacts with provider-specific validator behavior where available;
- keep exact release snapshots in lockfiles, not in the shared community catalog.

Provider credentials and operator controls:

- `HEARTHSYNC_CURSEFORGE_API_KEY` is required for CurseForge API access.
- `HEARTHSYNC_GITHUB_TOKEN` or `GITHUB_TOKEN` should be used for heavier GitHub-backed workflows.
- Standard `HTTP_PROXY`, `HTTPS_PROXY`, `ALL_PROXY`, and `NO_PROXY` variables should be documented
  for users behind proxies.
- `--addon-search-cache-ttl-secs 0` disables live provider search caching for debugging.
- A non-zero `--addon-search-cache-ttl-secs` reduces repeated provider calls during a CLI session.

## Catalog Strategy

The community catalog should remain metadata-only and live in this repository for now.

This is the right early-stage tradeoff because it keeps validation, source review, and code changes
in one pull request. A separate catalog repository becomes useful only after the catalog has enough
independent contributors or a release cadence that should diverge from the CLI.

The catalog should store source identities and governance metadata, not transient release tags or
download URLs. Exact versions belong in addon locks or explicit user manifests.

## Public Release Checklist

Before calling the project a consumer-facing beta:

- [x] README includes a short end-to-end quick start.
- [x] Product readiness doc links to all supported workflows.
- [x] Provider compatibility matrix has a recent read-only run.
- [x] Config package compatibility matrix lists real author-package shapes and sanitized Lua
      reductions.
- [x] Community catalog validation script is documented and easy to run.
- [x] Network/proxy/provider-token troubleshooting is documented.
- [ ] `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo nextest run`
      are green on the release machine.
- [ ] Windows and macOS binaries are published with checksums.
- [x] Persisted state/schema compatibility policy is documented.
- [ ] The CLI clearly labels destructive operations and dry-run defaults.

## Next Recommended Work

1. Run the CI and release-artifact workflows on GitHub and fix any hosted-runner-only failures.
2. Add migration fixture tests before introducing the next persisted-state schema version.
3. Keep expanding real author-package and provider matrices as new public packages or failures are
   found.
4. Audit CLI output for destructive operations and make dry-run/backup requirements consistently
   visible.
5. Only after those docs are stable, start the first `egui` screen around runtime diagnostics and
   read-only inspect/search flows.
