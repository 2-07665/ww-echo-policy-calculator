pub const NUM_BUFFS: usize = 13;
pub const NUM_ECHO_SLOTS: usize = 5;

pub type Histogram = &'static [(u16, u32)];

pub struct BuffData {
    #[allow(dead_code)]
    pub name: &'static str,
    pub histogram: Histogram,
}

// Source: Bilibili @IceHe何瀚清 (https://space.bilibili.com/13378662)
// Percentage stats use a fixed scale (e.g., 6.3% stored as 63 in 0.1% units); flat stats store raw values.
pub const HIST_CRIT_RATE: Histogram = &[
    (63, 1310),
    (69, 1231),
    (75, 1329),
    (81, 452),
    (87, 423),
    (93, 438),
    (99, 163),
    (105, 151),
];
pub const HIST_CRIT_DAMAGE: Histogram = &[
    (126, 1261),
    (138, 1293),
    (150, 1368),
    (162, 426),
    (174, 442),
    (186, 477),
    (198, 184),
    (210, 146),
];
pub const HIST_ATTACK: Histogram = &[
    (64, 388),
    (71, 466),
    (79, 1152),
    (86, 1412),
    (94, 977),
    (101, 876),
    (109, 341),
    (116, 175),
];
pub const HIST_DEFENSE: Histogram = &[
    (81, 408),
    (90, 514),
    (100, 1257),
    (109, 1627),
    (118, 1089),
    (128, 928),
    (138, 389),
    (147, 198),
];
pub const HIST_HP: Histogram = &[
    (64, 413),
    (71, 486),
    (79, 1286),
    (86, 1526),
    (94, 1020),
    (101, 866),
    (109, 362),
    (116, 177),
];
pub const HIST_ATTACK_FLAT: Histogram = &[(30, 409), (40, 3125), (50, 2325), (60, 147)];
pub const HIST_DEFENSE_FLAT: Histogram = &[(40, 897), (50, 2661), (60, 2323), (70, 175)];
pub const HIST_HP_FLAT: Histogram = &[
    (320, 386),
    (360, 526),
    (390, 1245),
    (430, 1511),
    (470, 1112),
    (510, 886),
    (540, 326),
    (580, 212),
];
pub const HIST_ER: Histogram = &[
    (68, 379),
    (76, 488),
    (84, 1201),
    (92, 1479),
    (100, 1106),
    (108, 826),
    (116, 358),
    (124, 163),
];
pub const HIST_BASIC_ATTACK_DAMAGE: Histogram = &[
    (64, 401),
    (71, 486),
    (79, 1219),
    (86, 1499),
    (94, 1084),
    (101, 925),
    (109, 328),
    (116, 204),
];
pub const HIST_HEAVY_ATTACK_DAMAGE: Histogram = &[
    (64, 407),
    (71, 479),
    (79, 1256),
    (86, 1493),
    (94, 1047),
    (101, 877),
    (109, 357),
    (116, 188),
];
pub const HIST_SKILL_DAMAGE: Histogram = &[
    (64, 422),
    (71, 451),
    (79, 1234),
    (86, 1473),
    (94, 1069),
    (101, 941),
    (109, 351),
    (116, 182),
];
pub const HIST_ULT_DAMAGE: Histogram = &[
    (64, 387),
    (71, 453),
    (79, 1240),
    (86, 1474),
    (94, 1051),
    (101, 858),
    (109, 359),
    (116, 189),
];

pub const BUFF_MAX_VALUES: [u16; NUM_BUFFS] = [
    105, 210, 116, 147, 116, 60, 70, 580, 124, 116, 116, 116, 116,
];

pub const BUFF_FIXED_VALUE_INDEX: [usize; 3] = [5, 6, 7];

pub static BUFF_TYPES: [BuffData; NUM_BUFFS] = [
    BuffData {
        name: "Crit. Rate",
        histogram: HIST_CRIT_RATE,
    },
    BuffData {
        name: "Crit. DMG",
        histogram: HIST_CRIT_DAMAGE,
    },
    BuffData {
        name: "ATK%",
        histogram: HIST_ATTACK,
    },
    BuffData {
        name: "DEF%",
        histogram: HIST_DEFENSE,
    },
    BuffData {
        name: "HP%",
        histogram: HIST_HP,
    },
    BuffData {
        name: "ATK",
        histogram: HIST_ATTACK_FLAT,
    },
    BuffData {
        name: "DEF",
        histogram: HIST_DEFENSE_FLAT,
    },
    BuffData {
        name: "HP",
        histogram: HIST_HP_FLAT,
    },
    BuffData {
        name: "Energy Regen",
        histogram: HIST_ER,
    },
    BuffData {
        name: "Basic Attack DMG Bonus",
        histogram: HIST_BASIC_ATTACK_DAMAGE,
    },
    BuffData {
        name: "Heavy Attack DMG Bonus",
        histogram: HIST_HEAVY_ATTACK_DAMAGE,
    },
    BuffData {
        name: "Resonance Skill DMG Bonus",
        histogram: HIST_SKILL_DAMAGE,
    },
    BuffData {
        name: "Resonance Liberation DMG Bonus",
        histogram: HIST_ULT_DAMAGE,
    },
];
