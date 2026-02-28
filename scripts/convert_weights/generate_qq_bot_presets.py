#!/usr/bin/env python3

from __future__ import annotations

import json
import sys
from decimal import Decimal, ROUND_DOWN
from pathlib import Path
from typing import Any


ROOT_DIR = Path(__file__).resolve().parents[2]
DEFAULT_INPUT_MAP_ROOT = ROOT_DIR / "tmp/WutheringWavesUID/WutheringWavesUID/utils/map"
DEFAULT_OUTPUT_JSON = ROOT_DIR / "apps/desktop/src-tauri/default-presets/qq_bot.json"
AUTO_CONVERTED_HINT = "该权重由脚本自动转换，请注意鉴别。"

ROVER_NAME_ALIASES = {
    "漂泊者·湮灭": "暗主",
    "漂泊者·衍射": "光主",
    "漂泊者·气动": "风主",
}

ATTRIBUTE_ID_TO_NAME = {
    1: "冷凝",
    2: "热熔",
    3: "导电",
    4: "气动",
    5: "衍射",
    6: "湮灭",
}

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

MAIN_VALUE_TABLE = {
    "攻击": ("0", "100", "150"),
    "攻击%": ("18", "30", "33"),
    "生命": ("2280", "0", "0"),
    "生命%": ("22.8", "30", "33"),
    "防御%": ("18", "38", "41.8"),
    "暴击": ("0", "0", "22"),
    "暴击伤害": ("0", "0", "44"),
    "共鸣效率": ("0", "32", "0"),
    "属性伤害加成": ("0", "30", "0"),
    "治疗效果加成": ("0", "0", "26.4"),
}

MAIN_COST_ORDER = (4, 3, 1)
MAIN_VALUE_INDEX = {1: 0, 3: 1, 4: 2}
FIXED_SECONDARY_MAIN_BY_COST = {
    4: "攻击",
    3: "攻击",
    1: "生命",
}


def load_json(path: Path) -> Any:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def decimal_from_any(value: Any) -> Decimal:
    if isinstance(value, Decimal):
        return value
    if isinstance(value, str):
        cleaned = value.strip().replace("%", "")
        if not cleaned:
            return Decimal("0")
        return Decimal(cleaned)
    if isinstance(value, (int, float)):
        return Decimal(str(value))
    return Decimal("0")


def floor_decimal(value: Decimal, digits: int = 3) -> Decimal:
    quant = Decimal("1") if digits <= 0 else Decimal(f"1e-{digits}")
    return value.quantize(quant, rounding=ROUND_DOWN)


def to_json_number(value: Decimal) -> float:
    return float(format(value.normalize(), "f"))


def normalize_preset_name(name: str) -> str:
    return ROVER_NAME_ALIASES.get(name, name)


def build_attribute_name_by_char_id(map_root: Path) -> dict[int, str]:
    out: dict[int, str] = {}
    detail_char_dir = map_root / "detail_json" / "char"
    if not detail_char_dir.is_dir():
        return out
    for item in detail_char_dir.glob("*.json"):
        try:
            char_id = int(item.stem)
            data = load_json(item)
            attribute_id = int(data.get("attributeId", 0))
            out[char_id] = ATTRIBUTE_ID_TO_NAME.get(attribute_id, "")
        except Exception:
            continue
    return out


def main_label(main_name: str, attribute_name: str) -> str:
    if main_name == "属性伤害加成":
        return attribute_name or main_name
    return main_name.replace("%", "")


def safe_skill_weight_list(raw: Any) -> list[Decimal]:
    if not isinstance(raw, list):
        return [Decimal("0"), Decimal("0"), Decimal("0"), Decimal("0")]
    out = [decimal_from_any(item) for item in raw[:4]]
    while len(out) < 4:
        out.append(Decimal("0"))
    return out


def build_weights(calc_map: dict[str, Any]) -> dict[str, float]:
    sub = calc_map.get("sub_props", {})
    skill_weight = safe_skill_weight_list(calc_map.get("skill_weight"))
    skill_bonus = decimal_from_any(sub.get("技能伤害加成", 0))

    weights_decimal: dict[str, Decimal] = {
        "Crit_Rate": decimal_from_any(sub.get("暴击", 0)),
        "Crit_Damage": decimal_from_any(sub.get("暴击伤害", 0)),
        "Attack": decimal_from_any(sub.get("攻击%", 0)),
        "Defence": decimal_from_any(sub.get("防御%", 0)),
        "HP": decimal_from_any(sub.get("生命%", 0)),
        "Attack_Flat": decimal_from_any(sub.get("攻击", 0)),
        "Defence_Flat": decimal_from_any(sub.get("防御", 0)),
        "HP_Flat": decimal_from_any(sub.get("生命", 0)),
        "ER": decimal_from_any(sub.get("共鸣效率", 0)),
        "Basic_Attack_Damage": skill_bonus * skill_weight[0],
        "Heavy_Attack_Damage": skill_bonus * skill_weight[1],
        "Skill_Damage": skill_bonus * skill_weight[2],
        "Ult_Damage": skill_bonus * skill_weight[3],
    }
    return {buff: to_json_number(weights_decimal[buff]) for buff in BUFF_NAMES}


def build_cost_variants(
    calc_map: dict[str, Any],
    cost: int,
    attribute_name: str,
) -> list[dict[str, Any]]:
    main_props = calc_map.get("main_props", {})
    cost_key = str(cost)
    main_props_cost = main_props.get(cost_key, {})
    if not isinstance(main_props_cost, dict):
        return []

    fixed_main = FIXED_SECONDARY_MAIN_BY_COST[cost]
    if fixed_main not in MAIN_VALUE_TABLE:
        return []
    value_index = MAIN_VALUE_INDEX[cost]
    fixed_weight = decimal_from_any(main_props_cost.get(fixed_main, 0))
    fixed_value = decimal_from_any(MAIN_VALUE_TABLE[fixed_main][value_index])
    fixed_score = fixed_weight * fixed_value

    candidates: list[dict[str, Any]] = []
    for index, (first_main, raw_weight) in enumerate(main_props_cost.items()):
        first_weight = decimal_from_any(raw_weight)
        if first_weight <= 0:
            continue
        if first_main == fixed_main:
            continue
        if first_main not in MAIN_VALUE_TABLE:
            continue
        first_value = decimal_from_any(MAIN_VALUE_TABLE[first_main][value_index])
        total_score = floor_decimal(first_weight * first_value + fixed_score, digits=3)
        candidates.append(
            {
                "order": index,
                "score": total_score,
                "label": main_label(first_main, attribute_name),
            }
        )

    if not candidates:
        return []

    candidates.sort(key=lambda item: (-item["score"], item["order"]))
    merged: list[dict[str, Any]] = []
    current_score = candidates[0]["score"]
    current_labels: list[str] = [candidates[0]["label"]]

    for item in candidates[1:]:
        if item["score"] == current_score:
            current_labels.append(item["label"])
            continue
        merged.append({"score": current_score, "labels": current_labels})
        current_score = item["score"]
        current_labels = [item["label"]]
    merged.append({"score": current_score, "labels": current_labels})

    variants: list[dict[str, Any]] = []
    for item in merged:
        labels: list[str] = []
        seen_labels: set[str] = set()
        for label in item["labels"]:
            if label in seen_labels:
                continue
            labels.append(label)
            seen_labels.add(label)
        variant_name = f"{cost}C {'/'.join(labels)}"
        variants.append(
            {
                "variantName": variant_name,
                "mainBuffScore": to_json_number(item["score"]),
            }
        )

    return variants


def build_variants(calc_map: dict[str, Any], attribute_name: str) -> list[dict[str, Any]]:
    out: list[dict[str, Any]] = []
    for cost in MAIN_COST_ORDER:
        out.extend(build_cost_variants(calc_map, cost, attribute_name))
    return out


def build_presets(map_root: Path) -> dict[str, Any]:
    limit_data = load_json(map_root / "limit.json")
    char_list = limit_data.get("charList", [])
    if not isinstance(char_list, list):
        raise ValueError("Invalid limit.json: charList is not a list")

    attribute_name_by_char_id = build_attribute_name_by_char_id(map_root)
    presets: list[dict[str, Any]] = []
    seen_preset_names: set[str] = set()

    for entry in char_list:
        if not isinstance(entry, dict):
            continue
        raw_name = str(entry.get("name", "")).strip()
        calc_file = str(entry.get("calcFile", "")).strip()
        if not raw_name or not calc_file:
            continue
        preset_name = normalize_preset_name(raw_name)
        if preset_name in seen_preset_names:
            continue

        calc_path = map_root / "character" / raw_name / calc_file
        calc_map = load_json(calc_path)
        weights = build_weights(calc_map)

        char_id = int(entry.get("charId"))
        attribute_name = attribute_name_by_char_id.get(char_id, "")
        variants = build_variants(calc_map, attribute_name)
        if not variants:
            raise ValueError(f"No variants generated for {raw_name} ({char_id}) from {calc_path}")

        base_variant = dict(variants[0])
        base_variant["weights"] = {buff: weights[buff] for buff in BUFF_NAMES}
        base_variant["presetIntro"] = AUTO_CONVERTED_HINT
        final_variants = [base_variant]
        final_variants.extend(variants[1:])

        presets.append(
            {
                "presetName": preset_name,
                "variants": final_variants,
            }
        )
        seen_preset_names.add(preset_name)

    return {"presets": presets}


def main() -> int:
    input_map_root = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else DEFAULT_INPUT_MAP_ROOT
    output_json = Path(sys.argv[2]).resolve() if len(sys.argv) > 2 else DEFAULT_OUTPUT_JSON

    output_obj = build_presets(input_map_root)
    output_json.parent.mkdir(parents=True, exist_ok=True)
    with output_json.open("w", encoding="utf-8") as handle:
        json.dump(output_obj, handle, ensure_ascii=False, indent=2)
        handle.write("\n")

    if output_json.is_relative_to(ROOT_DIR):
        output_target = output_json.relative_to(ROOT_DIR)
    else:
        output_target = output_json
    print(f"Generated {len(output_obj['presets'])} presets -> {output_target}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
