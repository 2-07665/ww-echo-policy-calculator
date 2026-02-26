# UI Maintainer Guide

This document explains `apps/desktop/ui/app.js` structure and update rules.

## Core Principle

Do not duplicate scorer math in JS.

Live score preview is backend-driven via:

- `preview_upgrade_score`

The UI sends current scorer settings + selected buffs, and backend returns:

- per-slot contributions
- main contribution
- total displayed score
- max displayed score

## State Model

`state` holds:

- scorer selection and per-scorer configs
- selected buff slots/values
- displayed contributions/score
- policy/reroll result state
- tab state

Key scorer config map:

- `state.scorerConfigs[linear_default|wuwa_echo_tool|mc_boost_assistant|qq_bot|fixed]`

## Async Update Flow

`computeContributions()` is async and authoritative for displayed score.

Rules:

- Always `await computeContributions()` before `updateSuggestion()` after buff changes.
- Avoid manual score rendering around `computeContributions()`; it already renders:
  - buff slot contribution column
  - total score card
  - compute button state

## Invalidation Rules

`resetPolicyResult()` is called when policy-affecting inputs change:

- scorer type
- scorer params
- weights
- target score
- cost weights / exp refund

This only invalidates result state in UI; solver reuse decision is backend responsibility in `compute_policy`.

## Scorer-Specific UX Rules

- `fixed`:
  - hide linear param block
  - integer weights and target
- `mc_boost_assistant`:
  - lock `mainBuffScore = 0.00`
  - lock `normalizedMaxScore = 120.00`
- `qq_bot`:
  - lock `normalizedMaxScore = 50.00`
- `wuwa_echo_tool`:
  - same scoring behavior as `linear_default`
  - independent hints and preset list

## Defaults You May Want to Edit

At top of `app.js`:

- scorer IDs and per-scorer default targets:
  - `DEFAULT_MC_BOOST_ASSISTANT_TARGET_SCORE`
  - `DEFAULT_QQ_BOT_TARGET_SCORE`
- locked values:
  - `MC_BOOST_ASSISTANT_LOCKED_*`
  - `QQ_BOT_LOCKED_NORMALIZED_MAX_SCORE`

Display names are in `index.html` scorer `<option>` labels.

Intro text is in `renderScorerConfig()` in `app.js`.

## Reroll Tab Note

Reroll is fixed-scorer only in UI flow.
Tab switching temporarily forces scorer to fixed and restores previous scorer on return.

## Validation

Before committing UI changes:

1. `node --check apps/desktop/ui/app.js`
2. Manual smoke test:
   - switch scorer types
   - edit weights/params/target
   - run `开始计算策略`
   - verify suggestion updates after buff edits
   - reroll compute and recommendation paths
