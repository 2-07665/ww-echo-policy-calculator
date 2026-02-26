use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::io::ErrorKind;
use std::net::UdpSocket;
use std::path::{Path, PathBuf};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use echo_policy::{
    CostModel, FixedScorer, InternalScorer, LinearScorer, RerollPolicySolver, SCORE_MULTIPLIER,
    UpgradePolicySolver, bits_to_mask, mask_to_bits,
};
use serde::{Deserialize, Serialize};
use tauri::{Emitter, Manager, State};

use crate::constants::*;

include!("app/types.rs");
include!("app/presets.rs");
include!("app/scoring.rs");
include!("app/commands.rs");
include!("app/run.rs");
