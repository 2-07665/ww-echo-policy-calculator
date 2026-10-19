#!/usr/bin/env python3
"""Convert the pinned, runtime-resolved Wuwa Echo Tool coefficient snapshot."""
from __future__ import annotations
import json
import math
import sys
from pathlib import Path

ROOT_DIR = Path(__file__).resolve().parents[2]
DEFAULT_INPUT = Path(__file__).parent / "sources/wuwa_echo_tool.json"
DEFAULT_OUTPUT = ROOT_DIR / "crates/echo_app/presets/wuwa_echo_tool.json"

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
    "漂泊者·导电（女）": "雷主",
    "漂泊者·导电（男）": "雷主",
}



def build_weights(coefficients):
    if len(coefficients) != len(BUFF_NAMES) or any(not math.isfinite(c) or c < 0 for c in coefficients):
        raise ValueError("Expected 13 finite nonnegative coefficients")
    return {name: math.floor((c * maximum * scale * OUTPUT_WEIGHT_MULTIPLIER + 1e-12) * 100 + .5) / 100
            for name, c, maximum, scale in zip(BUFF_NAMES, coefficients, BUFF_MAX_VALUES, DISPLAY_UNIT_SCALE)}


def generate_presets(snapshot):
    presets = []
    by_name = {}
    for role in snapshot["roles"]:
        name = ROLE_NAME_ALIASES.get(role["name"], role["name"])
        variants = []
        for mode in role["modes"]:
            profiles = mode["profiles"]
            if [p["chain"] for p in profiles] != list(range(7)):
                raise ValueError(f"Missing chain profile: {name}/{mode['mode']}")
            groups = []
            for profile in profiles:
                weights = build_weights(profile["coefficients"])
                if groups and groups[-1]["weights"] == weights:
                    groups[-1]["chains"].append(profile["chain"])
                else:
                    groups.append({"chains": [profile["chain"]], "weights": weights})
            for group in groups:
                chains = group["chains"]
                label = f"{chains[0]}链" if len(chains) == 1 else f"{chains[0]}-{chains[-1]}链"
                if mode["mode"] != "default":
                    label += " · " + snapshot["modes"][mode["mode"]]["zh"]
                weights = group["weights"]
                variant = {"variantName": label, "weights": weights if not variants else {
                    k: v for k, v in weights.items() if v != variants[0]["weights"][k]}}
                if not variants:
                    variant["presetIntro"] = AUTO_CONVERTED_HINT
                variants.append(variant)
        preset = {"presetName": name, "variants": variants}
        if name in by_name:
            if by_name[name] != preset:
                raise ValueError(f"Rover aliases have different weights: {name}")
        else:
            presets.append(preset)
            by_name[name] = preset
    return {"presets": presets}


def main():
    source = Path(sys.argv[1]) if len(sys.argv) > 1 else DEFAULT_INPUT
    output = Path(sys.argv[2]) if len(sys.argv) > 2 else DEFAULT_OUTPUT
    data = generate_presets(json.loads(source.read_text(encoding="utf-8")))
    output.write_text(json.dumps(data, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"Generated {len(data['presets'])} groups / {sum(len(p['variants']) for p in data['presets'])} variants")


if __name__ == "__main__":
    main()
