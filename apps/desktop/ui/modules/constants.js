export const PLACEHOLDER_LABEL = '选择词条';
export const TARGET_SCORE_STEP = 0.01;
export const TARGET_SCORE_DIGITS = 2;

export const SCORER_LINEAR_DEFAULT = 'linear_default';
export const SCORER_WUWA_ECHO_TOOL = 'wuwa_echo_tool';
export const SCORER_MC_BOOST_ASSISTANT = 'mc_boost_assistant';
export const SCORER_QQ_BOT = 'qq_bot';
export const SCORER_FIXED = 'fixed';

export const SCORER_TYPES = [
  SCORER_LINEAR_DEFAULT,
  SCORER_WUWA_ECHO_TOOL,
  SCORER_MC_BOOST_ASSISTANT,
  SCORER_QQ_BOT,
  SCORER_FIXED,
];

export const SCORER_PRESET_CUSTOM = '自定义';
export const SCORER_PRESET_VARIANT_DEFAULT = '默认';

export const DEFAULT_MC_BOOST_ASSISTANT_TARGET_SCORE = 95.0;
export const DEFAULT_QQ_BOT_TARGET_SCORE = 35.0;

export const MC_BOOST_ASSISTANT_LOCKED_MAIN_BUFF_SCORE = 0.0;
export const MC_BOOST_ASSISTANT_LOCKED_NORMALIZED_MAX_SCORE = 120.0;
export const QQ_BOT_LOCKED_NORMALIZED_MAX_SCORE = 50.0;

export const OCR_PORT_MIN = 1;
export const OCR_PORT_MAX = 65535;
export const DEFAULT_OCR_UDP_PORT = 39191;

export const OCR_UDP_EVENT_FILL_ENTRIES = 'ocr_udp_fill_entries';
export const OCR_UDP_EVENT_LISTENER_STATUS = 'ocr_udp_listener_status';

export function createScorerTypeMap(initializer) {
  const out = {};
  SCORER_TYPES.forEach((type) => {
    out[type] = initializer(type);
  });
  return out;
}

export function createScorerConfig(type) {
  if (type === SCORER_FIXED) {
    return { weights: {} };
  }
  if (type === SCORER_MC_BOOST_ASSISTANT) {
    return {
      mainBuffScore: MC_BOOST_ASSISTANT_LOCKED_MAIN_BUFF_SCORE,
      normalizedMaxScore: MC_BOOST_ASSISTANT_LOCKED_NORMALIZED_MAX_SCORE,
      weights: {},
    };
  }
  if (type === SCORER_QQ_BOT) {
    return {
      mainBuffScore: 0,
      normalizedMaxScore: QQ_BOT_LOCKED_NORMALIZED_MAX_SCORE,
      weights: {},
    };
  }
  return {
    mainBuffScore: 0,
    normalizedMaxScore: 100,
    weights: {},
  };
}

export function createScorerConfigMap() {
  return createScorerTypeMap((type) => createScorerConfig(type));
}
