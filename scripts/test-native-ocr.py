"""Offline regression on the six private, untracked game screenshots.

Run with a development Python containing Pillow; the shipped sidecar needs neither.
"""
import argparse
import json
from pathlib import Path
import subprocess
import tempfile
import time

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
UPGRADE_GROUPS = [
    [("HP", 86), ("Crit_Rate", 63), ("Basic_Attack_Damage", 86), ("Skill_Damage", 101), ("Attack_Flat", 40)],
    [("Heavy_Attack_Damage", 79), ("Attack_Flat", 40), ("Skill_Damage", 86), ("Defence", 100), ("Basic_Attack_Damage", 94)],
]
UPGRADE_EXPECTED = {str(group * 5 + count): entries[:count]
                    for group, entries in enumerate(UPGRADE_GROUPS) for count in range(1, 6)}


def upgrade_client(directory, name):
    source = Image.open(directory / f"{name}.png").convert("RGB")
    assert source.size == (2563, 1486), (name, source.size)
    # These ten supplied window screenshots have a 2560x1440 client at (2,45).
    # This fixture geometry is not used by live capture, which queries the HWND.
    return source.crop((2, 45, 2562, 1485))


def recognize(path):
    result = subprocess.run([str(EXE), "--image", str(path)], capture_output=True, text=True, encoding="utf-8", timeout=15)
    assert result.returncode in (0, 2), (result.stdout, result.stderr)
    scan = json.loads(result.stdout)
    scan["scan_ms"] = int(next(line.split("=", 1)[1] for line in result.stderr.splitlines() if line.startswith("scan_ms=")))
    return scan


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--captures", type=Path, default=ROOT / "captures/ocr")
    parser.add_argument("--upgrade-captures", type=Path, help="Directory containing the private 1.png–10.png upgrade sequence")
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
        if args.upgrade_captures:
            for name, expected in UPGRADE_EXPECTED.items():
                client = upgrade_client(args.upgrade_captures, name)
                for size in [(2560, 1440), (1920, 1080), (1600, 900), (1280, 720), (960, 540)]:
                    client.resize(size, Image.Resampling.LANCZOS).save(path)
                    started = time.perf_counter()
                    result = recognize(path)
                    elapsed = (time.perf_counter() - started) * 1000
                    actual = [(e["buffName"], e["buffValue"]) for e in result["packet"]["buffEntries"]]
                    assert result["valid"] and result["scene"] == 2 and actual == expected, (name, size, result)
                    checks += 1
                    report.append({"upgrade_image": name, "size": size, "substats": actual,
                                   "scan_ms": result["scan_ms"], "process_ms": round(elapsed, 1)})
            # A damaged third row must reject the whole frame, not deliver the
            # old two-row prefix. Duplicate rows must also remain invalid.
            client = upgrade_client(args.upgrade_captures, "3").resize((1920, 1080), Image.Resampling.LANCZOS)
            missing = client.copy()
            ImageDraw.Draw(missing).rectangle((252, 430, 663, 462), fill="black")
            missing.save(path)
            assert not recognize(path)["valid"], "Missing third row emitted a partial list"
            duplicate = client.copy()
            duplicate.paste(client.crop((252, 344, 663, 377)), (252, 387))
            duplicate.save(path)
            assert not recognize(path)["valid"], "Duplicate stat passed fallback recognition"
            checks += 2
    print(json.dumps({"passed": checks, "cases": report}, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
