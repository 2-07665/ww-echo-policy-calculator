"""Offline regression on the six private, untracked game screenshots.

Run with a development Python containing Pillow; the shipped sidecar needs neither.
"""
import argparse
import json
from pathlib import Path
import subprocess
import tempfile

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parents[1]
EXE = ROOT / "apps/ocr/runtime/echo-ocr.exe"
FULL = [("Ult_Damage", 79), ("Skill_Damage", 64), ("Crit_Damage", 138), ("Crit_Rate", 75), ("Attack", 101)]
EXPECTED = {
    "1-1": [("Basic_Attack_Damage", 64)],
    "1-2": FULL,
    "1-3": [("Crit_Rate", 63), ("Crit_Damage", 162), ("HP", 86), ("Defence_Flat", 60)],
    "2-1": FULL,
    "2-2": [("Ult_Damage", 86), ("Defence_Flat", 60)],
    "2-3": [("Crit_Damage", 150), ("HP_Flat", 430), ("Skill_Damage", 116)],
}


def recognize(path):
    result = subprocess.run([str(EXE), "--image", str(path)], capture_output=True, text=True, encoding="utf-8", timeout=15)
    assert result.returncode in (0, 2), (result.stdout, result.stderr)
    return json.loads(result.stdout)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--captures", type=Path, default=ROOT / "captures/ocr")
    args = parser.parse_args()
    checks = 0
    report = []
    subprocess.run([str(EXE), "--self-test"], check=True, timeout=10, capture_output=True)
    with tempfile.TemporaryDirectory(prefix="echo-ocr-") as temporary:
        path = Path(temporary) / "input.png"
        for name, expected in EXPECTED.items():
            source = Image.open(args.captures / f"{name}.png").convert("RGB")
            assert source.size == (1924, 1126), (name, source.size)
            client = source.crop((2, 45, 1922, 1125))
            for size in [(1920, 1080), (1600, 900), (1280, 720)]:
                client.resize(size, Image.Resampling.LANCZOS).save(path)
                result = recognize(path)
                actual = [(e["buffName"], e["buffValue"]) for e in result["packet"]["buffEntries"]]
                assert result["valid"] and actual == expected, (name, size, result)
                assert result["scene"] == int(name[0])
                checks += 1
                report.append({"image": name, "size": size, "substats": actual})
            # Keep the stat rows but remove all supported page anchors: reject.
            negative = client.copy()
            draw = ImageDraw.Draw(negative)
            draw.rectangle((0, 0, 250, 150), fill="black")
            draw.rectangle((1500, 180, 1650, 230), fill="black")
            draw.rectangle((1500, 970, 1650, 1030), fill="black")
            negative.save(path)
            assert not recognize(path)["valid"], name
            checks += 1
        for size in [(1920, 1080), (1920, 1200)]:
            Image.new("RGB", size).save(path)
            assert not recognize(path)["valid"]
            checks += 1
    print(json.dumps({"passed": checks, "cases": report}, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
