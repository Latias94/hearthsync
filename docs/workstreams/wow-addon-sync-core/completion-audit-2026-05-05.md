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
| Config export as first-class app flow | `src/core/app/request/config.rs` has `ExportConfigBundleAppRequest`; `src/core/app/response/config/bundle.rs` has `ConfigBundleResult`; `src/core/app/config.rs` reuses the external-package engine internally; `src/core/app/stable.rs` exposes `export_config`; `src/cli/config.rs` calls that app boundary. `src/cli/config/tests.rs::config_cli_exported_bundle_applies_through_bundle_cli` verifies the exported artifact can be inspected, planned, dry-run unpacked, and applied through the first-party bundle CLI. | Covered | This directly supports future egui reuse without making GUI code depend on external-package request shapes. |
| External package/config bundle creation | `src/core/bundle/external_package/create_bundle.rs`, `src/core/app/external_package.rs`, `src/cli/external_package.rs`, `src/cli/config.rs`; docs ADR-093/ADR-095 record the design. `src/core/bundle/external_package/analyze.rs` now auto-detects real `NewBeeBoxCache/modules/*.zip` addon cache packages as `newbeebox_addon`, with regression coverage in `src/core/bundle/tests/external_package/analysis.rs::analyze_external_package_auto_detects_newbeebox_module_cache_addon_with_mixed_separators`. Read-only live inspection also covers real NewBeeBox account and character WTF cache packages. | Covered | Public sharing gate is part of bundle creation rather than only CLI output. NewBeeBox-specific mixed separator support stays scoped to explicit or auto-detected NewBeeBox layouts; generic zip archives still reject backslash paths. |
| AddOns, WTF, Fonts, Interface resources | `src/core/app/config/tests.rs::config_service_plans_and_applies_shareable_package_with_mapping_backup_and_rewrite` verifies AddOns cleanup/write, `WTF/Config.wtf`, root, account, and character SavedVariables, Fonts, and Interface resources. `src/cli/config/tests.rs::config_cli_runs_export_plan_dry_run_and_apply_with_mapping` verifies the same resource groups through the CLI handler boundary. `src/core/bundle/tests/external_package/analysis.rs::analyze_external_package_directory_detects_direct_addons_and_root_savedvariables` covers root-level `WTF/SavedVariables` layouts. | Covered by integration tests | Synthetic fixture plus root-layout regression coverage, now exercised through both the shared app facade and CLI handler path. |
| Preview/plan/apply behavior | Config plan/apply tests, external-package plan/apply tests, exported bundle plan/apply test `stable_app_exports_config_bundle_and_applies_exported_bundle`, CLI handler test `config_cli_runs_export_plan_dry_run_and_apply_with_mapping`, and exported-bundle CLI test `config_cli_exported_bundle_applies_through_bundle_cli`. | Covered | The exported artifact is planned and applied through the first-party bundle path at both service and CLI handler levels, while the direct config CLI path covers inspect, export, plan, dry-run apply, and real apply. |
| Backup and rollback | `src/core/app/config/tests.rs::config_service_rolls_back_shareable_package_apply_when_resource_write_fails`, `src/cli/config/tests.rs::config_cli_apply_rolls_back_when_resource_write_fails`, `src/core/bundle/tests/apply/execution.rs::unpack_bundle_rolls_back_when_apply_fails`, `src/core/backup/tests/restore.rs::restore_backup_rolls_back_to_pre_restore_state_when_apply_fails`, addon lock rollback tests, and config apply backup assertions. | Covered | Rollback evidence now includes config facade apply rollback, CLI handler rollback, bundle, backup restore, addon lock, and addon dependency update paths. |
| Windows/macOS support | `apply_external_package_applies_complex_windows_author_zip_to_macos_target`, `config_service_plans_and_applies_shareable_package_with_mapping_backup_and_rewrite`, and `stable_app_exports_config_bundle_and_applies_exported_bundle` use Windows-source metadata with macOS targets. Path collision tests cover Windows/default-macOS case folding. | Covered for MVP | More real author packages should still be added to broaden compatibility confidence. |
| Lua SavedVariables safe rewrite | `src/core/lua_patch/tests/bytes/{fixtures,boundary,scope,encoding}.rs`, `src/core/lua_patch/tests/text.rs`, and `src/core/lua_patch/testdata/FIXTURES.md` cover scoped profile keys, identity fields, known identity-key containers, UTF-8, invalid UTF-8, Latin-1, controlled reductions from aggregate local shape findings, and initial malformed-Lua fail-closed behavior. Fresh targeted run: `CARGO_BUILD_JOBS=1 cargo nextest run lua_patch -j 1 --no-fail-fast` passed 44/44 after the DBM scalar identity table additions. | Improved but still not complete for broad claims | New fixtures added for `AuraUpdater.lua`, `DBM-*`, `Details_*.lua`, `EventsTracker.lua`, `ExWindCore.lua`, `HandyNotes_*.lua`, `MeetingStone.lua`, `SavedInstances.lua`, `TinyTooltip-Remake.lua`, `WeakAuras.lua`, `WeakAurasArchive.lua`, `WorldQuestTracker.lua`, and `ZygorGuidesViewer.lua`, plus controlled reductions for `DBM-Core`, `HandyNotes_Dragonflight`, `MeetingStone`, `SavedInstances`, and `WorldQuestTracker`. DBM now also has invalid UTF-8 compact identity-key coverage and scalar identity-table coverage for `DBM_UsedProfile`, `DBM_UseDualProfile`, and `DBM_CharSavedRevision`, while unrelated scalar popup/cache keys remain preserved. Malformed tests prove incomplete `profileKeys` tables fail closed and incomplete identity containers do not trigger key rewrites. The manifest records coverage expectations, fail-closed intent, and privacy-preserving provenance notes; broader real-package reductions are still missing. |
| Public sharing/privacy review | `src/core/bundle/external_package/sensitive_wtf.rs`, `src/core/bundle/external_package/analysis.rs`, app DTO projections, CLI shared output, serialization tests, and config/external-package tests now report sensitive WTF files and review-required/advisory public-sharing reasons. | Covered for MVP | This is a review gate, not a data scrubber. Public sharing still requires user/operator review. |
| CLI now, egui later | CLI `config export`, `external-package bundle`, plan/apply renderers, app-owned request/result DTOs, and CLI handler tests for config inspect/export/plan/dry-run/apply/rollback plus exported-bundle inspect/plan/unpack exist. | Covered for architecture | No egui UI is implemented yet, by design. |

## Fresh Verification

- `git status --short --branch` before the audit showed `main...origin/main [ahead 7]`.
- Initial `cargo nextest run lua_patch` hit Windows OS error 1455: page file too small while
  compiling test metadata.
- Retried with reduced concurrency:
  `CARGO_BUILD_JOBS=1 cargo nextest run lua_patch -j 1 --no-fail-fast`
  - Result after the new fixtures, malformed-Lua regressions, invalid UTF-8 DBM compact-key
    fixture, and DBM scalar identity-table coverage: 44 tests run, 44 passed, 730 skipped.
- `CARGO_BUILD_JOBS=1 cargo nextest run config_service_rolls_back_shareable_package_apply_when_resource_write_fails -j 1 --no-fail-fast` passed.
- `CARGO_BUILD_JOBS=1 cargo nextest run config_service_plans_and_applies_shareable_package_with_mapping_backup_and_rewrite config_service_rolls_back_shareable_package_apply_when_resource_write_fails stable_app_exports_config_bundle_and_applies_exported_bundle -j 1 --no-fail-fast` passed after adding root SavedVariables to the config facade fixture.
- `cargo nextest run config_cli -j 1 --no-fail-fast` passed: 3 tests run, 3 passed. The CLI handler slice covers inspect, public export with explicit risk allowance, plan, dry-run apply, real apply, backup creation, Lua identity rewrite, root SavedVariables placement, rollback after a forced resource write failure, and bundle CLI consumption of the exported artifact.
- `cargo nextest run config_cli_exported_bundle_applies_through_bundle_cli -j 1 --no-fail-fast` passed: 1 test run, 1 passed. The test exports a config package, then uses the bundle CLI handler to inspect, plan, dry-run unpack, and apply the exported first-party bundle.
- `cargo nextest run preview_lua_bytes_rewrite_fails_closed_on_malformed_profile_tables preview_lua_bytes_rewrite_scopes_malformed_identity_tables_to_safe_fields -j 1 --no-fail-fast` passed: 2 tests run, 2 passed.
- `cargo nextest run preview_lua_bytes_rewrite_rewrites_invalid_utf8_identity_key_fixture -j 1 --no-fail-fast` passed: 1 test run, 1 passed.
- `cargo nextest run preview_lua_bytes_rewrite_rewrites_invalid_utf8_dbm_scalar_identity_tables -j 1 --no-fail-fast` is covered by the latest `lua_patch` run and passes as part of the 44-test slice.
- `CARGO_BUILD_JOBS=1 cargo nextest run external_package -j 1 --no-fail-fast` passed after adding
  real NewBeeBox module-cache auto-detection coverage: 98 tests run, 98 passed, 675 skipped.
- A read-only live `external-package inspect` run against
  `C:\Program Files\NewBeeBox\NewBeeBoxCache\modules\11225-7685_164-MeetingStone.zip` now succeeds
  with default `Auto` layout. The summary reported `layout=newbeebox_addon`, `entry_count=408`,
  `addons=MeetingStone`, `normalized_files=408`, `warning_count=0`, and normalized
  `MeetingStone\addon_version.txt` to `addons/MeetingStone/addon_version.txt`.
- Read-only live `external-package inspect` runs against NewBeeBox WTF cache packages also succeed
  with default `Auto` layout when supplied synthetic source identity context. A representative
  `wtfserve-*.zip` account package reported `layout=newbeebox_wtf_account`, `entry_count=89`,
  `normalized_files=89`, `warning_count=0`, and `public_sharing.status=review_required`. A
  representative `wtfrole-*.zip` character package reported `layout=newbeebox_wtf_character`,
  `entry_count=59`, `normalized_files=59`, `warning_count=0`,
  `public_sharing.status=review_required`, and one detected source character.
- `cargo fmt --check` passed.
- `CARGO_BUILD_JOBS=1 cargo clippy --all-targets -- -D warnings` passed.
- `CARGO_BUILD_JOBS=1 cargo nextest run -j 1` passed after adding the CLI handler acceptance
  slice, exported-bundle CLI acceptance, malformed-Lua regressions, invalid UTF-8 DBM fixture,
  NewBeeBox module-cache auto-detection regression, and DBM scalar identity-table coverage:
  774 tests run, 774 passed, 0 skipped.
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
- A read-only live `config inspect` run against `E:\Games\World of Warcraft\_retail_` completed
  successfully after the root SavedVariables classifier fix. The inspection saw 59,839 normalized
  entries, 21,452 SavedVariables files, 5 review-required public-sharing reasons, 2 advisory
  reasons, and 0 warnings. Root-level `WTF/SavedVariables/*.lua` files were normalized instead of
  being dropped as unsupported layout noise.

## Remaining Gaps

The goal should not be marked complete yet.

1. The Lua rewrite fixture corpus is stronger, but still not broad enough for broad desktop-facing
   migration claims. The allowlisted families now have both first-shape and controlled-reduction
   samples, but more second-shape coverage and malformed/encoding edge cases are still needed before
   broad generalization.
2. The fixture manifest now records addon family, encoding, supported rewrite shape, fail-closed
   behavior, and privacy-preserving provenance notes. It now includes controlled reductions from
   aggregate local shape findings, but it still does not include full live SavedVariables content or
   provenance-backed replay of the user's private files.
3. The current audit has now performed read-only local structure scans, a read-only live
   `config inspect` pass against `E:\Games\World of Warcraft\_retail_`, and read-only live
   `external-package inspect` passes against representative real NewBeeBox addon, account WTF, and
   character WTF cache zips. It still did not ingest, sanitize, or run the apply pipeline against
   those live private contents. That is acceptable for privacy, but not enough to claim broad
   real-world author-package compatibility.
4. Full-suite verification is now green after the new Lua fixtures, root SavedVariables fix, and
   config-facade rollback test, but green tests still do not cover live-package provenance or broad
   real-world SavedVariables migration safety by themselves.

## Recommended Next Slice

1. Extend `src/core/lua_patch/testdata/FIXTURES.md` with source shape notes from controlled,
   sanitized reductions of full real SavedVariables files.
2. Add more second-shape fixtures for the narrowest remaining areas: `EventsTracker.lua`,
   additional `DBM-*`, `Details_*`, and `HandyNotes_*` variants, plus encoding/pathology variants.
3. Convert one or two read-only local structural findings into sanitized, provenance-recorded
   fixtures without copying private SavedVariables content into the repository.
