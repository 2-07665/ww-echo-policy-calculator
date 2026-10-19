pub const BUFF_TYPES: [&str; 13] = [
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

pub const BUFF_LABELS: [&str; 13] = [
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

pub const BUFF_TYPE_MAX_VALUES: [f64; 13] = [
    105.0, 210.0, 116.0, 147.0, 116.0, 60.0, 70.0, 580.0, 124.0, 116.0, 116.0, 116.0, 116.0,
];

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

pub const BUFF_VALUE_OPTIONS: [&[u16]; 13] = [
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
