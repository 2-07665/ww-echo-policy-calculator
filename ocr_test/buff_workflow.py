from __future__ import annotations

import time
from importlib import resources
from pathlib import Path
import sys

from PIL import Image

SCRIPT_ROOT = Path(__file__).resolve().parents[1]
if str(SCRIPT_ROOT) not in sys.path:
    sys.path.insert(0, str(SCRIPT_ROOT))

from policy_core.ocr.detector import is_upgrade_page
from policy_core.ocr.window_capture import WindowCapture, WindowCaptureConfig, WindowNotFoundError
from policy_core.ocr.workflow import BuffWorkflow, BuffWorkflowResult

WINDOW_TITLE = "鸣潮  "
DETECTION_CROP = (0.00, 0.00, 0.15, 0.10)
OCR_CROP = (0.110, 0.29, 0.37, 0.50)
DETECTION_INTERVAL = 3.0
DETAILED_CAPTURE_INTERVAL = 1.0


def _load_upgrade_logo() -> Image.Image:
    logo_resource = resources.files("policy_core.ocr").joinpath("upgrade_page_logo.png")
    with logo_resource.open("rb") as handle:
        with Image.open(handle) as loaded:
            return loaded.convert("RGB")


def _print_result(result: BuffWorkflowResult) -> None:
    for slot in result.slots:
        print(
            f"[debug] Slot {slot.index + 1}: status={slot.status} "
            f"type={slot.raw_type_text!r} value={slot.raw_value_text!r} cleaned={slot.cleaned_value_text!r}"
        )
        if slot.status == "buff" and slot.type_valid and slot.value_valid:
            print(f" -> {slot.buff_label} ({slot.buff_type}) normalized={slot.normalized_value}")
        elif slot.status in {"pending", "locked"} and slot.unlock_requirement:
            print(f" -> {slot.status} until +{slot.unlock_requirement}")
    payload = result.to_ui_payload()
    print("Buff names:", payload["buff_names"])
    print("Buff values:", payload["buff_values"])


def main() -> None:
    capture = WindowCapture(WindowCaptureConfig(title=WINDOW_TITLE))
    workflow = BuffWorkflow()
    logo_image = _load_upgrade_logo()

    print("Starting OCR watcher. Press Ctrl+C to stop.")
    try:
        while True:
            loop_start = time.time()
            if not capture.window_exists():
                print("Window not found; waiting...")
            else:
                try:
                    detection_frame = capture.grab(crop=DETECTION_CROP)
                    if not is_upgrade_page(detection_frame, logo_image):
                        print("Page mismatch; waiting...")
                    else:
                        if DETAILED_CAPTURE_INTERVAL > 0:
                            time.sleep(DETAILED_CAPTURE_INTERVAL)
                        detailed_frame = capture.grab(crop=OCR_CROP, restore=False)
                        _print_result(workflow.process_image(detailed_frame))
                except WindowNotFoundError:
                    print("Window not found; waiting...")

            elapsed = time.time() - loop_start
            if elapsed < DETECTION_INTERVAL:
                time.sleep(DETECTION_INTERVAL - elapsed)
    except KeyboardInterrupt:
        print("\nWatcher stopped.")


if __name__ == "__main__":
    main()
