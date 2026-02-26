pub(crate) const NUM_BUFFS: usize = 13;
pub(crate) const MAX_SELECTED_TYPES: usize = 5;
pub(crate) const DEFAULT_TARGET_SCORE: f64 = 60.0;
pub(crate) const DEFAULT_FIXED_TARGET_SCORE: u16 = 7;
pub(crate) const DEFAULT_MC_BOOST_ASSISTANT_TARGET_SCORE: f64 = 95.0;
pub(crate) const DEFAULT_QQ_BOT_TARGET_SCORE: f64 = 35.0;
pub(crate) const DEFAULT_EXP_REFUND_RATIO: f64 = 0.66;
pub(crate) const DEFAULT_SCORER_TYPE: &str = "linear_default";

pub(crate) const SCORER_TYPE_LINEAR_DEFAULT: &str = "linear_default";
pub(crate) const SCORER_TYPE_WUWA_ECHO_TOOL: &str = "wuwa_echo_tool";
pub(crate) const SCORER_TYPE_MC_BOOST_ASSISTANT: &str = "mc_boost_assistant";
pub(crate) const SCORER_TYPE_QQ_BOT: &str = "qq_bot";
pub(crate) const SCORER_TYPE_FIXED: &str = "fixed";
pub(crate) const SCORER_PRESET_DIR: &str = "scorer-presets";
pub(crate) const SCORER_PRESET_NAME_CUSTOM: &str = "自定义";
pub(crate) const SCORER_PRESET_VARIANT_NAME_DEFAULT: &str = "默认";
pub(crate) const DEFAULT_LINEAR_PRESETS_JSON: &str =
    include_str!("../default-presets/linear_default.json");
pub(crate) const DEFAULT_WUWA_ECHO_TOOL_PRESETS_JSON: &str =
    include_str!("../default-presets/wuwa_echo_tool.json");
pub(crate) const DEFAULT_MC_BOOST_ASSISTANT_PRESETS_JSON: &str =
    include_str!("../default-presets/mc_boost_assistant.json");
pub(crate) const DEFAULT_QQ_BOT_PRESETS_JSON: &str =
    include_str!("../default-presets/qq_bot.json");
pub(crate) const DEFAULT_FIXED_PRESETS_JSON: &str =
    include_str!("../default-presets/fixed.json");

pub(crate) const DEFAULT_LINEAR_MAIN_BUFF_SCORE: f64 = 0.0;
pub(crate) const DEFAULT_LINEAR_NORMALIZED_MAX_SCORE: f64 = 100.0;
pub(crate) const DEFAULT_WUWA_ECHO_TOOL_MAIN_BUFF_SCORE: f64 = 0.0;
pub(crate) const DEFAULT_WUWA_ECHO_TOOL_NORMALIZED_MAX_SCORE: f64 = 100.0;
pub(crate) const DEFAULT_WUWA_ECHO_TOOL_TARGET_SCORE: f64 = 60.0;
pub(crate) const DEFAULT_QQ_BOT_MAIN_BUFF_SCORE: f64 = 0.0;
pub(crate) const DEFAULT_QQ_BOT_NORMALIZED_MAX_SCORE: f64 = 50.0;
pub(crate) const MIN_NORMALIZED_MAX_SCORE: f64 = 0.01;
pub(crate) const DEFAULT_OCR_UDP_PORT: u16 = 9999;
pub(crate) const OCR_UDP_EVENT_FILL_ENTRIES: &str = "ocr_udp_fill_entries";
pub(crate) const OCR_UDP_EVENT_LISTENER_STATUS: &str = "ocr_udp_listener_status";
pub(crate) const OCR_UDP_PACKET_BUFFER_SIZE: usize = 16 * 1024;
pub(crate) const OCR_UDP_READ_TIMEOUT_MS: u64 = 300;

pub(crate) const BUFF_TYPES: [&str; NUM_BUFFS] = [
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
];

pub(crate) const BUFF_LABELS: [&str; NUM_BUFFS] = [
    "暴击",
    "暴击伤害",
    "攻击百分比",
    "防御百分比",
    "生命百分比",
    "攻击",
    "防御",
    "生命",
    "共鸣效率",
    "普攻伤害加成",
    "重击伤害加成",
    "共鸣技能伤害加成",
    "共鸣解放伤害加成",
];

pub(crate) const BUFF_TYPE_MAX_VALUES: [f64; NUM_BUFFS] = [
    105.0, 210.0, 116.0, 147.0, 116.0, 60.0, 70.0, 580.0, 124.0, 116.0, 116.0, 116.0, 116.0,
];

pub(crate) const DEFAULT_LINEAR_BUFF_WEIGHTS: [f64; NUM_BUFFS] = [
    100.0, 100.0, 70.0, 0.0, 0.0, 36.0, 0.0, 0.0, 40.0, 0.0, 0.0, 0.0, 0.0,
];

pub(crate) const DEFAULT_WUWA_ECHO_TOOL_BUFF_WEIGHTS: [f64; NUM_BUFFS] = [
    100.0, 100.0, 70.0, 0.0, 0.0, 36.0, 0.0, 0.0, 40.0, 0.0, 0.0, 0.0, 0.0,
];

pub(crate) const DEFAULT_MC_BOOST_ASSISTANT_BUFF_WEIGHTS: [f64; NUM_BUFFS] = [
    1.0, 1.0, 0.7, 0.0, 0.0, 0.36, 0.0, 0.0, 0.25, 0.0, 0.0, 0.0, 0.0,
];

pub(crate) const DEFAULT_QQ_BOT_BUFF_WEIGHTS: [f64; NUM_BUFFS] = [
    2.0, 1.0, 1.1, 0.0, 0.0, 0.1, 0.0, 0.0, 0.2, 0.0, 0.0, 0.0, 0.0,
];

pub(crate) const DEFAULT_FIXED_BUFF_WEIGHTS: [u16; NUM_BUFFS] =
    [3, 3, 1, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0];

const VALUES_CRIT_RATE: [u16; 8] = [63, 69, 75, 81, 87, 93, 99, 105];
const VALUES_CRIT_DAMAGE: [u16; 8] = [126, 138, 150, 162, 174, 186, 198, 210];
const VALUES_ATTACK: [u16; 8] = [64, 71, 79, 86, 94, 101, 109, 116];
const VALUES_DEFENCE: [u16; 8] = [81, 90, 100, 109, 118, 128, 138, 147];
const VALUES_HP: [u16; 8] = [64, 71, 79, 86, 94, 101, 109, 116];
const VALUES_ATTACK_FLAT: [u16; 4] = [30, 40, 50, 60];
const VALUES_DEFENCE_FLAT: [u16; 4] = [40, 50, 60, 70];
const VALUES_HP_FLAT: [u16; 8] = [320, 360, 390, 430, 470, 510, 540, 580];
const VALUES_ER: [u16; 8] = [68, 76, 84, 92, 100, 108, 116, 124];
const VALUES_BASIC: [u16; 8] = [64, 71, 79, 86, 94, 101, 109, 116];
const VALUES_HEAVY: [u16; 8] = [64, 71, 79, 86, 94, 101, 109, 116];
const VALUES_SKILL: [u16; 8] = [64, 71, 79, 86, 94, 101, 109, 116];
const VALUES_ULT: [u16; 8] = [64, 71, 79, 86, 94, 101, 109, 116];

pub(crate) const BUFF_VALUE_OPTIONS: [&[u16]; NUM_BUFFS] = [
    &VALUES_CRIT_RATE,
    &VALUES_CRIT_DAMAGE,
    &VALUES_ATTACK,
    &VALUES_DEFENCE,
    &VALUES_HP,
    &VALUES_ATTACK_FLAT,
    &VALUES_DEFENCE_FLAT,
    &VALUES_HP_FLAT,
    &VALUES_ER,
    &VALUES_BASIC,
    &VALUES_HEAVY,
    &VALUES_SKILL,
    &VALUES_ULT,
];
