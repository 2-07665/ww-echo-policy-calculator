#[derive(Clone, Copy)]
enum UpgradeScorerConfig {
    LinearDefault {
        weights: [f64; NUM_BUFFS],
        main_buff_score: f64,
        normalized_max_score: f64,
    },
    WuwaEchoTool {
        weights: [f64; NUM_BUFFS],
        main_buff_score: f64,
        normalized_max_score: f64,
    },
    McBoostAssistant {
        weights: [f64; NUM_BUFFS],
    },
    QQBot {
        qq_bot_weights: [f64; NUM_BUFFS],
        main_buff_score: f64,
        normalized_max_score: f64,
    },
    Fixed {
        weights: [u16; NUM_BUFFS],
    },
}

enum UpgradeScorer {
    Linear(LinearScorer),
    Fixed(FixedScorer),
}

struct SolverSession {
    solver: UpgradePolicySolver,
    target_score: f64,
    scorer_config: UpgradeScorerConfig,
    query_scorer: UpgradeScorer,
    blend_data: bool,
    cost_weights: CostWeightsOutput,
    exp_refund_ratio: f64,
}

struct RerollSession {
    solver: RerollPolicySolver,
    weights: [u16; NUM_BUFFS],
    scorer: FixedScorer,
}

struct OcrUdpListenerSession {
    port: u16,
    stop_flag: Arc<AtomicBool>,
    join_handle: JoinHandle<()>,
}

#[derive(Default)]
struct OcrUdpListenerState {
    session: Option<OcrUdpListenerSession>,
    last_error: Option<String>,
}

struct AppState {
    current_upgrade: Mutex<Option<SolverSession>>,
    current_reroll: Mutex<Option<RerollSession>>,
    ocr_udp_listener: Mutex<OcrUdpListenerState>,
}

impl AppState {
    fn new() -> Self {
        Self {
            current_upgrade: Mutex::new(None),
            current_reroll: Mutex::new(None),
            ocr_udp_listener: Mutex::new(OcrUdpListenerState::default()),
        }
    }
}
