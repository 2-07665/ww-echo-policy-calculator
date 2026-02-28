#!/usr/bin/env python3

from __future__ import annotations

import json
import math
import subprocess
import sys
from pathlib import Path
from typing import Any


ROOT_DIR = Path(__file__).resolve().parents[2]
DEFAULT_INPUT_JS = Path(__file__).resolve().parent / "base.js"
DEFAULT_OUTPUT_JSON = ROOT_DIR / "apps/desktop/src-tauri/default-presets/wuwa_echo_tool.json"

BUFF_NAMES = [
    "Crit_Rate",
    "Crit_Damage",
    "Attack",
    "Defence",
    "HP",
    "Attack_Flat",
    "Defence_Flat",
    "HP_Flat",
    "ER",
    "Basic_Attack_Damage",
    "Heavy_Attack_Damage",
    "Skill_Damage",
    "Ult_Damage",
]

BUFF_MAX_VALUES = [105, 210, 116, 147, 116, 60, 70, 580, 124, 116, 116, 116, 116]
DISPLAY_UNIT_SCALE = [0.1, 0.1, 0.1, 0.1, 0.1, 1.0, 1.0, 1.0, 0.1, 0.1, 0.1, 0.1, 0.1]
OUTPUT_WEIGHT_MULTIPLIER = 10.0
AUTO_CONVERTED_HINT = "该权重由脚本自动转换，请注意鉴别。"
ROLE_NAME_ALIASES = {
    "暗主-男": "暗主",
    "暗主-女": "暗主",
    "光主-男": "光主",
    "光主-女": "光主",
    "风主男": "风主",
    "风主女": "风主",
}


def round_two(value: float) -> float:
    scale = 10**2
    return math.floor((value + 1e-12) * scale + 0.5) / scale


def parse_base_js_arrays(input_path: Path) -> tuple[list[dict[str, Any]], list[dict[str, Any]], list[dict[str, Any]]]:
    node_script = r"""
const fs = require('fs');
const vm = require('vm');

function extractArrayLiteral(source, constName) {
  const token = `const ${constName} =`;
  const tokenIndex = source.indexOf(token);
  if (tokenIndex < 0) {
    throw new Error(`Cannot find "${token}"`);
  }
  const arrayStart = source.indexOf('[', tokenIndex);
  if (arrayStart < 0) {
    throw new Error(`Cannot find array start for ${constName}`);
  }
  let depth = 0;
  for (let i = arrayStart; i < source.length; i += 1) {
    const ch = source[i];
    if (ch === '[') depth += 1;
    else if (ch === ']') {
      depth -= 1;
      if (depth === 0) {
        return source.slice(arrayStart, i + 1);
      }
    }
  }
  throw new Error(`Cannot find array end for ${constName}`);
}

const inputPath = process.argv[1];
let source = fs.readFileSync(inputPath, 'utf8');
source = source.replace(/^\uFEFF/, '');
const roleList = vm.runInNewContext(`(${extractArrayLiteral(source, 'roleList')})`);
const ruleList = vm.runInNewContext(`(${extractArrayLiteral(source, 'ruleList')})`);
const roleSumProperty = vm.runInNewContext(`(${extractArrayLiteral(source, 'RoleSumProperty')})`);
process.stdout.write(JSON.stringify({ roleList, ruleList, roleSumProperty }));
"""
    result = subprocess.run(
        ["node", "-e", node_script, str(input_path)],
        check=True,
        capture_output=True,
        text=True,
    )
    parsed = json.loads(result.stdout)
    return parsed["roleList"], parsed["ruleList"], parsed["roleSumProperty"]


def to_float(value: Any, default: float = 0.0) -> float:
    try:
        return float(value)
    except (TypeError, ValueError):
        return default


def build_coefficients(rule: dict[str, Any], role_ratio: dict[str, Any]) -> list[float]:
    unike = to_float(rule.get("unike"))
    return [
        to_float(rule.get("crit")),
        to_float(rule.get("critDamage")),
        to_float(rule.get("attack01")),
        to_float(rule.get("defense01")),
        to_float(rule.get("health01")),
        to_float(rule.get("attack02")),
        to_float(rule.get("defense02")),
        to_float(rule.get("health02")),
        to_float(rule.get("efficiency01")),
        unike * to_float(role_ratio.get("normal")),
        unike * to_float(role_ratio.get("heavy")),
        unike * to_float(role_ratio.get("skill")),
        unike * to_float(role_ratio.get("liberate")),
    ]


def build_weights(rule: dict[str, Any], role_ratio: dict[str, Any]) -> dict[str, float]:
    coefficients = build_coefficients(rule, role_ratio)
    out: dict[str, float] = {}
    for index, buff_name in enumerate(BUFF_NAMES):
        base_value = coefficients[index] * BUFF_MAX_VALUES[index] * DISPLAY_UNIT_SCALE[index]
        out[buff_name] = round_two(base_value * OUTPUT_WEIGHT_MULTIPLIER)
    return out


def weights_equal(left: dict[str, float], right: dict[str, float]) -> bool:
    return all(left.get(name, 0.0) == right.get(name, 0.0) for name in BUFF_NAMES)


def chain_label(chain_numbers: list[int]) -> str:
    if len(chain_numbers) == 1:
        return f"{chain_numbers[0]}链"
    return f"{chain_numbers[0]}-{chain_numbers[-1]}链"


def normalize_preset_name(role_name: str) -> str:
    return ROLE_NAME_ALIASES.get(role_name, role_name)


def build_variant_groups(states: list[dict[str, Any]]) -> list[dict[str, Any]]:
    groups: list[dict[str, Any]] = []
    current_numbers = [states[0]["chain"]]
    current_weights = states[0]["weights"]

    for state in states[1:]:
        if weights_equal(current_weights, state["weights"]):
            current_numbers.append(state["chain"])
            continue
        groups.append({"chains": current_numbers, "weights": current_weights})
        current_numbers = [state["chain"]]
        current_weights = state["weights"]

    groups.append({"chains": current_numbers, "weights": current_weights})
    return groups


def build_preset_for_role(
    role: dict[str, Any],
    role_sum: dict[str, Any] | None,
    rule_list: list[dict[str, Any]],
) -> dict[str, Any]:
    role_name = str(role.get("name", "")).strip()
    if not role_name:
        raise ValueError("Found role with empty name in roleList")
    preset_name = normalize_preset_name(role_name)

    rule_index = int(to_float(role.get("rule"), default=-1))
    if rule_index < 0 or rule_index >= len(rule_list):
        raise ValueError(f"Role '{role_name}' references out-of-range rule index: {rule_index}")

    states: list[dict[str, Any]] = []
    base_weights = build_weights(rule_list[rule_index], role)
    states.append({"chain": 0, "weights": base_weights})

    mz_rules = role_sum.get("mzRule") if isinstance(role_sum, dict) else None
    mz_props = role_sum.get("mzProperty") if isinstance(role_sum, dict) else None
    if isinstance(mz_rules, list):
        for idx, mz_rule in enumerate(mz_rules):
            if not isinstance(mz_rule, dict):
                continue
            chain_number = mz_rule.get("ruleId")
            try:
                chain_number = int(chain_number)
            except (TypeError, ValueError):
                chain_number = idx + 1

            role_ratio = role
            if isinstance(mz_props, list) and idx < len(mz_props) and isinstance(mz_props[idx], dict):
                role_ratio = mz_props[idx]

            states.append({"chain": chain_number, "weights": build_weights(mz_rule, role_ratio)})

    has_mz_rule = isinstance(mz_rules, list) and len(mz_rules) > 0
    grouped = build_variant_groups(states)
    base_group = grouped[0]
    base_weights = base_group["weights"]
    base_name = chain_label(base_group["chains"]) if has_mz_rule else "0-6链"

    variants: list[dict[str, Any]] = [
        {
            "variantName": base_name,
            "weights": {name: base_weights[name] for name in BUFF_NAMES},
            "presetIntro": AUTO_CONVERTED_HINT,
        }
    ]

    for group in grouped[1:]:
        current_weights = group["weights"]
        diff_weights = {
            name: current_weights[name]
            for name in BUFF_NAMES
            if current_weights[name] != base_weights[name]
        }
        variants.append(
            {
                "variantName": chain_label(group["chains"]),
                "weights": diff_weights,
            }
        )

    return {
        "presetName": preset_name,
        "variants": variants,
    }


def generate_presets(
    role_list: list[dict[str, Any]],
    rule_list: list[dict[str, Any]],
    role_sum_property: list[dict[str, Any]],
) -> dict[str, Any]:
    role_sum_by_id: dict[int, dict[str, Any]] = {}
    for item in role_sum_property:
        if not isinstance(item, dict):
            continue
        try:
            role_sum_by_id[int(item.get("id"))] = item
        except (TypeError, ValueError):
            continue

    presets: list[dict[str, Any]] = []
    preset_index_by_name: dict[str, int] = {}
    for role in role_list:
        if not isinstance(role, dict):
            continue
        role_sum = None
        try:
            role_sum = role_sum_by_id.get(int(role.get("id")))
        except (TypeError, ValueError):
            role_sum = None
        built = build_preset_for_role(role, role_sum, rule_list)
        preset_name = built["presetName"]
        existing_index = preset_index_by_name.get(preset_name)
        if existing_index is None:
            preset_index_by_name[preset_name] = len(presets)
            presets.append(built)
            continue

        # Merge duplicate role aliases into one preset name.
        existing = presets[existing_index]
        existing_variants = existing.get("variants", [])
        existing_variant_names = {item.get("variantName") for item in existing_variants}
        for variant in built.get("variants", []):
            variant_name = variant.get("variantName")
            if variant_name in existing_variant_names:
                continue
            existing_variants.append(variant)
            existing_variant_names.add(variant_name)

    return {"presets": presets}


def main() -> int:
    input_path = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else DEFAULT_INPUT_JS
    output_path = Path(sys.argv[2]).resolve() if len(sys.argv) > 2 else DEFAULT_OUTPUT_JSON

    role_list, rule_list, role_sum_property = parse_base_js_arrays(input_path)
    output_obj = generate_presets(role_list, rule_list, role_sum_property)

    output_path.parent.mkdir(parents=True, exist_ok=True)
    with output_path.open("w", encoding="utf-8") as handle:
        json.dump(output_obj, handle, ensure_ascii=False, indent=2)
        handle.write("\n")

    print(
        f"Generated {len(output_obj['presets'])} presets -> "
        f"{output_path.relative_to(ROOT_DIR) if output_path.is_relative_to(ROOT_DIR) else output_path}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
