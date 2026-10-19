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
