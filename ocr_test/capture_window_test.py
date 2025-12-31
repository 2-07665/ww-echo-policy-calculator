from __future__ import annotations

import time
from datetime import datetime
from pathlib import Path
import sys

SCRIPT_ROOT = Path(__file__).resolve().parents[1]
if str(SCRIPT_ROOT) not in sys.path:
    sys.path.insert(0, str(SCRIPT_ROOT))

from policy_core.ocr.window_capture import WindowCapture, WindowCaptureConfig, WindowNotFoundError

WINDOW_TITLE = "鸣潮  "
CROP_REGION: tuple[float, float, float, float] | None = None
CAPTURE_INTERVAL = 1.0
CAPTURE_COUNT = 1
OUTPUT_DIR = Path("captures")
PRE_CAPTURE_DELAY = 0.5


def main() -> None:
    capture = WindowCapture(WindowCaptureConfig(title=WINDOW_TITLE))
    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)

    for index in range(CAPTURE_COUNT):
        try:
            capture.grab(crop=CROP_REGION)
            if PRE_CAPTURE_DELAY > 0:
                time.sleep(PRE_CAPTURE_DELAY)
            frame = capture.grab(crop=CROP_REGION, restore=False)
        except WindowNotFoundError:
            print("Window not found; aborting capture.")
            return

        filename = datetime.now().strftime("screenshot_%Y%m%d_%H%M%S_%f.png")
        output_path = OUTPUT_DIR / filename
        frame.save(output_path)
        print(f"Saved screenshot to: {output_path}")

        if index + 1 < CAPTURE_COUNT:
            time.sleep(max(CAPTURE_INTERVAL, 0.01))


if __name__ == "__main__":
    main()
