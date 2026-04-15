# Sync Semantics Hardening Plan

Date: 2026-04-15

## Purpose

This document turns the existing architecture review into an execution plan.
It is part of the current `wow-addon-sync-cli` workstream, not a new top-level workstream.

The goal is to finish the CLI on a stable core contract instead of adding more surface area on top of prototype semantics.

## Why This Is Blocking

The prototype already supports:

- addon install, update, remove, search, index, and lock workflows
- bundle pack, inspect, plan, unpack, addon-plan, and addon-apply
- backups and rollback

The main risk is no longer missing commands.
The main risk is that planning, staging, rewriting, and execution are still too coupled for safe long-term reuse.

If this is not corrected now:

- future CLI additions will deepen the coupling
- GUI work will bind to unstable request and result models
- transaction semantics will remain hard to reason about
- obsolete prototype paths will become harder to remove later

## Refactor Principles

1. keep the CLI working throughout the refactor
2. prefer removing obsolete transition code over preserving every intermediate path
3. separate preview data from execution-only state
4. fail closed for dangerous config rewrites
5. make operation-level rollback the default mental model

## Ordered Refactor Slices

### R1 - Pure bundle preview plan

Status: complete. The public apply plan no longer exposes staged execution paths or rewrite payloads, planning compares archive entry bytes without staged extraction, and operation ordering now follows explicit resource-group ordering instead of archive entry order.

Deliverables:

- a preview-oriented plan type without staged file paths
- planning that reads bundle metadata and target state without mutating target state
- explicit operation ordering independent of zip entry iteration order

Removals expected:

- execution-only fields from the preview plan
- duplicated staging inside planning

### R2 - Execution-only staging boundary

Status: complete. Archive materialization and Lua rewrite staging now happen only during execution, behind the executor boundary.

Deliverables:

- staging moved behind execution
- rewrite materialization only during execution
- clearer `Reader -> Planner -> Executor` contracts
- execution operations no longer wrap the public preview struct as an internal transport object

Removals expected:

- transitional helpers that exist only because planning and execution share one structure

### R3 - Transactional addon lock apply

Status: complete. Addon lock apply now uses one AddOns backup for the full operation and applies metadata-only actions explicitly.

Deliverables:

- one operation-level backup checkpoint for addon lock apply
- deterministic remove/update/install execution inside one transaction boundary
- explicit handling for metadata-only actions

Removals expected:

- nested backup behavior hidden inside multi-step apply flows
- duplicated partial rollback paths

### R4 - Manifest runtime semantics cleanup

Status: complete. `keep_original` ignores target overrides, while `explicit` and `prompt` now require caller-resolved target identities instead of silently falling back to source names.

Deliverables:

- runtime behavior for `keep_original`, `explicit`, and `prompt`
- clearer mapping resolution rules
- removal of dead manifest intent that is parsed but ignored

### R5 - Lua rewrite hardening

Status: partial. Rewrite scope is now limited to explicit account and character `SavedVariables` Lua paths, byte-safe fallback now covers invalid UTF-8 and Latin-1-compatible payloads, rewrite rules now require either explicit content signals or a small known-file rule set, and representative anonymized fixtures derived from local retail `SavedVariables` samples are now checked into tests. Broader encoding-aware handling beyond the current UTF-8 plus byte-safe fallback model is still open.

Deliverables:

- file-level allowlists
- encoding-aware read and write behavior
- representative fixtures from real-world WTF samples

Current evidence:

- checked-in fixtures now cover `MeetingStone.lua`, `EventsTracker.lua`, `SavedInstances.lua`, and an invalid UTF-8 `Auctionator.lua`-style payload
- local retail scans currently justify the byte-safe fallback more strongly than a broader encoding abstraction

Removals expected:

- broad rewrite attempts against every `.lua` file

## Deferred Until After Hardening

The following stay out of the critical path unless directly required by the refactor:

- new GUI work
- remote bundle indexes
- signed bundles
- richer provider ecosystems
- large reporting polish

## Exit Condition

This hardening track is complete when:

- preview planning is execution-independent
- bundle apply and addon lock apply each have one rollback boundary
- manifest runtime intent is materially enforced
- rewrite scope is explicit, fixture-backed, and test-backed
- the resulting core API is simple enough for a future `egui` frontend
