pub const NUM_BUFFS: usize = 13;
pub const NUM_ECHO_SLOTS: usize = 5;

pub type Histogram = &'static [(u16, u32)];

pub struct BuffData {
    #[allow(dead_code)]
    pub name: &'static str,
    pub histogram: Histogram,
}

// Exact roll probabilities published by the game:
// - Crit. Rate / Crit. DMG: 7/30, 7/30, 7/30, 2/25, 2/25, 2/25, 3/100, 3/100.
// - Flat ATK: 7/103, 54/103, 39/103, 3/103.
// - Flat DEF: 15/103, 46/103, 33/103, 9/103.
// - All other stats: 7/103, 8/103, 21/103, 25/103, 18/103, 15/103, 6/103, 3/103.
// Histogram counts are exact integer weights; scoring normalizes each histogram by its sum.
// Percentage stats use a fixed scale (e.g., 6.3% stored as 63 in 0.1% units); flat stats store raw values.
pub const HIST_CRIT_RATE: Histogram = &[
    (63, 70),
    (69, 70),
    (75, 70),
    (81, 24),
    (87, 24),
    (93, 24),
    (99, 9),
    (105, 9),
];
pub const HIST_CRIT_DAMAGE: Histogram = &[
    (126, 70),
    (138, 70),
    (150, 70),
    (162, 24),
    (174, 24),
    (186, 24),
    (198, 9),
    (210, 9),
];
pub const HIST_ATTACK: Histogram = &[
    (64, 7),
    (71, 8),
    (79, 21),
    (86, 25),
    (94, 18),
    (101, 15),
    (109, 6),
    (116, 3),
];
pub const HIST_DEFENSE: Histogram = &[
    (81, 7),
    (90, 8),
    (100, 21),
    (109, 25),
    (118, 18),
    (128, 15),
    (138, 6),
    (147, 3),
];
pub const HIST_HP: Histogram = &[
    (64, 7),
    (71, 8),
    (79, 21),
    (86, 25),
    (94, 18),
    (101, 15),
    (109, 6),
    (116, 3),
];
pub const HIST_ATTACK_FLAT: Histogram = &[(30, 7), (40, 54), (50, 39), (60, 3)];
pub const HIST_DEFENSE_FLAT: Histogram = &[(40, 15), (50, 46), (60, 33), (70, 9)];
pub const HIST_HP_FLAT: Histogram = &[
    (320, 7),
    (360, 8),
    (390, 21),
    (430, 25),
    (470, 18),
    (510, 15),
    (540, 6),
    (580, 3),
];
pub const HIST_ER: Histogram = &[
    (68, 7),
    (76, 8),
    (84, 21),
    (92, 25),
    (100, 18),
    (108, 15),
    (116, 6),
    (124, 3),
];
pub const HIST_BASIC_ATTACK_DAMAGE: Histogram = &[
    (64, 7),
    (71, 8),
    (79, 21),
    (86, 25),
    (94, 18),
    (101, 15),
    (109, 6),
    (116, 3),
];
pub const HIST_HEAVY_ATTACK_DAMAGE: Histogram = &[
    (64, 7),
    (71, 8),
    (79, 21),
    (86, 25),
    (94, 18),
    (101, 15),
    (109, 6),
    (116, 3),
];
pub const HIST_SKILL_DAMAGE: Histogram = &[
    (64, 7),
    (71, 8),
    (79, 21),
    (86, 25),
    (94, 18),
    (101, 15),
    (109, 6),
    (116, 3),
];
pub const HIST_ULT_DAMAGE: Histogram = &[
    (64, 7),
    (71, 8),
    (79, 21),
    (86, 25),
    (94, 18),
    (101, 15),
    (109, 6),
    (116, 3),
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
