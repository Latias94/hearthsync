# Completion Audit - Shareable WoW Config Package MVP

Date: 2026-05-05

Status: not complete yet.

This audit checks the active goal as concrete deliverables rather than treating prior effort or a
green test suite as completion proof.

## Restated Objective

The MVP is complete only when HearthSync can:

1. download and update addons from multiple source families through the open addon model;
2. preserve reproducible addon source state through addon index, addon lock, and bundle sidecars;
3. export a shareable config package through the config app boundary, not a parallel config engine;
4. include addon directories, WTF config, Fonts, and Interface resources in that package;
5. preview, back up, apply, and roll back package application;
6. support Windows and macOS source/target path semantics;
7. safely rewrite Lua SavedVariables identities during mapped apply;
8. surface public-sharing/privacy risk for WTF and SavedVariables data;
9. provide enough integration coverage to justify exposing the CLI flow now and reusing the app
   boundary from a future egui frontend.

## Prompt-To-Artifact Checklist

| Requirement | Concrete evidence inspected | Status | Notes |
| --- | --- | --- | --- |
| Open, multi-source addon download/update | `src/core/addon/provider/source.rs` defines `LocalArchive`, `HttpArchive`, `CurseForgeMod`, `GitHubRelease`, and `WagoAddon`; provider/update tests cover local archives, HTTP, CurseForge, GitHub, Wago, cache, and dependency policies. | Covered for core MVP | Provider depth exists, but live provider credentials/network behavior remains environment-dependent. |
| Addon index and lock reproducibility | `src/core/addon/lock.rs`, `src/core/addon/lock/*`, `src/core/addon/index/*`, bundle sidecar tests in `src/core/app/bundle/tests.rs`, and addon lock apply/verify tests. | Covered for core MVP | Lock/index state is separated from mutable policy state in the current architecture. |
| Config export as first-class app flow | `src/core/app/request/config.rs` has `ExportConfigBundleAppRequest`; `src/core/app/response/config/bundle.rs` has `ConfigBundleResult`; `src/core/app/config.rs` reuses the external-package engine internally; `src/core/app/stable.rs` exposes `export_config`; `src/cli/config.rs` calls that app boundary. | Covered | This directly supports future egui reuse without making GUI code depend on external-package request shapes. |
| External package/config bundle creation | `src/core/bundle/external_package/create_bundle.rs`, `src/core/app/external_package.rs`, `src/cli/external_package.rs`, `src/cli/config.rs`; docs ADR-093/ADR-095 record the design. | Covered | Public sharing gate is part of bundle creation rather than only CLI output. |
| AddOns, WTF, Fonts, Interface resources | `src/core/app/config/tests.rs::config_service_plans_and_applies_shareable_package_with_mapping_backup_and_rewrite` verifies AddOns cleanup/write, `WTF/Config.wtf`, account and character SavedVariables, Fonts, and Interface resources. | Covered by integration test | Synthetic fixture, but it exercises all required resource groups through the shared app facade. |
| Preview/plan/apply behavior | Config plan/apply tests, external-package plan/apply tests, and exported bundle plan/apply test `stable_app_exports_config_bundle_and_applies_exported_bundle`. | Covered | The exported artifact is planned and applied through the first-party bundle path. |
| Backup and rollback | `src/core/bundle/tests/apply/execution.rs::unpack_bundle_rolls_back_when_apply_fails`, `src/core/backup/tests/restore.rs::restore_backup_rolls_back_to_pre_restore_state_when_apply_fails`, addon lock rollback tests, and config apply backup assertions. | Covered | Rollback evidence spans bundle, backup restore, addon lock, and addon dependency update paths. |
| Windows/macOS support | `apply_external_package_applies_complex_windows_author_zip_to_macos_target`, `config_service_plans_and_applies_shareable_package_with_mapping_backup_and_rewrite`, and `stable_app_exports_config_bundle_and_applies_exported_bundle` use Windows-source metadata with macOS targets. Path collision tests cover Windows/default-macOS case folding. | Covered for MVP | More real author packages should still be added to broaden compatibility confidence. |
| Lua SavedVariables safe rewrite | `src/core/lua_patch/tests/bytes/{fixtures,boundary,scope,encoding}.rs`, `src/core/lua_patch/tests/text.rs`, and `src/core/lua_patch/testdata/FIXTURES.md` cover scoped profile keys, identity fields, known identity-key containers, UTF-8, invalid UTF-8, and Latin-1. Fresh targeted run: `CARGO_BUILD_JOBS=1 cargo nextest run lua_patch -j 1` passed 39/39. | Improved but still not complete for broad claims | New fixtures added for `AuraUpdater.lua`, `DBM-*`, `Details_*.lua`, `ExWindCore.lua`, `HandyNotes_*.lua`, `MeetingStone.lua`, `SavedInstances.lua`, `TinyTooltip-Remake.lua`, `WeakAuras.lua`, `WeakAurasArchive.lua`, `WorldQuestTracker.lua`, and `ZygorGuidesViewer.lua`. The manifest records coverage expectations and privacy-preserving provenance notes; broader real-package reductions are still missing. |
| Public sharing/privacy review | `src/core/bundle/external_package/sensitive_wtf.rs`, `src/core/bundle/external_package/analysis.rs`, app DTO projections, CLI shared output, serialization tests, and config/external-package tests now report sensitive WTF files and review-required/advisory public-sharing reasons. | Covered for MVP | This is a review gate, not a data scrubber. Public sharing still requires user/operator review. |
| CLI now, egui later | CLI `config export`, `external-package bundle`, plan/apply renderers, and app-owned request/result DTOs exist. | Covered for architecture | No egui UI is implemented yet, by design. |

## Fresh Verification

- `git status --short --branch` before the audit showed `main...origin/main [ahead 7]`.
- Initial `cargo nextest run lua_patch` hit Windows OS error 1455: page file too small while
  compiling test metadata.
- Retried with reduced concurrency:
  `CARGO_BUILD_JOBS=1 cargo nextest run lua_patch -j 1`
  - Result after the new fixtures: 39 tests run, 39 passed, 725 skipped.
- `cargo fmt --check` passed.
- `CARGO_BUILD_JOBS=1 cargo clippy --all-targets -- -D warnings` passed.
- `CARGO_BUILD_JOBS=1 cargo nextest run -j 1` passed: 764 tests run, 764 passed, 0 skipped.
- Local read-only structure scan found a substantial real install at
  `E:\Games\World of Warcraft\_retail_` without copying SavedVariables contents into the repo:
  90 addon directories, 21,452 SavedVariables Lua files, 23,670 WTF files, 18 font files, and
  36,150 Interface files. The most frequent SavedVariables filenames include `DBM-*`,
  `MeetingStone.lua`, `ElvUI.lua`, `Details*.lua`, `Baganator.lua`, `Auctionator.lua`, and
  `TinyTooltip-Remake.lua`.
- Local read-only structure scan found `C:\Program Files\NewBeeBox\NewBeeBoxCache` with 342 zip
  files and 113 Lua files. This was only a cache/file-type scan, not a behavior clone.
- Privacy-preserving local shape audit wrote
  `target/research/savedvariables-shape-audit-2026-05-05.json`. It records only counts, encodings,
  ASCII global assignment names, known marker counts, and identity-shape counts. It confirmed, for
  example, compact DBM identity keys, `Details_MythicPlus.lua` profile/compact shapes,
  `MeetingStone.lua` profile/search-history shapes, `SavedInstances.lua` `Toons`, TinyTooltip
  realm fields, and WeakAuras files without supported identity markers.

## Remaining Gaps

The goal should not be marked complete yet.

1. The Lua rewrite fixture corpus is stronger, but still not broad enough for broad desktop-facing
   migration claims. The common missing allowlisted families now have first fixture evidence and
   some second-shape samples, but several rules still rely on sanitized slices rather than
   provenance-recorded real-package reductions, especially `EventsTracker.lua`, `DBM-*`,
   `Details_*`, `HandyNotes_*`, `MeetingStone.lua`, and `SavedInstances.lua`.
2. The fixture manifest now records addon family, encoding, supported rewrite shape, fail-closed
   behavior, and privacy-preserving provenance notes. It still does not include controlled
   reductions from full real SavedVariables files.
3. The current audit only performed read-only local structure scans for
   `E:\Games\World of Warcraft` and `C:\Program Files\NewBeeBox`; it did not ingest, sanitize,
   or run the apply pipeline against those live contents. That is acceptable for privacy, but not
   enough to claim broad real-world author-package compatibility.
4. Full-suite verification is now green after the new Lua fixtures, but green tests still do not
   cover live-package provenance or broad real-world SavedVariables migration safety by themselves.

## Recommended Next Slice

1. Extend `src/core/lua_patch/testdata/FIXTURES.md` with source shape notes from controlled,
   sanitized reductions of full real SavedVariables files.
2. Add more second-shape fixtures for the narrowest remaining areas: `EventsTracker.lua`,
   additional `DBM-*`, `Details_*`, and `HandyNotes_*` variants, plus encoding/pathology variants.
3. Convert one or two read-only local structural findings into sanitized, provenance-recorded
   fixtures without copying private SavedVariables content into the repository.
