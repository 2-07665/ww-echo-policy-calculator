import { DEFAULT_OCR_UDP_PORT, OCR_PORT_MAX, OCR_PORT_MIN } from './constants.js';

export function escapeHtml(value) {
  return String(value)
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;');
}

export function numberOr(value, fallback = 0) {
  const num = Number(value);
  return Number.isFinite(num) ? num : fallback;
}

export function formatFixedOr(value, digits, fallback = '--') {
  const num = Number(value);
  return Number.isFinite(num) ? num.toFixed(digits) : fallback;
}

export function normalizeOcrPort(value, fallback = DEFAULT_OCR_UDP_PORT) {
  const numeric = Math.round(Number(value));
  if (!Number.isFinite(numeric)) {
    return fallback;
  }
  return Math.max(OCR_PORT_MIN, Math.min(OCR_PORT_MAX, numeric));
}
