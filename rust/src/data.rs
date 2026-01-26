pub const TOTAL_BUFF_TYPES: usize = 13;
pub const MAX_SELECTED_TYPES: usize = 5;

pub type Histogram = &'static [(u16, u32)];

pub struct BuffTypeData {
    // Percentage stats use a fixed scale (e.g., 6.3% stored as 63 in 0.1% units); flat stats store raw values.
    #[allow(dead_code)]
    pub name: &'static str,
    pub max_value: u16,
    pub histogram: Histogram,
}

// Source: Bilibili @IceHe何瀚清 (https://space.bilibili.com/13378662)
pub const HIST_CRIT_RATE: Histogram = &[
    (63, 1036),
    (69, 964),
    (75, 1053),
    (81, 362),
    (87, 322),
    (93, 328),
    (99, 131),
    (105, 112),
];
pub const HIST_CRIT_DAMAGE: Histogram = &[
    (126, 995),
    (138, 1005),
    (150, 1090),
    (162, 335),
    (174, 362),
    (186, 387),
    (198, 129),
    (210, 119),
];
pub const HIST_ATTACK: Histogram = &[
    (64, 316),
    (71, 373),
    (79, 921),
    (86, 1125),
    (94, 781),
    (101, 707),
    (109, 254),
    (116, 139),
];
pub const HIST_DEFENSE: Histogram = &[
    (81, 318),
    (90, 413),
    (100, 995),
    (109, 1277),
    (118, 872),
    (128, 718),
    (138, 295),
    (147, 152),
];
pub const HIST_HP: Histogram = &[
    (64, 321),
    (71, 386),
    (79, 1005),
    (86, 1213),
    (94, 800),
    (101, 669),
    (109, 275),
    (116, 137),
];
pub const HIST_ATTACK_FLAT: Histogram = &[(30, 326), (40, 2496), (50, 1838), (60, 120)];
pub const HIST_DEFENSE_FLAT: Histogram = &[(40, 700), (50, 2128), (60, 1846), (70, 141)];
pub const HIST_HP_FLAT: Histogram = &[
    (320, 298),
    (360, 419),
    (390, 971),
    (430, 1205),
    (470, 864),
    (510, 680),
    (540, 258),
    (580, 168),
];
pub const HIST_ER: Histogram = &[
    (68, 302),
    (76, 375),
    (84, 975),
    (92, 1199),
    (100, 871),
    (108, 643),
    (116, 274),
    (124, 126),
];
pub const HIST_BASIC_ATTACK_DAMAGE: Histogram = &[
    (64, 316),
    (71, 360),
    (79, 959),
    (86, 1199),
    (94, 859),
    (101, 723),
    (109, 263),
    (116, 160),
];
pub const HIST_HEAVY_ATTACK_DAMAGE: Histogram = &[
    (64, 319),
    (71, 369),
    (79, 968),
    (86, 1187),
    (94, 809),
    (101, 697),
    (109, 283),
    (116, 150),
];
pub const HIST_SKILL_DAMAGE: Histogram = &[
    (64, 328),
    (71, 357),
    (79, 978),
    (86, 1173),
    (94, 847),
    (101, 731),
    (109, 283),
    (116, 149),
];
pub const HIST_ULT_DAMAGE: Histogram = &[
    (64, 292),
    (71, 358),
    (79, 973),
    (86, 1162),
    (94, 823),
    (101, 694),
    (109, 280),
    (116, 144),
];

pub const BUFF_TYPES: [BuffTypeData; TOTAL_BUFF_TYPES] = [
    BuffTypeData {
        name: "Crit. Rate",
        max_value: 105,
        histogram: HIST_CRIT_RATE,
    },
    BuffTypeData {
        name: "Crit. DMG",
        max_value: 210,
        histogram: HIST_CRIT_DAMAGE,
    },
    BuffTypeData {
        name: "ATK%",
        max_value: 116,
        histogram: HIST_ATTACK,
    },
    BuffTypeData {
        name: "DEF%",
        max_value: 147,
        histogram: HIST_DEFENSE,
    },
    BuffTypeData {
        name: "HP%",
        max_value: 116,
        histogram: HIST_HP,
    },
    BuffTypeData {
        name: "ATK",
        max_value: 60,
        histogram: HIST_ATTACK_FLAT,
    },
    BuffTypeData {
        name: "DEF",
        max_value: 70,
        histogram: HIST_DEFENSE_FLAT,
    },
    BuffTypeData {
        name: "HP",
        max_value: 580,
        histogram: HIST_HP_FLAT,
    },
    BuffTypeData {
        name: "Energy Regen",
        max_value: 124,
        histogram: HIST_ER,
    },
    BuffTypeData {
        name: "Basic Attack DMG Bonus",
        max_value: 116,
        histogram: HIST_BASIC_ATTACK_DAMAGE,
    },
    BuffTypeData {
        name: "Heavy Attack DMG Bonus",
        max_value: 116,
        histogram: HIST_HEAVY_ATTACK_DAMAGE,
    },
    BuffTypeData {
        name: "Resonance Skill DMG Bonus",
        max_value: 116,
        histogram: HIST_SKILL_DAMAGE,
    },
    BuffTypeData {
        name: "Resonance Liberation DMG Bonus",
        max_value: 116,
        histogram: HIST_ULT_DAMAGE,
    },
];
