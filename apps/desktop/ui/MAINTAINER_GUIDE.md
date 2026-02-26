# UI Maintainer Guide

This document explains the UI runtime structure and update rules.

## Module Layout

- `app.js`: thin entrypoint that imports runtime modules.
- `modules/app-core.js`: orchestration, state transitions, DOM binding, async flows.
- `modules/ocr-controller.js`: OCR UDP listener UI/event flow and OCR payload application.
- `modules/upgrade-ui-controller.js`: composition layer for upgrade-mode UI subcontrollers.
- `modules/upgrade-scorer-config-view.js`: scorer config panel rendering and scorer-mode hints.
- `modules/upgrade-weight-inputs-view.js`: weight editor rendering and input normalization.
- `modules/upgrade-buff-slots-view.js`: selected buff-slot rendering and slot-change flows.
- `modules/upgrade-label-formatters.js`: buff/value label formatting helpers for upgrade UI.
- `modules/upgrade-score-controller.js`: backend score preview integration for selected buff slots.
- `modules/target-score-controller.js`: target score recommendation/clamping and top-weight aggregation.
- `modules/state-bootstrap.js`: bootstrap payload -> in-memory UI state initialization.
- `modules/mode-flow-controller.js`: scorer/tab mode transitions and related UI flow.
- `modules/scorer-state-controller.js`: scorer-type normalization and scorer-specific state helpers.
- `modules/app-state-factory.js`: initial runtime state factory.
- `modules/dom-cache.js`: centralized DOM element lookup/cache.
- `modules/scorer-config-copy.js`: scorer config copy/clone helpers.
- `modules/scorer-payload-builder.js`: payload weight builders for upgrade/fixed scorer APIs.
- `modules/help-tooltip.js`: shared tooltip helper for contextual hint widgets.
- `modules/app-init-controller.js`: startup/bootstrapping lifecycle orchestration.
- `modules/policy-controller.js`: composition layer for policy view + actions.
- `modules/policy-view.js`: score/result card rendering and suggestion presentation.
- `modules/policy-actions.js`: policy compute, suggestion fetch, and policy-state invalidation.
- `modules/presets-controller.js`: scorer preset CRUD + preset/variant state orchestration.
- `modules/presets-model.js`: preset payload normalization and in-memory list/model helpers.
- `modules/presets-view-controller.js`: preset panel rendering, hints, and status output.
- `modules/presets-mutations.js`: preset save/delete mutation workflows.
- `modules/presets-selection-actions.js`: preset list loading and preset/variant selection apply flows.
- `modules/presets-button-loading.js`: shared loading-state helper for preset action buttons.
- `modules/presets-save-actions.js`: preset save/variant-save mutation paths.
- `modules/presets-delete-actions.js`: preset delete/variant-delete mutation paths.
- `modules/reroll-controller.js`: reroll recommendation state, rendering, and compute flow.
- `modules/reroll-policy-controller.js`: reroll policy/recommendation async flow and invalidation.
- `modules/reroll-view-controller.js`: composition layer for reroll panel subviews.
- `modules/reroll-view-meta.js`: reroll score/accept-summary meta calculations and slot-meta patching.
- `modules/reroll-view-output.js`: reroll recommendation table rendering and choice pick actions.
- `modules/reroll-view-slots.js`: reroll baseline/candidate slot selector rendering.
- `modules/event-handlers-controller.js`: composition layer for UI event-binding groups.
- `modules/event-handlers-ocr-tabs.js`: OCR controls and tab-switch event bindings.
- `modules/event-handlers-upgrade.js`: upgrade-page scorer/preset/parameter event bindings.
- `modules/event-handlers-reroll.js`: reroll-page event bindings.
- `modules/constants.js`: scorer/ocr constants and scorer-map factory helpers.
- `modules/utils.js`: formatting and numeric helpers.
- `modules/tauri-api.js`: Tauri invoke wrapper.

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
  - separate scorer type from `linear_default`
  - independent hints and preset list

Preset controls support `preset -> variants[]`:

- selecting a preset auto-loads its first variant
- additional variants override the first variant
- UI provides separate save/delete actions for presets and variants
- preset dropdown groups presets into `自定义预设` then `内置预设`
- user-defined presets are listed before bundled presets
- in `自定义` mode, variant name input stays editable for creating a new preset
- on bundled preset selection, variant name input also stays editable, but variant save/delete remain disabled
- saving as a new preset carries the current/typed variant name as the base variant
- bundled presets are read-only; users must save with a new preset name to customize

## Defaults You May Want to Edit

At top of `modules/constants.js`:

- locked values:
  - `MC_BOOST_ASSISTANT_LOCKED_*`
  - `QQ_BOT_LOCKED_NORMALIZED_MAX_SCORE`

From backend bootstrap (`main.rs` constants):

- per-scorer default targets:
  - `DEFAULT_MC_BOOST_ASSISTANT_TARGET_SCORE`
  - `DEFAULT_QQ_BOT_TARGET_SCORE`

Display names are in `index.html` scorer `<option>` labels.

Intro text is in `renderScorerConfig()` in `modules/upgrade-scorer-config-view.js`.

## Reroll Tab Note

Reroll is fixed-scorer only in UI flow.
Tab switching temporarily forces scorer to fixed and restores previous scorer on return.

## Validation

Before committing UI changes:

Optional one-shot runner: `bash scripts/check-ui-node.sh`

1. `node --check apps/desktop/ui/app.js`
2. `node --check apps/desktop/ui/modules/app-core.js`
3. `node --check apps/desktop/ui/modules/ocr-controller.js`
4. `node --check apps/desktop/ui/modules/upgrade-ui-controller.js`
5. `node --check apps/desktop/ui/modules/upgrade-scorer-config-view.js`
6. `node --check apps/desktop/ui/modules/upgrade-weight-inputs-view.js`
7. `node --check apps/desktop/ui/modules/upgrade-buff-slots-view.js`
8. `node --check apps/desktop/ui/modules/upgrade-label-formatters.js`
9. `node --check apps/desktop/ui/modules/upgrade-score-controller.js`
10. `node --check apps/desktop/ui/modules/target-score-controller.js`
11. `node --check apps/desktop/ui/modules/state-bootstrap.js`
12. `node --check apps/desktop/ui/modules/policy-controller.js`
13. `node --check apps/desktop/ui/modules/policy-view.js`
14. `node --check apps/desktop/ui/modules/policy-actions.js`
15. `node --check apps/desktop/ui/modules/mode-flow-controller.js`
16. `node --check apps/desktop/ui/modules/scorer-state-controller.js`
17. `node --check apps/desktop/ui/modules/app-state-factory.js`
18. `node --check apps/desktop/ui/modules/dom-cache.js`
19. `node --check apps/desktop/ui/modules/scorer-config-copy.js`
20. `node --check apps/desktop/ui/modules/scorer-payload-builder.js`
21. `node --check apps/desktop/ui/modules/help-tooltip.js`
22. `node --check apps/desktop/ui/modules/app-init-controller.js`
23. `node --check apps/desktop/ui/modules/presets-controller.js`
24. `node --check apps/desktop/ui/modules/presets-model.js`
25. `node --check apps/desktop/ui/modules/presets-view-controller.js`
26. `node --check apps/desktop/ui/modules/presets-mutations.js`
27. `node --check apps/desktop/ui/modules/presets-selection-actions.js`
28. `node --check apps/desktop/ui/modules/presets-button-loading.js`
29. `node --check apps/desktop/ui/modules/presets-save-actions.js`
30. `node --check apps/desktop/ui/modules/presets-delete-actions.js`
31. `node --check apps/desktop/ui/modules/reroll-controller.js`
32. `node --check apps/desktop/ui/modules/reroll-policy-controller.js`
33. `node --check apps/desktop/ui/modules/reroll-view-controller.js`
34. `node --check apps/desktop/ui/modules/reroll-view-meta.js`
35. `node --check apps/desktop/ui/modules/reroll-view-output.js`
36. `node --check apps/desktop/ui/modules/reroll-view-slots.js`
37. `node --check apps/desktop/ui/modules/event-handlers-controller.js`
38. `node --check apps/desktop/ui/modules/event-handlers-ocr-tabs.js`
39. `node --check apps/desktop/ui/modules/event-handlers-upgrade.js`
40. `node --check apps/desktop/ui/modules/event-handlers-reroll.js`
41. `node --check apps/desktop/ui/modules/constants.js`
42. `node --check apps/desktop/ui/modules/utils.js`
43. `node --check apps/desktop/ui/modules/tauri-api.js`
44. Manual smoke test:
   - switch scorer types
   - edit weights/params/target
   - run `开始计算策略`
   - verify suggestion updates after buff edits
   - reroll compute and recommendation paths
