from __future__ import annotations

import threading
from time import perf_counter
from typing import Iterable, Sequence

import numpy as np
from onnxocr.onnx_paddleocr import ONNXPaddleOcr

from .common import ImageInput, ensure_pil_image

_ENGINE_LOCK = threading.Lock()
_ENGINE: ONNXPaddleOcr | None = None
_LAST_DURATION = 0.0


def _to_ocr_array(image: ImageInput) -> np.ndarray:
    if isinstance(image, np.ndarray):
        array = np.asarray(image)
        if array.ndim == 2:
            array = np.stack([array, array, array], axis=-1)
        if array.ndim != 3 or array.shape[2] < 3:
            raise ValueError("NumPy image input must be HxWxC with at least 3 channels.")
        if array.shape[2] > 3:
            array = array[:, :, :3]
        if array.dtype != np.uint8:
            array = array.astype(np.uint8)
        return np.ascontiguousarray(array)
    pil_image = ensure_pil_image(image)
    array = np.asarray(pil_image, dtype=np.uint8)
    return np.ascontiguousarray(array)


def get_ocr_engine() -> ONNXPaddleOcr:
    """Return a shared ONNXPaddleOcr engine instance."""
    global _ENGINE
    with _ENGINE_LOCK:
        if _ENGINE is None:
            _ENGINE = ONNXPaddleOcr()
        return _ENGINE


def _call_engine(engine: object, image: np.ndarray) -> object:
    if hasattr(engine, "ocr") and callable(getattr(engine, "ocr")):
        return engine.ocr(image)  # type: ignore[no-any-return]
    if callable(engine):
        return engine(image)  # type: ignore[no-any-return]
    raise TypeError("onnxocr engine is not callable and has no .ocr method.")


def _is_listlike(value: object) -> bool:
    return isinstance(value, (list, tuple, np.ndarray))


def _is_number(value: object) -> bool:
    return isinstance(value, (int, float, np.integer, np.floating))


def _normalize_points(points: object) -> list[list[float]]:
    if isinstance(points, np.ndarray):
        points = points.tolist()
    if isinstance(points, (list, tuple)):
        if len(points) == 4 and all(_is_number(p) for p in points):
            x1, y1, x2, y2 = points
            return [
                [float(x1), float(y1)],
                [float(x2), float(y1)],
                [float(x2), float(y2)],
                [float(x1), float(y2)],
            ]
        if len(points) >= 4 and all(isinstance(p, (list, tuple)) and len(p) >= 2 for p in points):
            return [[float(p[0]), float(p[1])] for p in points[:4]]
    return []


def _split_text_score(value: object) -> tuple[str, float]:
    if isinstance(value, (list, tuple)):
        if not value:
            return "", 0.0
        text = value[0]
        score = value[1] if len(value) > 1 else 1.0
        return str(text), float(score) if score is not None else 0.0
    return str(value), 1.0


def _looks_like_detection_entry(item: object) -> bool:
    if not isinstance(item, (list, tuple)) or len(item) < 2:
        return False
    return bool(_normalize_points(item[0]))


def _looks_like_detection_list(raw: object) -> bool:
    if not isinstance(raw, list):
        return False
    if not raw:
        return True
    return all(_looks_like_detection_entry(item) for item in raw if item is not None)


def _iter_detection_entries(raw: object) -> Iterable[tuple[object, str, float]]:
    if raw is None:
        return
    if isinstance(raw, tuple):
        if len(raw) == 2 and _is_listlike(raw[0]) and _is_listlike(raw[1]):
            boxes, texts = raw
            for index, box in enumerate(boxes):
                text_value = texts[index] if index < len(texts) else ""
                text, score = _split_text_score(text_value)
                yield box, text, score
            return
        if len(raw) == 3 and _is_listlike(raw[0]) and _is_listlike(raw[1]):
            boxes, texts, scores = raw
            for index, box in enumerate(boxes):
                text_value = texts[index] if index < len(texts) else ""
                text, score = _split_text_score(text_value)
                if index < len(scores):
                    score = float(scores[index])
                yield box, text, score
            return
    if isinstance(raw, list):
        if len(raw) == 1 and _looks_like_detection_list(raw[0]):
            raw = raw[0]
        if _looks_like_detection_list(raw):
            for item in raw:
                if item is None:
                    continue
                box = item[0]
                text_value = item[1] if len(item) > 1 else ""
                text, score = _split_text_score(text_value)
                if len(item) > 2:
                    score = float(item[2])
                yield box, text, score
            return
        if raw and all(isinstance(item, dict) for item in raw):
            for item in raw:
                box = item.get("box") or item.get("bbox") or item.get("points")
                text_value = item.get("text") or item.get("label") or item.get("value")
                score_value = item.get("score") or item.get("confidence") or 1.0
                if box is None or text_value is None:
                    continue
                text, score = _split_text_score(text_value)
                yield box, text, float(score_value) if score_value is not None else score


def _normalize_ocr_result(raw: object) -> list[list[object]]:
    detections: list[list[object]] = []
    for box, text, score in _iter_detection_entries(raw):
        points = _normalize_points(box)
        if not points:
            continue
        detections.append([points, text, float(score) if score is not None else 0.0])
    return detections


def run_ocr(
    image: ImageInput,
    *,
    engine: object | None = None,
) -> tuple[Sequence[Sequence[object]], tuple[int, int]]:
    global _LAST_DURATION
    ocr_engine = engine or get_ocr_engine()
    array = _to_ocr_array(image)
    start = perf_counter()
    raw = _call_engine(ocr_engine, array)
    _LAST_DURATION = perf_counter() - start
    detections = _normalize_ocr_result(raw)
    height, width = array.shape[:2]
    return detections, (width, height)


def extract_text(
    image: ImageInput,
    *,
    engine: object | None = None,
) -> str:
    raw_result, _ = run_ocr(image, engine=engine)
    lines: list[str] = []
    for entry in raw_result:
        if len(entry) >= 2 and entry[1]:
            lines.append(str(entry[1]))
    return "\n".join(lines)


def measure_last_call_duration() -> float:
    """Return the duration in seconds of the most recent OCR call."""
    return _LAST_DURATION
