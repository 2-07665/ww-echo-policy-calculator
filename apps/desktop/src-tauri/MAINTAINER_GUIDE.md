# Tauri Backend Maintainer Guide

This document explains the architecture and invariants of `apps/desktop/src-tauri/src/main.rs`.

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

In `main.rs` constants:

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

1. `cargo check -p echo_calculator_app`
2. Verify `compute_policy` reuse behavior manually:
   - same scorer/cost + new target => reuse path
   - change scorer/cost => rebuild path
