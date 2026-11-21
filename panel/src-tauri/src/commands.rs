//! Tauri commands: a thin layer over `hashguard-core`.

use hashguard_core::baseline::{self, Baseline, Diff};
use hashguard_core::scan::{self, Control, FileReport, ScanOptions};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tauri::{AppHandle, Emitter, Manager, State};

type CmdResult<T> = Result<T, String>;

fn err(e: impl std::fmt::Display) -> String {
    format!("{e:#}")
}

#[derive(Default)]
pub struct AppState {
    cancel: Arc<AtomicBool>,
    scanning: Arc<AtomicBool>,
}

/// Which window backdrop is active: `"mica"` (Windows 11) or `"solid"`.
pub struct Backdrop(pub &'static str);

/// Turn on the Windows 11 Mica material when the OS supports it. The window is
/// created transparent, so the UI must paint its own background when this fails.
pub fn apply_backdrop(app: &AppHandle) -> &'static str {
    #[cfg(windows)]
    if windows_build() >= 22000 {
        if let Some(w) = app.get_webview_window("main") {
            use tauri::window::{Effect, EffectsBuilder};
            if w.set_effects(EffectsBuilder::new().effect(Effect::Mica).build())
                .is_ok()
            {
                return "mica";
            }
        }
    }
    let _ = app;
    "solid"
}

/// Real OS build number (RtlGetVersion is not subject to manifest version lies).
#[cfg(windows)]
fn windows_build() -> u32 {
    #[repr(C)]
    struct OsVersionInfoW {
        size: u32,
        major: u32,
        minor: u32,
        build: u32,
        platform: u32,
        csd: [u16; 128],
    }
    #[link(name = "ntdll")]
    extern "system" {
        fn RtlGetVersion(info: *mut OsVersionInfoW) -> i32;
    }
    let mut info = OsVersionInfoW {
        size: std::mem::size_of::<OsVersionInfoW>() as u32,
        major: 0,
        minor: 0,
        build: 0,
        platform: 0,
        csd: [0; 128],
    };
    // SAFETY: `info` is a correctly sized, initialised OSVERSIONINFOW.
    if unsafe { RtlGetVersion(&mut info) } == 0 {
        info.build
    } else {
        0
    }
}

#[tauri::command]
pub fn backdrop(state: State<'_, Backdrop>) -> &'static str {
    state.0
}

#[derive(Serialize)]
pub struct QuickPath {
    label: String,
    path: String,
}

#[derive(Serialize)]
pub struct ScanResultDto {
    reports: Vec<FileReport>,
    failures: Vec<(String, String)>,
    cancelled: bool,
    elapsed_ms: u64,
}

#[derive(Serialize, Clone)]
struct Progress {
    done: usize,
    total: usize,
}

#[derive(Serialize)]
pub struct BaselineInfo {
    name: String,
    root: String,
    created_unix: u64,
    files: usize,
}

#[derive(Serialize)]
pub struct VerifyResult {
    baseline: BaselineInfo,
    root: String,
    diff: Diff,
}

/// Folders worth scanning first, limited to the ones that exist.
#[tauri::command]
pub fn quick_paths() -> Vec<QuickPath> {
    let env = |k: &str| std::env::var_os(k).map(PathBuf::from);
    let mut out = Vec::new();
    let mut add = |label: &str, p: Option<PathBuf>| {
        if let Some(p) = p.filter(|p| p.is_dir()) {
            out.push(QuickPath {
                label: label.into(),
                path: p.display().to_string(),
            });
        }
    };
    add("System32", env("SystemRoot").map(|p| p.join("System32")));
    add("Загрузки", env("USERPROFILE").map(|p| p.join("Downloads")));
    add(
        "Рабочий стол",
        env("USERPROFILE").map(|p| p.join("Desktop")),
    );
    add("Temp", env("TEMP"));
    add("Program Files", env("ProgramFiles"));
    out
}

#[tauri::command]
pub async fn start_scan(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
    blocklist: Option<String>,
) -> CmdResult<ScanResultDto> {
    if state.scanning.swap(true, Ordering::SeqCst) {
        return Err("Сканирование уже выполняется".into());
    }
    state.cancel.store(false, Ordering::SeqCst);
    let cancel = state.cancel.clone();
    let scanning = state.scanning.clone();

    let joined = tauri::async_runtime::spawn_blocking(move || -> CmdResult<ScanResultDto> {
        let blocklist = match blocklist.filter(|b| !b.trim().is_empty()) {
            Some(p) => scan::load_blocklist(Path::new(p.trim())).map_err(err)?,
            None => Default::default(),
        };
        let opts = ScanOptions { blocklist };

        let started = Instant::now();
        let last_emit = AtomicU64::new(0);
        // Thousands of files finish per second; the UI only needs ~20 updates/s.
        let on_progress = |done: usize, total: usize| {
            let now = started.elapsed().as_millis() as u64;
            let prev = last_emit.load(Ordering::Relaxed);
            let due = now >= prev + 50
                && last_emit
                    .compare_exchange(prev, now, Ordering::Relaxed, Ordering::Relaxed)
                    .is_ok();
            if due || done == total {
                let _ = app.emit("scan-progress", Progress { done, total });
            }
        };

        let out = scan::scan_with(
            Path::new(path.trim()),
            &opts,
            &Control {
                cancel: &cancel,
                on_progress: &on_progress,
            },
        )
        .map_err(err)?;

        Ok(ScanResultDto {
            reports: out.reports,
            failures: out.failures,
            cancelled: out.cancelled,
            elapsed_ms: started.elapsed().as_millis() as u64,
        })
    })
    .await;

    scanning.store(false, Ordering::SeqCst);
    joined.map_err(err)?
}

#[tauri::command]
pub fn cancel_scan(state: State<'_, AppState>) {
    state.cancel.store(true, Ordering::SeqCst);
}

fn baselines_dir(app: &AppHandle) -> CmdResult<PathBuf> {
    let dir = app.path().app_data_dir().map_err(err)?.join("baselines");
    std::fs::create_dir_all(&dir).map_err(err)?;
    Ok(dir)
}

/// Baseline names become file names, so only allow harmless characters.
fn safe_name(name: &str) -> CmdResult<String> {
    let name = name.trim();
    let ok = !name.is_empty()
        && name.chars().count() <= 64
        && name
            .chars()
            .all(|c| c.is_alphanumeric() || c == ' ' || c == '-' || c == '_');
    if ok {
        Ok(name.to_string())
    } else {
        Err("Название: до 64 символов, только буквы, цифры, пробел, «-» и «_»".into())
    }
}

fn baseline_path(app: &AppHandle, name: &str) -> CmdResult<PathBuf> {
    Ok(baselines_dir(app)?.join(format!("{}.json", safe_name(name)?)))
}

fn info_of(name: &str, b: &Baseline) -> BaselineInfo {
    BaselineInfo {
        name: name.to_string(),
        root: b.root.clone(),
        created_unix: b.created_unix,
        files: b.files.len(),
    }
}

#[tauri::command]
pub fn list_baselines(app: AppHandle) -> CmdResult<Vec<BaselineInfo>> {
    let mut list = Vec::new();
    for entry in std::fs::read_dir(baselines_dir(&app)?)
        .map_err(err)?
        .flatten()
    {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let Some(name) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        if let Ok(b) = Baseline::load(&path) {
            list.push(info_of(name, &b));
        }
    }
    list.sort_by_key(|b| std::cmp::Reverse(b.created_unix));
    Ok(list)
}

#[tauri::command]
pub async fn create_baseline(
    app: AppHandle,
    name: String,
    path: String,
) -> CmdResult<BaselineInfo> {
    let name = safe_name(&name)?;
    let target = baseline_path(&app, &name)?;
    if target.exists() {
        return Err(format!("Эталон «{name}» уже существует"));
    }
    let root = PathBuf::from(path.trim());

    tauri::async_runtime::spawn_blocking(move || -> CmdResult<BaselineInfo> {
        let (files, _failures) = scan::hash_tree(&root).map_err(err)?;
        if files.is_empty() {
            return Err("В папке нет исполняемых файлов".into());
        }
        let b = Baseline::new(&root, files);
        b.save(&target).map_err(err)?;
        Ok(info_of(&name, &b))
    })
    .await
    .map_err(err)?
}

#[tauri::command]
pub async fn verify_baseline(app: AppHandle, name: String) -> CmdResult<VerifyResult> {
    let file = baseline_path(&app, &name)?;
    let name = safe_name(&name)?;

    tauri::async_runtime::spawn_blocking(move || -> CmdResult<VerifyResult> {
        let saved = Baseline::load(&file).map_err(err)?;
        let root = PathBuf::from(&saved.root);
        let (current, _failures) = scan::hash_tree(&root).map_err(err)?;
        let diff = baseline::diff(&saved, &current);
        Ok(VerifyResult {
            baseline: info_of(&name, &saved),
            root: saved.root.clone(),
            diff,
        })
    })
    .await
    .map_err(err)?
}

#[tauri::command]
pub fn delete_baseline(app: AppHandle, name: String) -> CmdResult<()> {
    std::fs::remove_file(baseline_path(&app, &name)?).map_err(err)
}

/// Open Explorer with the file selected.
#[tauri::command]
pub fn reveal_in_explorer(path: String) -> CmdResult<()> {
    if !Path::new(&path).exists() {
        return Err("Файл не найден".into());
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        std::process::Command::new("explorer")
            .raw_arg(format!("/select,\"{path}\""))
            .spawn()
            .map_err(err)?;
    }
    Ok(())
}

#[tauri::command]
pub fn save_text(path: String, content: String) -> CmdResult<()> {
    std::fs::write(path, content).map_err(err)
}
