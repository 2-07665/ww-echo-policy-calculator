# Tauri Backend Maintainer Guide

This document explains the architecture and invariants of the Tauri backend runtime.

## Module Layout

- `main.rs`: thin entrypoint that calls `app::run()`.
- `app.rs`: backend orchestration module; pulls feature sections via `include!`.
- `app/types*.rs`: request/response/session structures.
  - Request types are split into:
    - `types_requests_common.rs`
    - `types_requests_upgrade.rs`
    - `types_requests_reroll_ocr.rs`
    - `types_requests_presets.rs`
  - Response/data types are split into:
    - `types_data_presets.rs`
    - `types_data_upgrade.rs`
    - `types_data_reroll.rs`
    - `types_data_ocr.rs`
- `app/presets*.rs`: scorer preset parsing/normalization/merge utilities.
  - Preset resolution is split into:
    - `presets_resolution_variants.rs`
    - `presets_resolution_groups.rs`
    - `presets_resolution_lookup.rs`
    - `presets_resolution_response.rs`
- `app/scoring*.rs`: scorer construction, mask/weight helpers, OCR parsing helpers.
- `app/commands*.rs`: Tauri command handlers grouped by feature.
  - Preset commands are split into:
    - `commands_presets_shared.rs`
    - `commands_presets_load_save.rs`
    - `commands_presets_delete.rs`
- `app/run.rs`: Tauri builder wiring and invoke handler registration.
- `constants.rs`: scorer IDs, defaults, buff metadata, bundled preset JSON constants.

## Scope

The backend exposes Tauri commands for two independent tabs:

- `强化策略` (upgrade policy)
- `重抽策略` (reroll policy)

Each tab keeps exactly one in-memory solver session:

- `AppState.current_upgrade: Mutex<Option<SolverSession>>`
- `AppState.current_reroll: Mutex<Option<RerollSession>>`

## Command Overview

- `bootstrap`: returns static metadata and default values.
- `preview_upgrade_score`: computes live displayed score/contributions for UI preview.
- `compute_policy`: computes/updates upgrade policy summary.
- `policy_suggestion`: queries current upgrade solver for Continue/Abandon.
- `compute_reroll_policy`: computes/updates reroll policy.
- `query_reroll_recommendation`: queries reroll lock/accept recommendations.

## Scoring Invariants

Always use `echo_policy` scorer helpers:

- UI display score path:
  - `buff_score_display`
  - `echo_score_display`
- Policy query path:
  - `echo_score_internal`

Do not hand-roll score conversion in backend.

## Upgrade Scorer Flow

Upgrade scorer handling is centralized by helpers:

- `parse_scorer_type`
- `build_upgrade_scorer_config_from_inputs`
- `build_upgrade_scorer`
- `resolve_target_scores`
- `build_upgrade_solver`

This keeps `preview_upgrade_score` and `compute_policy` aligned.

## Solver Reuse Rules

### Upgrade tab (`compute_policy`)

The existing `UpgradePolicySolver` is reused when all of these are unchanged:

- scorer config (`UpgradeScorerConfig`)
- `blend_data`
- cost weights
- exp refund ratio

When reused, only target is updated via:

- `UpgradePolicySolver::update_target_score`

When any of the above changes, a new solver is built.

### Reroll tab (`compute_reroll_policy`)

The existing `RerollPolicySolver` is reused when fixed weights are unchanged.
For target-only changes, call:

- `set_target`
- `derive_policy`

If weights change, rebuild solver.

## Session Structures

`SolverSession` stores:

- `solver`
- displayed `target_score`
- `scorer_config` (for reuse comparison)
- `query_scorer` (for `policy_suggestion` internal score queries)
- `blend_data`
- cost weights
- exp refund ratio

`RerollSession` stores:

- `solver`
- fixed weights
- `FixedScorer` for displayed score queries in recommendation API

## Defaults You May Want to Edit

In `constants.rs`:

- default target scores:
  - `DEFAULT_TARGET_SCORE`
  - `DEFAULT_FIXED_TARGET_SCORE`
  - `DEFAULT_MC_BOOST_ASSISTANT_TARGET_SCORE`
  - `DEFAULT_QQ_BOT_TARGET_SCORE`
- scorer defaults:
  - `DEFAULT_LINEAR_*`
  - `DEFAULT_QQ_BOT_*`
  - `DEFAULT_MC_BOOST_ASSISTANT_BUFF_WEIGHTS`
  - `DEFAULT_FIXED_BUFF_WEIGHTS`

## Validation

Before committing backend changes:

Optional one-shot runner: `bash scripts/check-tauri-app.sh`

1. `cargo check -p echo_calculator_app`
2. Verify `compute_policy` reuse behavior manually:
   - same scorer/cost + new target => reuse path
   - change scorer/cost => rebuild path
