//! Versioned, validated data independent of solver code. A failed reload keeps the last good pack.
use crate::{AppError, AppResult, metadata::BUFF_TYPES};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

pub const MAX_BYTES: usize = 8 * 1024 * 1024;
const BUILTIN: &str = include_str!("../../../data/echo-data.json");
const SCORERS: [&str; 5] = [
    "linear",
    "fixed",
    "wuwa_echo_tool",
    "mc_boost_assistant",
    "qq_bot",
];
fn invalid() -> AppError {
    AppError {
        code: "invalid_data_pack",
        message: "数据包无效或版本不兼容，已保留原数据。".into(),
    }
}
fn storage_error() -> AppError {
    AppError {
        code: "data_storage",
        message: "无法读取或保存数据包，已保留原数据。".into(),
    }
}
fn text(v: &Value) -> bool {
    v.as_str()
        .is_some_and(|s| !s.trim().is_empty() && s.len() <= 4096)
}
fn nonnegative(v: &Value) -> bool {
    v.as_f64()
        .is_some_and(|x| x.is_finite() && (0.0..=1e9).contains(&x))
}
fn date(s: &str) -> bool {
    if s.is_empty() {
        return true;
    }
    let bytes = s.as_bytes();
    if bytes.len() != 10
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes
            .iter()
            .enumerate()
            .any(|(i, c)| i != 4 && i != 7 && !c.is_ascii_digit())
    {
        return false;
    }
    let year: u32 = s[0..4].parse().unwrap();
    let month: usize = s[5..7].parse().unwrap();
    let day: u32 = s[8..10].parse().unwrap();
    let leap = year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400));
    (1..=12).contains(&month)
        && day > 0
        && day
            <= [
                31,
                if leap { 29 } else { 28 },
                31,
                30,
                31,
                30,
                31,
                31,
                30,
                31,
                30,
                31,
            ][month - 1]
}

#[derive(Debug)]
pub struct DataPack {
    pub value: Value,
    pub revision: String,
}
impl DataPack {
    pub fn parse(bytes: &[u8]) -> AppResult<Self> {
        if bytes.len() > MAX_BYTES {
            return Err(invalid());
        }
        let mut value: Value = serde_json::from_slice(bytes).map_err(|_| invalid())?;
        Self::validate(&value).ok_or_else(invalid)?;
        // Keep revisions identical with or without serde_json/preserve_order (desktop).
        value.sort_all_objects();
        // Canonical JSON avoids treating formatting-only changes as a new pack.
        let revision = format!("{:x}", Sha256::digest(serde_json::to_vec(&value).unwrap()));
        Ok(Self { value, revision })
    }
    fn validate(v: &Value) -> Option<()> {
        if v["schema_version"].as_u64() != Some(1) || !text(&v["version"]) {
            return None;
        }
        let characters = v["characters"].as_object()?;
        let variants = v["variant_names"].as_object()?;
        let descriptions = v["descriptions"].as_object()?;
        for (key, c) in characters {
            if key.is_empty() || !text(&c["en"]) || !date(c["release_date"].as_str()?) {
                return None;
            }
        }
        for (key, val) in variants.iter().chain(descriptions.iter()) {
            if key.is_empty() || !text(val) {
                return None;
            }
        }
        let sources = v["presets"].as_object()?;
        if sources.len() != SCORERS.len() {
            return None;
        }
        let mut count = 0;
        for scoring in SCORERS {
            let mut names = HashSet::new();
            let mut ids = HashSet::new();
            for group in sources.get(scoring)?["presets"].as_array()? {
                if group
                    .as_object()?
                    .keys()
                    .any(|key| !["presetName", "variants"].contains(&key.as_str()))
                {
                    return None;
                }
                let name = group["presetName"].as_str()?;
                if !characters.contains_key(name) || !names.insert(name) {
                    return None;
                }
                let entries = group["variants"].as_array()?;
                if entries.is_empty() {
                    return None;
                }
                let mut labels = HashSet::new();
                let base = entries[0]["weights"].as_object()?;
                if base.len() != 13 || !BUFF_TYPES.iter().all(|k| base.contains_key(*k)) {
                    return None;
                }
                for entry in entries {
                    count += 1;
                    if count > 10000 {
                        return None;
                    }
                    let id = entry["id"].as_str()?;
                    let name = entry["variantName"].as_str()?;
                    if id.is_empty()
                        || id.len() > 256
                        || !ids.insert(id)
                        || !variants.contains_key(name)
                        || !labels.insert(name)
                    {
                        return None;
                    }
                    for (key, w) in entry["weights"].as_object()? {
                        if !BUFF_TYPES.contains(&key.as_str())
                            || !nonnegative(w)
                            || (scoring == "fixed" && w.as_f64()?.fract() != 0.0)
                        {
                            return None;
                        }
                    }
                    if let Some(intro) = entry.get("presetIntro").filter(|x| !x.is_null())
                        && !descriptions.contains_key(intro.as_str()?)
                    {
                        return None;
                    }
                    for key in ["mainBuffScore", "normalizedMaxScore", "target"] {
                        if let Some(number) = entry.get(key).filter(|x| !x.is_null())
                            && (!nonnegative(number)
                                || (key == "normalizedMaxScore" && number.as_f64()? == 0.0))
                        {
                            return None;
                        }
                    }
                }
            }
        }
        if count == 0 {
            return None;
        }
        Some(())
    }
    pub fn from_file(path: &Path) -> AppResult<Self> {
        Self::parse(&read(path)?)
    }
    pub fn builtin() -> Self {
        Self::parse(BUILTIN.as_bytes()).expect("valid bundled data pack")
    }
}

fn read(path: &Path) -> AppResult<Vec<u8>> {
    let mut bytes = Vec::new();
    File::open(path)
        .map_err(|_| storage_error())?
        .take((MAX_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| storage_error())?;
    if bytes.len() > MAX_BYTES {
        return Err(invalid());
    }
    Ok(bytes)
}
struct State {
    pack: Arc<DataPack>,
    checked: Option<Instant>,
    warning: Option<String>,
    external: bool,
}
pub struct DataStore {
    path: Option<PathBuf>,
    missing_ok: bool,
    state: Mutex<State>,
}
impl DataStore {
    pub fn new(path: Option<PathBuf>) -> Self {
        Self {
            path,
            missing_ok: false,
            state: Mutex::new(State {
                pack: Arc::new(DataPack::builtin()),
                checked: None,
                warning: None,
                external: false,
            }),
        }
    }
    pub fn cached(path: PathBuf) -> Self {
        let mut store = Self::new(Some(path));
        store.missing_ok = true;
        store
    }
    /// Server reloads at most once a second, with all readers seeing a whole validated pack.
    pub fn snapshot(&self) -> (Arc<DataPack>, Value) {
        let mut state = self.state.lock().unwrap();
        if let Some(path) = &self.path
            && state
                .checked
                .is_none_or(|t| t.elapsed() >= Duration::from_secs(1))
        {
            state.checked = Some(Instant::now());
            match read(path).and_then(|b| DataPack::parse(&b)) {
                Ok(pack) => {
                    state.pack = Arc::new(pack);
                    state.warning = None;
                    state.external = true;
                }
                Err(error) => {
                    if !(self.missing_ok && !state.external && !path.exists()) {
                        state.warning = Some(error.message);
                    }
                }
            }
        }
        let status = json!({"source":if state.external {"external"} else {"builtin"},"warning":state.warning});
        (state.pack.clone(), status)
    }
    /// Validate and persist before replacing in-memory data. Used only by the local desktop.
    pub fn install(&self, bytes: &[u8]) -> AppResult<()> {
        let pack = DataPack::parse(bytes)?;
        let path = self.path.as_ref().ok_or_else(storage_error)?;
        let mut state = self.state.lock().unwrap();
        atomic_write(path, bytes).map_err(|_| storage_error())?;
        state.pack = Arc::new(pack);
        state.warning = None;
        state.external = true;
        state.checked = Some(Instant::now());
        Ok(())
    }
}
fn atomic_write(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    static SERIAL: AtomicU64 = AtomicU64::new(0);
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(parent)?;
    let temp = parent.join(format!(
        ".echo-data-{}-{}.tmp",
        std::process::id(),
        SERIAL.fetch_add(1, Ordering::Relaxed)
    ));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)?;
    let result = (|| {
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temp, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    fn changed(version: &str) -> Vec<u8> {
        let mut value = DataPack::builtin().value;
        value["version"] = json!(version);
        value["characters"]["新角色"] = json!({"en":"New Resonator","release_date":"2026-11-01"});
        value["variant_names"]["新模式"] = json!("New Mode");
        let mut group = value["presets"]["linear"]["presets"][0].clone();
        group["presetName"] = json!("新角色");
        group["variants"][0]["id"] = json!("new-character");
        group["variants"][0]["variantName"] = json!("新模式");
        group["variants"][0]["weights"]["Crit_Rate"] = json!(99);
        value["presets"]["linear"]["presets"]
            .as_array_mut()
            .unwrap()
            .push(group);
        serde_json::to_vec(&value).unwrap()
    }
    fn temp_dir() -> PathBuf {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let p = std::env::temp_dir().join(format!(
            "echo-pack-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&p).unwrap();
        p
    }
    #[test]
    fn revision_ignores_object_key_order_and_whitespace() {
        fn reversed_json(value: &Value) -> String {
            match value {
                Value::Object(map) => format!(
                    "{{ {} }}",
                    map.iter()
                        .rev()
                        .map(|(key, value)| format!(
                            "{} : {}",
                            serde_json::to_string(key).unwrap(),
                            reversed_json(value)
                        ))
                        .collect::<Vec<_>>()
                        .join(",\n")
                ),
                Value::Array(items) => format!(
                    "[{}]",
                    items
                        .iter()
                        .map(reversed_json)
                        .collect::<Vec<_>>()
                        .join(",")
                ),
                _ => serde_json::to_string(value).unwrap(),
            }
        }
        let pack = DataPack::builtin();
        let reordered = DataPack::parse(reversed_json(&pack.value).as_bytes()).unwrap();
        assert_eq!(pack.revision, reordered.revision);
        assert_ne!(
            pack.revision,
            DataPack::parse(&changed("different")).unwrap().revision
        );
    }
    #[test]
    fn validates_complete_pack_and_rejects_incompatible_or_malformed_updates() {
        let base = DataPack::builtin();
        assert_eq!(
            base.value["presets"]["wuwa_echo_tool"]["presets"]
                .as_array()
                .unwrap()
                .len(),
            60
        );
        assert!(DataPack::parse(&changed("test")).is_ok());
        for (pointer, bad) in [
            ("/schema_version", json!(2)),
            ("/characters/今汐/release_date", json!("2026-02-30")),
            ("/characters/今汐/en", json!(null)),
            (
                "/presets/linear/presets/0/variants/0/weights/Crit_Rate",
                json!(-1),
            ),
            (
                "/presets/linear/presets/0/variants/0/weights/BadStat",
                json!(1),
            ),
            (
                "/presets/linear/presets/0/variants/0/normalizedMaxScore",
                json!(0),
            ),
        ] {
            let mut value = base.value.clone();
            let (parent, key) = pointer.rsplit_once('/').unwrap();
            value
                .pointer_mut(parent)
                .unwrap()
                .as_object_mut()
                .unwrap()
                .insert(key.into(), bad);
            assert!(
                DataPack::parse(&serde_json::to_vec(&value).unwrap()).is_err(),
                "{pointer}"
            );
        }
        let mut duplicate = base.value.clone();
        let entry = duplicate["presets"]["linear"]["presets"][0].clone();
        duplicate["presets"]["linear"]["presets"]
            .as_array_mut()
            .unwrap()
            .push(entry);
        assert!(DataPack::parse(&serde_json::to_vec(&duplicate).unwrap()).is_err());
        assert!(DataPack::parse(b"{").is_err());
        assert!(DataPack::parse(&vec![b' '; MAX_BYTES + 1]).is_err());
    }
    #[test]
    fn live_reload_and_desktop_cache_keep_last_good_data() {
        let dir = temp_dir();
        let path = dir.join("echo-data.json");
        let desktop = DataStore::cached(path.clone());
        assert!(desktop.snapshot().1["warning"].is_null());
        desktop.install(&changed("first")).unwrap();
        // A fresh process loads the cached data without a network connection.
        let server = DataStore::new(Some(path.clone()));
        assert_eq!(server.snapshot().0.value["version"], "first");
        desktop.install(&changed("second")).unwrap();
        server.state.lock().unwrap().checked = None;
        assert_eq!(server.snapshot().0.value["version"], "second");
        let before = fs::read(&path).unwrap();
        assert!(desktop.install(b"{}").is_err());
        assert_eq!(fs::read(&path).unwrap(), before);
        fs::write(&path, b"bad external file").unwrap();
        server.state.lock().unwrap().checked = None;
        let (pack, status) = server.snapshot();
        assert_eq!(pack.value["version"], "second");
        assert!(status["warning"].is_string());
        fs::remove_file(&path).unwrap();
        server.state.lock().unwrap().checked = None;
        assert_eq!(server.snapshot().0.value["version"], "second");
        desktop.install(&changed("recovered")).unwrap();
        server.state.lock().unwrap().checked = None;
        assert!(server.snapshot().1["warning"].is_null());
        // Failed persistence must not change active data.
        fs::remove_file(&path).unwrap();
        fs::create_dir(&path).unwrap();
        assert!(desktop.install(&changed("cannot-save")).is_err());
        assert_eq!(desktop.snapshot().0.value["version"], "recovered");
        fs::remove_dir_all(dir).unwrap();
    }
}
