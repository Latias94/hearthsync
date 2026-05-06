# Lua Patch Fixture Manifest

This manifest records what each sanitized SavedVariables fixture is expected to prove. Fixtures are
small compatibility slices, not full addon databases.

## Coverage Map

| Fixture | Addon or rule | Encoding | Expected behavior |
| --- | --- | --- | --- |
| `addonskins_realistic_utf8.lua` | `AddOnSkins.lua` via `profileKeys` marker | UTF-8 | Rewrite profile keys and profile tables; preserve addon options. |
| `auctionator_invalid_utf8.lua.escape` | `Auctionator.lua` via `profileKeys` marker | invalid UTF-8 byte fixture | Rewrite profile keys through byte fallback; preserve non-UTF-8 bytes. |
| `auraupdater_identity_container_utf8.lua` | `AuraUpdater.lua` exact identity rule | UTF-8 | Rewrite known `char` identity keys; preserve history/cache keys outside known containers. |
| `baganator_recent_characters_utf8.lua` | `Baganator.lua` fail-closed cache sample | UTF-8 | Do not rewrite recent-character history that only looks like identities. |
| `baganator_recent_characters_reduced_utf8.lua` | `Baganator.lua` controlled local shape reduction | UTF-8 | Do not rewrite reduced recent-character and search-history cache shape that still lacks allowlisted identity markers. |
| `bagsync_realistic_utf8.lua` | `BagSync.lua` exact identity rule | UTF-8 | Rewrite realm/character account maps, `currentrealm`, `totals`, and identity fields. |
| `bigwigs_profilekeys_utf8.lua` | `BigWigs.lua` via `profileKeys` marker | UTF-8 | Rewrite profile keys while preserving descriptive boss notes. |
| `clique_realistic_utf8.lua` | `Clique.lua` exact identity rule plus profile keys | UTF-8 | Rewrite localized profile keys and `char` tables; preserve spell/notes text. |
| `dbm_core_reduced_compact_keys_utf8.lua` | `DBM-*` prefix identity rule, controlled local shape reduction | UTF-8 | Rewrite table-valued compact top-level DBM identity keys; preserve scalar popup/cache keys. |
| `dbm_core_invalid_utf8_compact_keys.lua.escape` | `DBM-*` prefix identity rule, invalid UTF-8 controlled reduction | invalid UTF-8 byte fixture | Rewrite table-valued compact top-level DBM identity keys through byte fallback; preserve scalar popup/cache keys and Latin-1 bytes. |
| `dbm_core_scalar_identity_tables_utf8.lua` | `DBM-*` prefix identity rule, controlled local shape reduction | UTF-8 | Rewrite scalar keys in known DBM character identity tables; preserve unrelated scalar popup/cache keys. |
| `dbm_party_compact_identity_utf8.lua` | `DBM-*` prefix identity rule | UTF-8 | Rewrite compact top-level DBM character keys; preserve warning/template text. |
| `details_realistic_utf8.lua` | `Details.lua` exact identity rule plus profile keys | UTF-8 | Rewrite profile keys and explicit identity fields; preserve localized free text. |
| `details_mythicplus_identity_fields_utf8.lua` | `Details_*` prefix identity rule | UTF-8 | Rewrite explicit run identity fields; preserve run notes and `lastPlayerName`. |
| `details_mythicplus_profiles_compact_utf8.lua` | `Details_*` prefix identity rule plus `profiles` | UTF-8 | Rewrite compact profile keys; preserve compact cache text outside known containers. |
| `details_streamer_profilekeys_utf8.lua` | `Details_*` prefix rule plus profile keys | UTF-8 | Rewrite profile containers; preserve streamer/free-text identity strings. |
| `elvui_realistic_utf8.lua` | `ElvUI.lua` exact identity rule plus profile keys | UTF-8 | Rewrite profile keys, `char`, `faction`, `worldBoss`, compact identity keys, and fields. |
| `eventstracker_character_keys.lua` | `EventsTracker.lua` exact identity rule | UTF-8 | Rewrite exact identity keys without field markers. |
| `eventstracker_value_reduced_utf8.lua` | `EventsTracker.lua` exact identity rule, controlled local shape reduction | UTF-8 | Rewrite identity keys in `value`; preserve history strings outside known containers. |
| `exwindcore_identity_fields_utf8.lua` | `ExWindCore.lua` exact identity rule | UTF-8 | Rewrite explicit `playerName` and `realm` fields; preserve unrelated text and `lastPlayerName`. |
| `handynotes_dragonflight_value_reduced_utf8.lua` | `HandyNotes_*` prefix rule, controlled local shape reduction | UTF-8 | Rewrite identity keys in `value`; preserve map-node owner text outside known containers. |
| `handynotes_travelguide_profilekeys_utf8.lua` | `HandyNotes_*` prefix rule plus profile keys | UTF-8 | Rewrite profile containers; preserve map-note/cache identities outside known containers. |
| `meetingstone_character_reduced_utf8.lua` | `MeetingStone.lua` exact identity rule, controlled local shape reduction | UTF-8 | Rewrite character DB profile keys, profiles, and `searchHistoryList`; preserve activity cache scalars. |
| `meetingstone_character_invalid_utf8_search_history.lua.escape` | `MeetingStone.lua` exact identity rule, invalid UTF-8 controlled reduction | invalid UTF-8 byte fixture | Rewrite character DB profile keys, profiles, and `searchHistoryList` through byte fallback; preserve free text, cache scalars, and Latin-1 bytes. |
| `meetingstone_profilekeys.lua` | `MeetingStone.lua` exact identity rule plus profile keys | UTF-8 | Rewrite profile keys and `searchHistoryList` identity keys. |
| `meetingstone_search_history_context_utf8.lua` | `MeetingStone.lua` exact identity rule | UTF-8 | Rewrite `searchHistoryList` identity keys; preserve activity labels outside known containers. |
| `mrt_profilekeys_reduced_utf8.lua` | `MRT.lua` profile-key marker, controlled local shape reduction | UTF-8 | Rewrite uppercase `ProfileKeys`/`Profiles` and lowercase `profiles` containers; preserve encounter/history identity text outside profile containers. |
| `ndui_bags_realistic_utf8.lua` | `NDui_Bags.lua` via `profileKeys` marker | UTF-8 | Rewrite dense one-line profile keys and preserve author text/options. |
| `newbeebox_realistic_utf8.lua` | `NewBeeBox.lua` exact identity rule | UTF-8 | Rewrite reverse compact identities and name/realm pairs; preserve player GUIDs. |
| `pawn_realistic_latin1.lua.escape` | `Pawn.lua` exact identity rule | Latin-1 byte fixture | Rewrite character SavedVariables while preserving Latin-1-only bytes. |
| `omnicd_profilekeys_char_reduced_utf8.lua` | `OmniCD.lua` profile-key marker, controlled local shape reduction | UTF-8 | Rewrite profile keys and dot-form profile tables; preserve `char` identity keys because OmniCD is not identity allowlisted. |
| `plater_profilekeys_reduced_utf8.lua` | `Plater.lua` profile-key marker, controlled local shape reduction | UTF-8 | Rewrite spaced profile keys and dot-form profile tables; preserve script/cache identity text outside profile containers. |
| `rarity_realistic_utf8.lua` | `Rarity.lua` profile-key-only safety sample | UTF-8 | Rewrite profile keys while preserving account-wide statistics identity fields. |
| `rurutiasuite_realistic_utf8.lua` | `RurutiaSuite.lua` via `profileKeys` marker | UTF-8 | Rewrite profile keys in real author layout and preserve author notes. |
| `savedinstances_reduced_toon_compact_utf8.lua` | `SavedInstances.lua` exact identity rule, controlled local shape reduction | UTF-8 | Rewrite spaced and compact `Toons` keys; preserve historical lockout maps outside `Toons`. |
| `savedinstances_toon_keys.lua` | `SavedInstances.lua` exact identity rule | UTF-8 | Rewrite `Toons` identity keys without generic field markers. |
| `savedinstances_toon_multifield_utf8.lua` | `SavedInstances.lua` exact identity rule | UTF-8 | Rewrite richer `Toons` records; preserve note/history text. |
| `tinytooltip_remake_profilekeys_utf8.lua` | `TinyTooltip-Remake.lua` exact identity rule plus profile keys | UTF-8 | Rewrite profile containers; preserve tooltip cache identities. |
| `tinytooltip_remake_realm_field_utf8.lua` | `TinyTooltip-Remake.lua` exact identity rule | UTF-8 | Rewrite explicit realm fields; preserve free text mentioning the source realm. |
| `weakauras_profilekeys_utf8.lua` | `WeakAuras.lua` exact identity rule plus profile keys | UTF-8 | Rewrite profile containers; preserve display author text. |
| `weakauras_no_identity_utf8.lua` | `WeakAuras.lua` fail-closed no-identity sample | UTF-8 | Do not rewrite WeakAuras payloads without supported identity markers. |
| `weakaurasarchive_identity_keys_utf8.lua` | `WeakAurasArchive.lua` exact identity rule | UTF-8 | Rewrite top-level archive identity keys; preserve nested cache/author strings. |
| `wim_realistic_utf8.lua` | `WIM.lua` fail-closed chat-history sample | UTF-8 | Do not rewrite chat-history account data. |
| `worldquesttracker_reduced_realm_profiles_utf8.lua` | `WorldQuestTracker.lua` exact identity rule, controlled local shape reduction | UTF-8 | Rewrite profile containers and realm fields; preserve nested quest-owner maps. |
| `worldquesttracker_profilekeys_utf8.lua` | `WorldQuestTracker.lua` exact identity rule plus profile keys | UTF-8 | Rewrite profile containers; preserve historical quest text. |
| `zygorguidesviewer_realistic_utf8.lua` | `ZygorGuidesViewer.lua` exact identity rule plus profile keys | UTF-8 | Rewrite guide profile containers; preserve guide notes and cache text. |

## Provenance Notes

- These fixtures are repo-authored sanitized compatibility slices. They are not copied from the
  user's live SavedVariables files.
- The 2026-05-05 read-only local structure scan of `E:\Games\World of Warcraft\_retail_` informed
  fixture priority for common addon families such as `DBM-*`, `MeetingStone.lua`,
  `Details_*.lua`, `ElvUI.lua`, `Baganator.lua`, `Auctionator.lua`, and
  `TinyTooltip-Remake.lua`.
- A follow-up privacy-preserving marker scan also prioritized `Plater.lua` and `OmniCD.lua`
  profile-key shapes. The checked-in fixtures are synthetic reductions of marker/container shape,
  not copies of the live SavedVariables content.
- The same aggregate marker scan found `MRT.lua` uses uppercase `ProfileKeys`/`Profiles` alongside
  lowercase `profiles`. The checked-in MRT fixture keeps only that container-shape signal with
  synthetic account, realm, character, and note values.
- The Baganator fixtures keep only the recent-character and search-history cache shape with
  synthetic character names and search text so the fail-closed behavior stays covered while the
  sanitized reduction grows beyond a single minimal slice.
- The local shape audit in `target/research/savedvariables-shape-audit-2026-05-05.json` records
  counts, encodings, ASCII global assignment names, and marker counts only. It intentionally omits
  paths, account names, character names, and string values.
- Fixtures with `_reduced_` in the name are hand-authored reductions from those aggregate local
  shape findings. They preserve global assignment names, marker/container names, and value-kind
  shape while replacing every account, realm, character, note, and scalar payload with synthetic
  test data.
- Fixture names ending in `.escape` store explicit byte escapes for invalid UTF-8 or Latin-1
  coverage.

## Inline Malformed-Lua Safety Cases

- `src/core/lua_patch/tests/bytes/scope.rs::preview_lua_bytes_rewrite_fails_closed_on_malformed_profile_tables`
  verifies an incomplete `profileKeys` table does not fall back to broad string replacement.
- `src/core/lua_patch/tests/bytes/scope.rs::preview_lua_bytes_rewrite_scopes_malformed_identity_tables_to_safe_fields`
  verifies an incomplete identity-key container does not rewrite keys, while explicit allowlisted
  identity fields can still be rewritten without touching free text.
- `src/core/lua_patch/tests/bytes/scope.rs::preview_lua_bytes_rewrite_fails_closed_on_malformed_known_identity_containers`
  verifies known `searchHistoryList`, `Toons`, and `value` identity containers fail closed when
  their table shape is incomplete.

## Remaining Corpus Gaps

- Fixtures are sanitized slices and controlled reductions. They prove scoped behavior, not full
  real-addon database coverage.
- Some exact or prefix rules still have only one shape. Add second-shape samples before widening any
  identity-key container allowlist or claiming broad migration safety.
- Malformed-Lua and non-UTF-8 coverage now has explicit scoped/fail-closed regression cases and
  byte-fallback reductions for several real-shape families. Add more addon-family reductions before
  claiming broad migration safety for arbitrary desktop users.
