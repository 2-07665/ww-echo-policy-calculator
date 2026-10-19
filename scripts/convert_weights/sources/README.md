# XutheringWavesUID weight snapshot

`xwuid.json` is the decoded `TEMPLATES` / `CONDITIONS` data from
[IceHe/okww-xwuid-echo-score](https://github.com/IceHe/okww-xwuid-echo-score/blob/843cd0783af7ba2230b01f910d97de3e93d4e027/echo-score/xwuid_echo_data.py).
The `source` object records the distribution commit, source-file SHA-256, original
resource repository and resource commit. The source project is GPL-3.0; its license
is retained in `LICENSE-XWUID`. This attribution applies to the imported snapshot.

Generate and verify the app presets offline from the repository root:

```sh
python3 scripts/convert_weights/generate_qq_bot_presets.py
python3 scripts/convert_weights/test_xwuid_presets.py
```

The generator also accepts `input_snapshot.json [output_presets.json]` arguments.
For a future refresh, inspect the upstream data file, extract its `_ENCODED` string
literal with `ast.parse` / `ast.literal_eval`, decode with `base64.b85decode` then
`zlib.decompress`, and parse as JSON. Do not execute the downloaded Python module.
Update provenance alongside the decoded data, then regenerate and run the tests.
The pinned snapshot contains 68 templates; four identical Rover pairs are merged,
resulting in 64 distinct templates, 61 character groups and 278 cost/main-stat variants.

All 13 per-unit weights are stored on every generated variant. Skill damage weights
are `sub_props[技能伤害加成] * skill_weight[index]`. Raw main scores include both main
stats at +25, retaining decimal precision. Equal-score main choices are grouped only
within the same cost and modal template. IDs use the upstream character/modal and
main-stat identity, rather than list positions.

This is a weight refresh, not an implementation of upstream score calculation.
The application keeps `qq_bot` internally for saved-preset compatibility and displays
`XutheringWavesUID`. Its scorer normalizes the chosen main stat plus the best five
substats to 50 using solver quantization. Upstream uses its own per-cost `score_max`
and truncates each contribution, so exact displayed scores can differ. The upstream
maximum remains available in each generated variant as `sourceScoreMax`.

# Wuwa Echo Tool weight snapshot

`wuwa_echo_tool.json` records the runtime scoring coefficients from
[WuwaEchoTool at 8f292f5](https://github.com/2-07665/WuwaEchoTool/blob/8f292f50f0df479689a57221dcc25c6b840aefcb/js/base.js),
refreshed on 2026-10-19. Provenance includes the commit, source-file SHA-256 and
model defaults. Mode labels come from that checkout's `js/i18n-dictionaries.js`.
The snapshot contains 64 source roles; four identical Rover pairs are merged,
resulting in 60 character groups and 284 sequence/mode variants.

Refresh and verify from the repository root:

```sh
node scripts/convert_weights/extract_wuwa_echo_tool.cjs /path/to/WuwaEchoTool
python3 scripts/convert_weights/generate_wuwa_echo_tool_presets.py
python3 scripts/convert_weights/test_wuwa_echo_tool_presets.py
```

Review the upstream scoring definitions before extraction. The extractor evaluates
only selected scoring definitions, without page/UI code. Node's VM is not a
security boundary for untrusted code. Subsequent generation uses the checked-in
JSON offline and needs only Python.

Runtime coefficient lookup is required: newer characters use computed models,
mode-specific profiles and explicit damage-type coefficients. Reading only the
literal role arrays or damage shares would miss those weights. Each per-unit
coefficient is multiplied by its maximum substat roll, display-unit scale and 10,
then rounded half up to two decimals. Consecutive equal sequence profiles merge
within a mode; sparse variants inherit from the group's first variant.

The existing linear Wuwa Echo Tool scorer is preserved. This import does not add
upstream full-loadout normalization, total Energy Regen thresholds or live HP
inputs. Models are sampled with upstream defaults: zero extra Energy Regen and
40,000 reference HP.
