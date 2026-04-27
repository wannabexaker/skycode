// Prevents additional console window on Windows in release mode
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::process::{Command, Stdio};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::PathBuf;
use std::env;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use tauri::Emitter;
use tauri::Manager;

// Windows: hide console windows for spawned processes
#[cfg(windows)]
use std::os::windows::process::CommandExt;
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

#[derive(Clone, serde::Serialize)]
struct MessagePayload {
    content: String,
    is_complete: bool,
}

#[derive(Clone, serde::Serialize)]
struct MessagePayloadV2 {
    request_id: String,
    content: String,
    is_complete: bool,
}

/// Shared state: currently running sky.exe processes (request_id -> pid)
struct AppState {
    running_pids: Arc<Mutex<HashMap<String, u32>>>,
}

// ── Binary resolution ─────────────────────────────────────────────────────────
//
// Tauri externalBin places sidecar binaries next to the app exe with the
// Cargo TARGET triple suffix: sky-x86_64-pc-windows-msvc.exe
// We must look for this name first, before falling back to plain "sky.exe".

fn target_triple() -> &'static str {
    if cfg!(target_arch = "x86_64") && cfg!(target_os = "windows") {
        "x86_64-pc-windows-msvc"
    } else if cfg!(all(target_arch = "aarch64", target_os = "windows")) {
        "aarch64-pc-windows-msvc"
    } else if cfg!(all(target_arch = "x86_64", target_os = "macos")) {
        "x86_64-apple-darwin"
    } else if cfg!(all(target_arch = "aarch64", target_os = "macos")) {
        "aarch64-apple-darwin"
    } else if cfg!(all(target_arch = "x86_64", target_os = "linux")) {
        "x86_64-unknown-linux-gnu"
    } else {
        "x86_64-pc-windows-msvc"
    }
}

fn exe_dir() -> PathBuf {
    env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("."))
}

fn sky_binary() -> PathBuf {
    if let Ok(p) = env::var("SKYCODE_BIN") {
        return PathBuf::from(p);
    }
    let dir = exe_dir();
    // 1. Tauri MSI install: sky-<triple>.exe next to app
    let sidecar = dir.join(format!("sky-{}{}", target_triple(), if cfg!(windows) { ".exe" } else { "" }));
    if sidecar.exists() { return sidecar; }
    // 2. Dev mode: plain sky.exe next to app (cargo run output dir)
    let plain = dir.join(if cfg!(windows) { "sky.exe" } else { "sky" });
    if plain.exists() { return plain; }
    // 3. Dev fallback: look in standard cargo release directories
    for candidate in [
        r"C:\Projects\Skycode\sky-code\target\release\sky.exe",
        r"C:\Projects\MyTests\ClaudeCode\sky-code\target\release\sky.exe",
        r"C:\Projects\MyTests\ClaudeCode\claw-code\rust\target\release\sky.exe",
    ] {
        let p = PathBuf::from(candidate);
        if p.exists() { return p; }
    }
    // Return the sidecar path even if not found so callers get a meaningful error
    sidecar
}

fn skybridge_binary() -> PathBuf {
    if let Ok(p) = env::var("SKYBRIDGE_BIN") {
        return PathBuf::from(p);
    }
    let dir = exe_dir();
    // 1. Tauri MSI install: skybridge-<triple>.exe next to app
    let sidecar = dir.join(format!("skybridge-{}{}", target_triple(), if cfg!(windows) { ".exe" } else { "" }));
    if sidecar.exists() { return sidecar; }
    // 2. Dev mode: plain skybridge.exe
    let plain = dir.join(if cfg!(windows) { "skybridge.exe" } else { "skybridge" });
    if plain.exists() { return plain; }
    // 3. Dev fallback: look in standard cargo release directories
    for candidate in [
        r"C:\Projects\Skycode\skybridge\target\release\skybridge.exe",
        r"C:\Projects\MyTests\ClaudeCode\skybridge\target\release\skybridge.exe",
        r"C:\Projects\MyTests\ClaudeCode\claw-code\rust\target\release\skybridge.exe",
    ] {
        let p = PathBuf::from(candidate);
        if p.exists() { return p; }
    }
    sidecar
}

// ── Health checks ─────────────────────────────────────────────────────────────

fn bridge_is_alive() -> bool {
    std::net::TcpStream::connect_timeout(
        &"127.0.0.1:4000".parse().unwrap(),
        std::time::Duration::from_millis(400),
    ).is_ok()
}

fn ollama_is_alive() -> bool {
    std::net::TcpStream::connect_timeout(
        &"127.0.0.1:11434".parse().unwrap(),
        std::time::Duration::from_millis(800),
    ).is_ok()
}

fn process_is_alive(pid: u32) -> bool {
    #[cfg(windows)]
    {
        let filter = format!("PID eq {}", pid);
        let mut cmd = Command::new("tasklist");
        cmd.args(["/FI", &filter, "/FO", "CSV", "/NH"]);
        cmd.creation_flags(CREATE_NO_WINDOW);

        match cmd.output() {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout).to_lowercase();
                if stdout.contains("no tasks are running") || stdout.contains("no instance") {
                    return false;
                }
                stdout.contains(&format!(",\"{}\"", pid)) || stdout.contains(&pid.to_string())
            }
            Err(_) => false,
        }
    }
    #[cfg(unix)]
    {
        Command::new("kill")
            .args(["-0", &pid.to_string()])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
    #[cfg(not(any(windows, unix)))]
    {
        false
    }
}

/// Query Ollama /api/tags via raw HTTP
fn ollama_list_models() -> Vec<String> {
    let Ok(mut stream) = std::net::TcpStream::connect("127.0.0.1:11434") else { return vec![]; };
    stream.set_read_timeout(Some(std::time::Duration::from_secs(4))).ok();
    let req = "GET /api/tags HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n";
    if stream.write_all(req.as_bytes()).is_err() { return vec![]; }
    let mut buf = String::new();
    stream.read_to_string(&mut buf).ok();
    let body = buf.split("\r\n\r\n").nth(1).unwrap_or("{}");
    let clean: String = body.lines()
        .filter(|l| {
            let t = l.trim();
            !(t.len() <= 8 && t.chars().all(|c| c.is_ascii_hexdigit()) && !t.is_empty())
        })
        .collect::<Vec<_>>()
        .join("\n");
    let json: serde_json::Value = serde_json::from_str(&clean).unwrap_or_default();
    json["models"].as_array()
        .map(|arr| arr.iter().filter_map(|m| m["name"].as_str().map(String::from)).collect())
        .unwrap_or_default()
}

// ── SkyBridge auto-start ──────────────────────────────────────────────────────

/// Kill any running skybridge that isn't our binary, then start ours.
fn ensure_bridge() {
    let bin = skybridge_binary();
    if !bin.exists() {
        eprintln!("[GUI] SkyBridge binary not found: {}", bin.display());
        return;
    }

    // On Windows: if a skybridge process is already running, check if it's the right one.
    // If it's a different binary (e.g., old C:\Program Files\SkyCode\skybridge.exe), kill it.
    #[cfg(windows)]
    {
        let correct = bin.to_string_lossy().to_lowercase();
        let mut check = Command::new("tasklist");
        check.args(["/FI", "IMAGENAME eq skybridge.exe", "/FO", "CSV", "/NH"]);
        check.creation_flags(CREATE_NO_WINDOW);
        if let Ok(out) = check.output() {
            let stdout = String::from_utf8_lossy(&out.stdout);
            if stdout.to_lowercase().contains("skybridge.exe") {
                // Get path of running skybridge via WMIC
                let mut wmic = Command::new("wmic");
                wmic.args(["process", "where", "name='skybridge.exe'", "get", "ExecutablePath", "/format:csv"]);
                wmic.creation_flags(CREATE_NO_WINDOW);
                if let Ok(wout) = wmic.output() {
                    let wstdout = String::from_utf8_lossy(&wout.stdout).to_lowercase();
                    let running_path = wstdout
                        .lines()
                        .find(|l| l.contains("skybridge.exe") && !l.trim_start().starts_with("node") && l.contains(":\\"))
                        .and_then(|l| l.split(',').last())
                        .unwrap_or("")
                        .trim()
                        .to_string();
                    if !running_path.is_empty() && running_path != correct {
                        eprintln!("[GUI] Wrong skybridge running ({}), killing it...", running_path);
                        let mut kill = Command::new("taskkill");
                        kill.args(["/F", "/IM", "skybridge.exe"]);
                        kill.creation_flags(CREATE_NO_WINDOW);
                        let _ = kill.output();
                        std::thread::sleep(std::time::Duration::from_millis(800));
                    } else if running_path == correct && bridge_is_alive() {
                        println!("[GUI] SkyBridge already running (correct binary)");
                        return;
                    }
                } else if bridge_is_alive() {
                    // Can't verify path but port is alive — assume it's fine
                    return;
                }
            }
        }
    }

    #[cfg(not(windows))]
    {
        if bridge_is_alive() { return; }
    }

    let mut cmd = Command::new(&bin);
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    match cmd.spawn() {
        Ok(_child) => {
            for _ in 0..15 {
                std::thread::sleep(std::time::Duration::from_millis(200));
                if bridge_is_alive() {
                    println!("[GUI] SkyBridge started on :4000 ({})", bin.display());
                    return;
                }
            }
            eprintln!("[GUI] SkyBridge did not respond within 3 s");
        }
        Err(e) => eprintln!("[GUI] Failed to start SkyBridge: {}", e),
    }
}

// ── Tauri commands ────────────────────────────────────────────────────────────

#[tauri::command]
async fn send_message(
    message: String,
    agent:   Option<String>,
    model:   Option<String>,
    state:   tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<String, String> {
    let request_id = format!("legacy_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis());
    send_message_v2(message, agent, model, request_id.clone(), None, None, None, state, app_handle).await?;
    Ok("ok".to_string())
}

#[tauri::command]
async fn send_message_v2(
    message: String,
    agent:   Option<String>,
    model:   Option<String>,
    request_id: String,
    base_url: Option<String>,
    api_key: Option<String>,
    permission_mode: Option<String>,
    state:   tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<String, String> {
    // If bridge died after app startup, retry launching before each request.
    ensure_bridge();

    let model_name = model.unwrap_or_else(|| "llama3.2:1b".to_string());
    if !model_name.chars().all(|c| c.is_alphanumeric() || matches!(c, '.' | ':' | '-' | '_' | '/')) {
        return Err("Invalid model name".to_string());
    }

    let sky = sky_binary();
    if !sky.exists() {
        return Err(format!("sky binary not found at: {}", sky.display()));
    }

    println!("[GUI] send_message_v2 | req={} model={} agent={:?} prompt_len={}", request_id, model_name, agent, message.len());

    let resolved_base_url = base_url
        .filter(|v| !v.trim().is_empty())
        .or_else(|| env::var("FILANTHROPIC_BASE_URL").ok())
        .unwrap_or_else(|| "http://localhost:4000".to_string());
    let resolved_api_key = api_key
        .filter(|v| !v.trim().is_empty())
        .or_else(|| env::var("FILANTHROPIC_API_KEY").ok())
        .unwrap_or_else(|| "ollama".to_string());
    let resolved_permission_mode = permission_mode
        .filter(|v| !v.trim().is_empty())
        .or_else(|| env::var("CLAW_PERMISSION_MODE").ok())
        .unwrap_or_else(|| "workspace-write".to_string());
    if !is_valid_permission_mode(&resolved_permission_mode) {
        return Err("Invalid permission mode (use read-only, workspace-write, or danger-full-access)".to_string());
    }

    // Use -p flag for one-shot prompt mode (NOT stdin which triggers REPL)
    let mut cmd = Command::new(&sky);
    cmd.arg("-p").arg(&message)
        .arg("--model").arg(&model_name)
        .arg("--permission-mode").arg(&resolved_permission_mode)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env("HOME", env::var("HOME").unwrap_or_else(|_| env::var("USERPROFILE").unwrap_or_default()))
        .env("FILANTHROPIC_BASE_URL", resolved_base_url)
        .env("FILANTHROPIC_API_KEY",  resolved_api_key)
        .env("FILANTHROPIC_MODEL",    &model_name);

    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);

    let child = cmd.spawn()
        .map_err(|e| format!("Failed to spawn sky: {}", e))?;

    let pid = child.id();
    state.running_pids.lock().unwrap().insert(request_id.clone(), pid);
    println!("[GUI] sky.exe spawned | req={} pid={}", request_id, pid);

    let stdout = child.stdout;
    let stderr = child.stderr;

    // Coordinate stream completion to avoid dropping stderr-only failures.
    let open_streams = Arc::new(AtomicUsize::new(2));
    let had_visible_output = Arc::new(AtomicBool::new(false));
    let stderr_lines: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));

    // Stream stdout line-by-line to frontend.
    if let Some(out) = stdout {
        let reader = BufReader::new(out);
        let app = app_handle.clone();
        let req_id = request_id.clone();
        let open_streams = open_streams.clone();
        let had_visible_output = had_visible_output.clone();
        let stderr_lines = stderr_lines.clone();
        let pid_state = state.running_pids.clone();

        std::thread::spawn(move || {
            for line in reader.lines() {
                match line {
                    Ok(text) => {
                        had_visible_output.store(true, Ordering::SeqCst);
                        app.emit("message-chunk", MessagePayload {
                            content: text.clone(),
                            is_complete: false,
                        }).ok();
                        app.emit("message-chunk-v2", MessagePayloadV2 {
                            request_id: req_id.clone(),
                            content: text,
                            is_complete: false,
                        }).ok();
                    }
                    Err(_) => break,
                }
            }

            if open_streams.fetch_sub(1, Ordering::SeqCst) == 1 {
                if !had_visible_output.load(Ordering::SeqCst) {
                    let fallback = stderr_lines.lock().unwrap().join("\n");
                    if !fallback.trim().is_empty() {
                        app.emit("message-chunk-v2", MessagePayloadV2 {
                            request_id: req_id.clone(),
                            content: fallback,
                            is_complete: false,
                        }).ok();
                    }
                }
                pid_state.lock().unwrap().remove(&req_id);
                app.emit("message-chunk", MessagePayload {
                    content: String::new(),
                    is_complete: true,
                }).ok();
                app.emit("message-chunk-v2", MessagePayloadV2 {
                    request_id: req_id,
                    content: String::new(),
                    is_complete: true,
                }).ok();
            }
        });
    }

    // Stream stderr both to logs and UI (as diagnostic lines).
    if let Some(err) = stderr {
        let app = app_handle.clone();
        let req_id = request_id.clone();
        let open_streams = open_streams.clone();
        let had_visible_output = had_visible_output.clone();
        let stderr_lines = stderr_lines.clone();
        let pid_state = state.running_pids.clone();

        std::thread::spawn(move || {
            let reader = BufReader::new(err);
            for line in reader.lines() {
                if let Ok(line) = line {
                    eprintln!("[sky.exe stderr] {}", line);
                    stderr_lines.lock().unwrap().push(format!("[stderr] {}", line));
                    had_visible_output.store(true, Ordering::SeqCst);
                    app.emit("message-chunk-v2", MessagePayloadV2 {
                        request_id: req_id.clone(),
                        content: format!("[stderr] {}", line),
                        is_complete: false,
                    }).ok();
                }
            }

            if open_streams.fetch_sub(1, Ordering::SeqCst) == 1 {
                if !had_visible_output.load(Ordering::SeqCst) {
                    let fallback = stderr_lines.lock().unwrap().join("\n");
                    if !fallback.trim().is_empty() {
                        app.emit("message-chunk-v2", MessagePayloadV2 {
                            request_id: req_id.clone(),
                            content: fallback,
                            is_complete: false,
                        }).ok();
                    }
                }
                pid_state.lock().unwrap().remove(&req_id);
                app.emit("message-chunk", MessagePayload {
                    content: String::new(),
                    is_complete: true,
                }).ok();
                app.emit("message-chunk-v2", MessagePayloadV2 {
                    request_id: req_id,
                    content: String::new(),
                    is_complete: true,
                }).ok();
            }
        });
    }

    Ok(request_id)
}

#[tauri::command]
async fn cancel_message(state: tauri::State<'_, AppState>) -> Result<(), String> {
    // Backward-compatible: cancel the first running request if present
    let pid = {
        let mut guard = state.running_pids.lock().unwrap();
        let first = guard.iter().next().map(|(k, v)| (k.clone(), *v));
        if let Some((k, _)) = first.clone() { guard.remove(&k); }
        first.map(|(_, v)| v)
    };

    if let Some(pid) = pid {
        println!("[GUI] Killing sky.exe PID {}", pid);
        #[cfg(windows)]
        {
            let mut kill = Command::new("taskkill");
            kill.args(["/PID", &pid.to_string(), "/F", "/T"]);
            kill.creation_flags(CREATE_NO_WINDOW);
            kill.spawn().ok();
        }
        #[cfg(not(windows))]
        {
            Command::new("kill").arg("-9").arg(pid.to_string()).spawn().ok();
        }
    }
    Ok(())
}

#[tauri::command]
async fn cancel_message_v2(request_id: String, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let pid = state.running_pids.lock().unwrap().remove(&request_id);
    if let Some(pid) = pid {
        println!("[GUI] Killing sky.exe | req={} pid={}", request_id, pid);
        #[cfg(windows)]
        {
            let mut kill = Command::new("taskkill");
            kill.args(["/PID", &pid.to_string(), "/F", "/T"]);
            kill.creation_flags(CREATE_NO_WINDOW);
            kill.spawn().ok();
        }
        #[cfg(not(windows))]
        {
            Command::new("kill").arg("-9").arg(pid.to_string()).spawn().ok();
        }
    }
    Ok(())
}

#[tauri::command]
async fn list_models() -> Result<Vec<String>, String> {
    Ok(ollama_list_models())
}

#[tauri::command]
async fn get_diagnostics() -> Result<serde_json::Value, String> {
    let sky = sky_binary();
    let bridge = skybridge_binary();
    let dir = exe_dir();

    // List files next to the exe for debugging
    let nearby_files: Vec<String> = std::fs::read_dir(&dir)
        .map(|rd| rd
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect()
        )
        .unwrap_or_default();

    Ok(serde_json::json!({
        "sky_binary_path":        sky.to_string_lossy(),
        "sky_binary_exists":      sky.exists(),
        "skybridge_binary_path":  bridge.to_string_lossy(),
        "skybridge_binary_exists":bridge.exists(),
        "exe_dir":                dir.to_string_lossy(),
        "target_triple":          target_triple(),
        "bridge_alive":           bridge_is_alive(),
        "ollama_alive":           ollama_is_alive(),
        "nearby_files":           nearby_files,
    }))
}

#[tauri::command]
async fn get_request_status(
    request_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let pid = state.running_pids.lock().unwrap().get(&request_id).copied();
    let process_alive = pid.map(process_is_alive).unwrap_or(false);

    Ok(serde_json::json!({
        "request_id": request_id,
        "known": pid.is_some(),
        "pid": pid,
        "process_alive": process_alive,
        "bridge_alive": bridge_is_alive(),
        "ollama_alive": ollama_is_alive(),
    }))
}

#[tauri::command]
async fn get_status() -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "ollama":  ollama_is_alive(),
        "bridge":  bridge_is_alive(),
        "sky_bin": sky_binary().exists(),
        "model":   env::var("FILANTHROPIC_MODEL").unwrap_or_else(|_| "llama3.2:1b".to_string()),
    }))
}

#[tauri::command]
async fn get_settings() -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "model":           env::var("FILANTHROPIC_MODEL").unwrap_or_else(|_| "llama3.2:1b".to_string()),
        "base_url":        env::var("FILANTHROPIC_BASE_URL").unwrap_or_else(|_| "http://localhost:4000".to_string()),
        "permission_mode": env::var("CLAW_PERMISSION_MODE").unwrap_or_else(|_| "workspace-write".to_string()),
        "bridge_alive":    bridge_is_alive(),
        "ollama_alive":    ollama_is_alive(),
    }))
}

#[tauri::command]
async fn save_settings(settings: serde_json::Value) -> Result<(), String> {
    println!("[GUI] Save settings: {:?}", settings);
    Ok(())
}

fn is_safe_cli_token(value: &str) -> bool {
    !value.trim().is_empty()
        && value
            .chars()
            .all(|c| c.is_alphanumeric() || matches!(c, '.' | ':' | '-' | '_' | '/'))
}

fn is_valid_permission_mode(value: &str) -> bool {
    matches!(value, "read-only" | "workspace-write" | "danger-full-access")
}

fn run_sky_models_command(sub_args: &[String], profile: Option<String>) -> Result<serde_json::Value, String> {
    let sky = sky_binary();
    if !sky.exists() {
        return Err(format!("sky binary not found at: {}", sky.display()));
    }

    let mut cmd = Command::new(&sky);
    if let Some(profile) = profile {
        if !is_safe_cli_token(&profile) {
            return Err("Invalid profile name".to_string());
        }
        cmd.arg("--profile").arg(profile);
    }

    cmd.arg("models");
    for arg in sub_args {
        cmd.arg(arg);
    }

    cmd.env(
        "HOME",
        env::var("HOME").unwrap_or_else(|_| env::var("USERPROFILE").unwrap_or_default()),
    );

    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);

    let output = cmd
        .output()
        .map_err(|e| format!("Failed to execute sky models command: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if output.status.success() {
        Ok(serde_json::json!({
            "ok": true,
            "stdout": stdout,
            "stderr": stderr,
        }))
    } else {
        Err(format!(
            "Models command failed: {}",
            if stderr.trim().is_empty() { stdout.trim() } else { stderr.trim() }
        ))
    }
}

#[tauri::command]
async fn install_model(name: String, profile: Option<String>) -> Result<serde_json::Value, String> {
    if !is_safe_cli_token(&name) {
        return Err("Invalid model name".to_string());
    }
    run_sky_models_command(&["install".to_string(), name], profile)
}

#[tauri::command]
async fn uninstall_model(name: String, profile: Option<String>) -> Result<serde_json::Value, String> {
    if !is_safe_cli_token(&name) {
        return Err("Invalid model name".to_string());
    }
    run_sky_models_command(&["uninstall".to_string(), name], profile)
}

#[tauri::command]
async fn apply_preset(preset_id: String, profile: Option<String>) -> Result<serde_json::Value, String> {
    if !is_safe_cli_token(&preset_id) {
        return Err("Invalid preset id".to_string());
    }
    run_sky_models_command(&["install-pack".to_string(), preset_id], profile)
}

#[tauri::command]
async fn create_model_alias(
    alias: String,
    source: String,
    profile: Option<String>,
) -> Result<serde_json::Value, String> {
    if !is_safe_cli_token(&alias) || !is_safe_cli_token(&source) {
        return Err("Invalid alias or source model".to_string());
    }
    run_sky_models_command(&["create-alias".to_string(), alias, source], profile)
}

fn remove_dir_if_exists(path: &PathBuf) {
    if path.exists() {
        let _ = std::fs::remove_dir_all(path);
    }
}

fn remove_external_data_dirs() {
    let home = env::var("HOME").ok().or_else(|| env::var("USERPROFILE").ok());
    if let Some(home) = home {
        let home_path = PathBuf::from(home);
        remove_dir_if_exists(&home_path.join(".skycode"));
        remove_dir_if_exists(&home_path.join(".codex"));
    }
    if let Ok(codex_home) = env::var("CODEX_HOME") {
        remove_dir_if_exists(&PathBuf::from(codex_home));
    }
}

#[tauri::command]
async fn safe_uninstall(
    delete_temp: bool,
    clear_local_data: bool,
    delete_external_data: bool,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let resolver = app_handle.path();

    // Persistent local app memory (only when explicitly requested)
    if clear_local_data {
        if let Ok(dir) = resolver.app_data_dir() {
            remove_dir_if_exists(&dir);
        }
        if let Ok(dir) = resolver.app_local_data_dir() {
            remove_dir_if_exists(&dir);
        }
    }

    // Optional temporary memory/cache
    if delete_temp {
        if let Ok(dir) = resolver.app_cache_dir() {
            remove_dir_if_exists(&dir);
        }
        if let Ok(dir) = resolver.app_log_dir() {
            remove_dir_if_exists(&dir);
        }
    }

    if delete_external_data {
        remove_external_data_dirs();
    }

    // Open uninstall UI so user can complete OS uninstall
    #[cfg(windows)]
    {
        let mut cmd = Command::new("explorer.exe");
        cmd.arg("ms-settings:appsfeatures");
        cmd.creation_flags(CREATE_NO_WINDOW);
        cmd.spawn().ok();
    }

    Ok(())
}

// ── main ──────────────────────────────────────────────────────────────────────

fn main() {
    ensure_bridge();

    tauri::Builder::default()
        .manage(AppState {
            running_pids: Arc::new(Mutex::new(HashMap::new())),
        })
        .invoke_handler(tauri::generate_handler![
            send_message,
            send_message_v2,
            cancel_message,
            cancel_message_v2,
            list_models,
            get_status,
            get_settings,
            get_diagnostics,
            get_request_status,
            save_settings,
            install_model,
            uninstall_model,
            apply_preset,
            create_model_alias,
            safe_uninstall,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
