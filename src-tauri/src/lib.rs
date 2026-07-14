use base64::{engine::general_purpose, Engine as _};
use image::{codecs::png::PngEncoder, ColorType, ImageEncoder};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    fs::{self, OpenOptions},
    io,
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, State, WindowEvent,
};

const PREVIEW_CACHE_SUFFIX: &str = ".centered-v1.png";
static ID_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CursorRole {
    role: String,
    windows_key: String,
    file_path: Option<String>,
    file_name: Option<String>,
    #[serde(default)]
    preview_path: Option<String>,
    #[serde(default)]
    preview_data_url: Option<String>,
    #[serde(rename = "type")]
    cursor_type: Option<String>,
    exists: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CursorFile {
    file_path: String,
    file_name: String,
    #[serde(default)]
    preview_path: Option<String>,
    #[serde(default)]
    preview_data_url: Option<String>,
    #[serde(rename = "type")]
    cursor_type: String,
    exists: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SkinPackage {
    id: String,
    name: String,
    source_path: String,
    storage_path: String,
    imported_at: String,
    has_inf: bool,
    is_complete: bool,
    cursor_count: usize,
    is_applied: bool,
    #[serde(default)]
    import_note: Option<String>,
    roles: Vec<CursorRole>,
    #[serde(default)]
    unassigned_files: Vec<CursorFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AppSettings {
    close_to_tray: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            close_to_tray: true,
        }
    }
}

struct SettingsState {
    close_to_tray: AtomicBool,
    tray_available: AtomicBool,
}

#[derive(Debug, Clone)]
struct RoleDef {
    role: &'static str,
    windows_key: &'static str,
    keywords: &'static [&'static str],
}

struct CursorMutation {
    old_path: Option<String>,
    new_path: String,
}

const ROLE_DEFS: &[RoleDef] = &[
    RoleDef {
        role: "Normal Select",
        windows_key: "Arrow",
        keywords: &["arrow", "normal", "pointer"],
    },
    RoleDef {
        role: "Help Select",
        windows_key: "Help",
        keywords: &["help"],
    },
    RoleDef {
        role: "Working In Background",
        windows_key: "AppStarting",
        keywords: &["appstarting", "working"],
    },
    RoleDef {
        role: "Busy",
        windows_key: "Wait",
        keywords: &["wait", "busy", "loading"],
    },
    RoleDef {
        role: "Precision Select",
        windows_key: "Crosshair",
        keywords: &["cross", "crosshair"],
    },
    RoleDef {
        role: "Text Select",
        windows_key: "IBeam",
        keywords: &["text", "ibeam", "beam"],
    },
    RoleDef {
        role: "Handwriting",
        windows_key: "NWPen",
        keywords: &["pen", "handwriting"],
    },
    RoleDef {
        role: "Unavailable",
        windows_key: "No",
        keywords: &["no", "unavailable", "forbidden"],
    },
    RoleDef {
        role: "Vertical Resize",
        windows_key: "SizeNS",
        keywords: &["sizens", "size_ns", "ns", "vert", "vertical"],
    },
    RoleDef {
        role: "Horizontal Resize",
        windows_key: "SizeWE",
        keywords: &["sizewe", "size_we", "we", "horz", "horizontal"],
    },
    RoleDef {
        role: "Diagonal Resize 1",
        windows_key: "SizeNWSE",
        keywords: &["nwse", "size_nwse"],
    },
    RoleDef {
        role: "Diagonal Resize 2",
        windows_key: "SizeNESW",
        keywords: &["nesw", "size_nesw"],
    },
    RoleDef {
        role: "Move",
        windows_key: "SizeAll",
        keywords: &["move", "all", "sizeall", "size_all"],
    },
    RoleDef {
        role: "Alternate Select",
        windows_key: "UpArrow",
        keywords: &["up", "alternate", "uparrow", "up_arrow"],
    },
    RoleDef {
        role: "Link Select",
        windows_key: "Hand",
        keywords: &["hand", "link"],
    },
];

#[tauri::command]
fn load_library(app: AppHandle) -> Result<Vec<SkinPackage>, String> {
    run_logged(&app, "load_library", read_library)
}

#[tauri::command]
fn import_skin(app: AppHandle, source_path: String) -> Result<SkinPackage, String> {
    run_logged(&app, "import_skin", |app| {
        import_skin_inner(app, source_path)
    })
}

fn import_skin_inner(app: &AppHandle, source_path: String) -> Result<SkinPackage, String> {
    let source = PathBuf::from(&source_path);
    if !source.exists() {
        return Err("选择的路径不存在。".into());
    }

    let mut library = read_library(app)?;
    if library
        .iter()
        .any(|skin| source_path_matches(&skin.source_path, &source))
    {
        return Err("该皮肤包已经导入过。".into());
    }

    let skin_id = new_id();
    let storage_path = app_data_dir(app)?.join("skins").join(&skin_id);
    fs::create_dir_all(&storage_path).map_err(to_string)?;

    if source.is_dir() {
        copy_dir(&source, &storage_path).map_err(to_string)?;
    } else if extension_is(&source, "zip") {
        extract_zip(&source, &storage_path).map_err(to_string)?;
    } else if extension_is(&source, "inf") {
        copy_inf_package(&source, &storage_path).map_err(to_string)?;
    } else {
        let file_name = source.file_name().ok_or("无法读取文件名。")?;
        fs::copy(&source, storage_path.join(file_name)).map_err(to_string)?;
    }

    let mut skin = analyze_skin(&skin_id, &source, &storage_path)?;
    if skin.cursor_count == 0 {
        let _ = fs::remove_dir_all(&storage_path);
        return Err("未找到 .cur / .ani 文件，请选择有效的鼠标指针皮肤包。".into());
    }

    library.insert(0, skin.clone());
    write_library(app, &library)?;
    skin.storage_path = storage_path.to_string_lossy().to_string();
    hydrate_skin_previews(&mut skin);
    Ok(skin)
}

#[tauri::command]
fn replace_cursor_role(
    app: AppHandle,
    skin_id: String,
    windows_key: String,
    source_path: String,
) -> Result<Vec<SkinPackage>, String> {
    let action =
        format!("replace_cursor_role skin_id={skin_id} role={windows_key} source={source_path}");
    run_logged(&app, &action, move |app| {
        replace_cursor_role_inner(app, &skin_id, &windows_key, &source_path)
    })
}

fn replace_cursor_role_inner(
    app: &AppHandle,
    skin_id: &str,
    windows_key: &str,
    source_path: &str,
) -> Result<Vec<SkinPackage>, String> {
    let source = PathBuf::from(source_path);
    let mut library = read_library(app)?;
    let skin_index = library
        .iter()
        .position(|skin| skin.id == skin_id)
        .ok_or("未找到该皮肤包。")?;
    let role_index = library[skin_index]
        .roles
        .iter()
        .position(|role| role.windows_key.eq_ignore_ascii_case(windows_key))
        .ok_or("未找到要替换的 Windows 光标角色。")?;
    let storage_path = validated_skin_storage(app, &library[skin_index])?;
    let attempted_old_path = library[skin_index].roles[role_index]
        .file_path
        .clone()
        .unwrap_or_else(|| "<unset>".to_string());

    let mutation = replace_cursor_role_transaction(
        &mut library,
        skin_index,
        role_index,
        &source,
        &storage_path,
        |library| write_library(app, library),
    )
    .map_err(|error| {
        append_log(
            app,
            &format!(
                "replace_cursor_role rollback skin_id={skin_id} role={windows_key} old={attempted_old_path} new={} reason={error}",
                source.to_string_lossy()
            ),
        );
        error
    })?;

    if let Err(error) = cleanup_unused_preview_cache(&library[skin_index]) {
        append_log(
            app,
            &format!("replace_cursor_role preview cleanup warning: {error}"),
        );
    }
    append_log(
        app,
        &format!(
            "replace_cursor_role committed skin_id={skin_id} role={windows_key} old={} new={}",
            mutation.old_path.as_deref().unwrap_or("<unset>"),
            mutation.new_path
        ),
    );
    hydrate_library_previews(&mut library);
    Ok(library)
}

#[tauri::command]
fn assign_unassigned_cursor(
    app: AppHandle,
    skin_id: String,
    source_file_path: String,
    windows_key: String,
) -> Result<Vec<SkinPackage>, String> {
    let action = format!(
        "assign_unassigned_cursor skin_id={skin_id} role={windows_key} source={source_file_path}"
    );
    run_logged(&app, &action, move |app| {
        assign_unassigned_cursor_inner(app, &skin_id, &source_file_path, &windows_key)
    })
}

fn assign_unassigned_cursor_inner(
    app: &AppHandle,
    skin_id: &str,
    source_file_path: &str,
    windows_key: &str,
) -> Result<Vec<SkinPackage>, String> {
    let mut library = read_library(app)?;
    let skin_index = library
        .iter()
        .position(|skin| skin.id == skin_id)
        .ok_or("未找到该皮肤包。")?;
    let storage_path = validated_skin_storage(app, &library[skin_index])?;
    let role_index = library[skin_index]
        .roles
        .iter()
        .position(|role| role.windows_key.eq_ignore_ascii_case(windows_key))
        .ok_or("未找到目标 Windows 光标角色。")?;
    let unassigned_index = library[skin_index]
        .unassigned_files
        .iter()
        .position(|file| stored_paths_match(&file.file_path, source_file_path))
        .ok_or("该文件已不在未分配列表中，请刷新后重试。")?;
    let attempted_old_path = library[skin_index].roles[role_index]
        .file_path
        .clone()
        .unwrap_or_else(|| "<unset>".to_string());

    let source = PathBuf::from(&library[skin_index].unassigned_files[unassigned_index].file_path);
    let canonical_source = source
        .canonicalize()
        .map_err(|_| "未分配光标文件不存在。")?;
    if !canonical_source.starts_with(&storage_path) {
        return Err("未分配光标文件不在当前皮肤的应用内部目录中。".into());
    }

    let mutation = assign_unassigned_cursor_transaction(
        &mut library,
        skin_index,
        role_index,
        unassigned_index,
        &storage_path,
        |library| write_library(app, library),
    )
    .map_err(|error| {
        append_log(
            app,
            &format!(
                "assign_unassigned_cursor rollback skin_id={skin_id} role={windows_key} old={attempted_old_path} new={} reason={error}",
                source.to_string_lossy()
            ),
        );
        error
    })?;

    if let Err(error) = cleanup_unused_preview_cache(&library[skin_index]) {
        append_log(
            app,
            &format!("assign_unassigned_cursor preview cleanup warning: {error}"),
        );
    }
    append_log(
        app,
        &format!(
            "assign_unassigned_cursor committed skin_id={skin_id} role={windows_key} old={} new={}",
            mutation.old_path.as_deref().unwrap_or("<unset>"),
            mutation.new_path
        ),
    );
    hydrate_library_previews(&mut library);
    Ok(library)
}

#[tauri::command]
fn delete_skin(app: AppHandle, skin_id: String) -> Result<(), String> {
    run_logged(&app, "delete_skin", |app| delete_skin_inner(app, skin_id))
}

fn delete_skin_inner(app: &AppHandle, skin_id: String) -> Result<(), String> {
    let mut library = read_library(app)?;
    if let Some((is_applied, storage_path)) = library
        .iter()
        .find(|item| item.id == skin_id)
        .map(|skin| (skin.is_applied, skin.storage_path.clone()))
    {
        if is_applied {
            reset_windows_default_cursors()?;
            for item in &mut library {
                item.is_applied = false;
            }
        }

        let path = PathBuf::from(storage_path);
        if path.exists() {
            fs::remove_dir_all(path).map_err(to_string)?;
        }
    }
    library.retain(|item| item.id != skin_id);
    write_library(app, &library)?;
    Ok(())
}

#[tauri::command]
fn apply_skin(app: AppHandle, skin_id: String) -> Result<Vec<SkinPackage>, String> {
    run_logged(&app, "apply_skin", |app| apply_skin_inner(app, skin_id))
}

#[tauri::command]
fn reset_system_cursors(app: AppHandle) -> Result<Vec<SkinPackage>, String> {
    run_logged(&app, "reset_system_cursors", reset_system_cursors_inner)
}

fn apply_skin_inner(app: &AppHandle, skin_id: String) -> Result<Vec<SkinPackage>, String> {
    let mut library = read_library(app)?;
    let skin = library
        .iter()
        .find(|item| item.id == skin_id)
        .cloned()
        .ok_or("未找到该皮肤包。")?;

    if skin
        .roles
        .iter()
        .any(|role| role.file_path.is_some() && !role.exists)
    {
        return Err("应用失败：应用内部保存的部分光标文件已缺失，请重新导入该皮肤包。".into());
    }

    apply_to_windows_current_user(&skin)?;

    for item in &mut library {
        item.is_applied = item.id == skin_id;
    }
    write_library(app, &library)?;
    Ok(library)
}

fn reset_system_cursors_inner(app: &AppHandle) -> Result<Vec<SkinPackage>, String> {
    reset_windows_default_cursors()?;

    let mut library = read_library(app)?;
    for item in &mut library {
        item.is_applied = false;
    }
    write_library(app, &library)?;
    Ok(library)
}

#[tauri::command]
fn load_app_settings(
    app: AppHandle,
    state: State<'_, SettingsState>,
) -> Result<AppSettings, String> {
    let mut settings = read_app_settings(&app)?;
    settings.close_to_tray &= state.tray_available.load(Ordering::Relaxed);
    state
        .close_to_tray
        .store(settings.close_to_tray, Ordering::Relaxed);
    Ok(settings)
}

#[tauri::command]
fn set_close_to_tray(
    app: AppHandle,
    state: State<'_, SettingsState>,
    enabled: bool,
) -> Result<AppSettings, String> {
    if enabled && !state.tray_available.load(Ordering::Relaxed) {
        return Err("系统托盘当前不可用，无法启用关闭到托盘。".into());
    }
    let settings = AppSettings {
        close_to_tray: enabled,
    };
    write_app_settings(&app, &settings)?;
    state.close_to_tray.store(enabled, Ordering::Relaxed);
    Ok(settings)
}

#[tauri::command]
fn clear_all_skins(app: AppHandle) -> Result<Vec<SkinPackage>, String> {
    run_logged(&app, "clear_all_skins", clear_all_skins_inner)
}

fn clear_all_skins_inner(app: &AppHandle) -> Result<Vec<SkinPackage>, String> {
    let library = read_library(app)?;
    if library.iter().any(|skin| skin.is_applied) {
        reset_windows_default_cursors()?;
    }

    let skins_path = app_data_dir(app)?.join("skins");
    if skins_path.exists() {
        fs::remove_dir_all(&skins_path).map_err(to_string)?;
    }
    fs::create_dir_all(&skins_path).map_err(to_string)?;
    write_library(app, &[])?;
    Ok(Vec::new())
}

#[tauri::command]
fn open_skin_dir(app: AppHandle, skin_id: String) -> Result<(), String> {
    run_logged(&app, "open_skin_dir", |app| {
        open_skin_dir_inner(app, skin_id)
    })
}

fn open_skin_dir_inner(app: &AppHandle, skin_id: String) -> Result<(), String> {
    let library = read_library(app)?;
    let skin = library
        .iter()
        .find(|skin| skin.id == skin_id)
        .ok_or("未找到该皮肤包。")?;
    let skins_root = app_data_dir(app)?
        .join("skins")
        .canonicalize()
        .map_err(to_string)?;
    let path = PathBuf::from(&skin.storage_path)
        .canonicalize()
        .map_err(to_string)?;
    if !path.starts_with(&skins_root) {
        return Err("皮肤目录不在应用数据范围内。".into());
    }

    tauri_plugin_opener::open_path(path.to_string_lossy().to_string(), None::<&str>)
        .map_err(to_string)
}

#[tauri::command]
fn open_log_file(app: AppHandle) -> Result<(), String> {
    run_logged(&app, "open_log_file", |app| {
        let path = log_path(app)?;
        if !path.exists() {
            append_log(app, "日志文件已创建。");
        }
        tauri_plugin_opener::open_path(path.to_string_lossy().to_string(), None::<&str>)
            .map_err(to_string)
    })
}

#[tauri::command]
fn refresh_system_cursors(app: AppHandle) -> Result<(), String> {
    run_logged(&app, "refresh_system_cursors", |_app| {
        refresh_windows_cursors()
    })
}

fn analyze_skin(skin_id: &str, source: &Path, storage: &Path) -> Result<SkinPackage, String> {
    let cursor_files = collect_files(storage, &["cur", "ani"]).map_err(to_string)?;
    let inf_files = collect_files(storage, &["inf"]).map_err(to_string)?;
    let has_inf = !inf_files.is_empty();
    let preview_dir = storage.join(".previews");
    fs::create_dir_all(&preview_dir).map_err(to_string)?;

    let mut matches = HashMap::<String, PathBuf>::new();
    let mut inf_match_count = 0;
    if let Some(inf_path) = inf_files.first() {
        let parsed = parse_inf(inf_path, &cursor_files).unwrap_or_default();
        inf_match_count = parsed.len();
        matches.extend(parsed);
    }

    for file in &cursor_files {
        if let Some(role) = match_by_filename(file) {
            matches
                .entry(role.to_string())
                .or_insert_with(|| file.clone());
        }
    }

    let roles: Vec<CursorRole> = ROLE_DEFS
        .iter()
        .map(|def| {
            let path = matches.get(def.windows_key).cloned();
            let file_name = path
                .as_ref()
                .and_then(|value| value.file_name())
                .map(|value| value.to_string_lossy().to_string());
            let cursor_type = path
                .as_ref()
                .and_then(|value| value.extension())
                .map(|value| value.to_string_lossy().to_ascii_lowercase());
            let preview_path = path
                .as_ref()
                .and_then(|value| build_cursor_preview(value, &preview_dir).ok().flatten());

            CursorRole {
                role: def.role.to_string(),
                windows_key: def.windows_key.to_string(),
                file_path: path
                    .as_ref()
                    .map(|value| value.to_string_lossy().to_string()),
                file_name,
                preview_path: preview_path.map(|value| value.to_string_lossy().to_string()),
                preview_data_url: None,
                cursor_type,
                exists: path.as_ref().is_some_and(|value| value.exists()),
            }
        })
        .collect();

    let matched_files: Vec<PathBuf> = matches.values().cloned().collect();
    let unassigned_files = cursor_files
        .iter()
        .filter(|file| !matched_files.iter().any(|matched| same_path(matched, file)))
        .map(|file| {
            let preview_path = build_cursor_preview(file, &preview_dir).ok().flatten();
            CursorFile {
                file_path: file.to_string_lossy().to_string(),
                file_name: file
                    .file_name()
                    .map(|value| value.to_string_lossy().to_string())
                    .unwrap_or_else(|| "unknown".to_string()),
                preview_path: preview_path.map(|value| value.to_string_lossy().to_string()),
                preview_data_url: None,
                cursor_type: file
                    .extension()
                    .map(|value| value.to_string_lossy().to_ascii_lowercase())
                    .unwrap_or_else(|| "cur".to_string()),
                exists: file.exists(),
            }
        })
        .collect();

    let name = infer_skin_name(source, inf_files.first());
    let is_complete = roles.iter().all(|role| role.exists);
    let import_note = if has_inf && inf_match_count == 0 {
        Some("未能完整解析安装配置，已尝试根据文件名识别光标。".to_string())
    } else if has_inf && inf_match_count < 15 && !is_complete {
        Some(format!(
            "安装配置已匹配 {inf_match_count} / 15 个光标角色，未设置角色应用时保持当前系统配置。"
        ))
    } else {
        None
    };

    Ok(SkinPackage {
        id: skin_id.to_string(),
        name,
        source_path: source.to_string_lossy().to_string(),
        storage_path: storage.to_string_lossy().to_string(),
        imported_at: imported_at(),
        has_inf,
        is_complete,
        cursor_count: cursor_files.len(),
        is_applied: false,
        import_note,
        roles,
        unassigned_files,
    })
}

fn parse_inf(
    inf_path: &Path,
    cursor_files: &[PathBuf],
) -> Result<HashMap<String, PathBuf>, String> {
    let content = read_text_lossy(inf_path).map_err(to_string)?;
    let strings = inf_strings(&content);

    let mut matches = HashMap::new();
    let mut ordered_candidates = Vec::<PathBuf>::new();
    let mut section = String::new();

    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.starts_with('[') && line.ends_with(']') {
            section = line.trim_matches(&['[', ']'][..]).to_ascii_lowercase();
            continue;
        }

        let expanded = expand_inf_strings(raw_line, &strings);
        let lower = expanded.to_ascii_lowercase();
        if !lower.contains(".cur") && !lower.contains(".ani") {
            continue;
        }

        for def in ROLE_DEFS {
            if line_mentions_windows_key(&lower, def.windows_key) {
                if let Some(path) = find_cursor_from_line(&expanded, cursor_files) {
                    matches.insert(def.windows_key.to_string(), path);
                }
            }
        }

        let files_from_line = find_cursors_from_line(&expanded, cursor_files);
        let uses_ordered_scheme = section == "scheme.cur"
            || ((section == "scheme.reg"
                || lower.contains("control panel\\cursors\\schemes")
                || lower.contains("control panel/cursors/schemes"))
                && files_from_line.len() >= 2);
        if uses_ordered_scheme {
            ordered_candidates.extend(files_from_line);
        }
    }

    let mut ordered_index = 0;
    for def in ROLE_DEFS {
        if matches.contains_key(def.windows_key) {
            continue;
        }
        while ordered_index < ordered_candidates.len() {
            let candidate = ordered_candidates[ordered_index].clone();
            ordered_index += 1;
            if !matches
                .values()
                .any(|matched| same_path(matched, &candidate))
            {
                matches.insert(def.windows_key.to_string(), candidate);
                break;
            }
        }
    }

    Ok(matches)
}

fn inf_strings(content: &str) -> HashMap<String, String> {
    let mut strings = HashMap::<String, String>::new();
    let mut section = String::new();

    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.starts_with('[') && line.ends_with(']') {
            section = line.trim_matches(&['[', ']'][..]).to_ascii_lowercase();
            continue;
        }
        if section == "strings" {
            if let Some((key, value)) = line.split_once('=') {
                strings.insert(key.trim().to_ascii_lowercase(), clean_inf_value(value));
            }
        }
    }

    strings
}

fn match_by_filename(path: &Path) -> Option<&'static str> {
    let file_name = path.file_stem()?.to_string_lossy().to_ascii_lowercase();
    let normalized = file_name.replace(['-', ' ', '.'], "_");

    ROLE_DEFS
        .iter()
        .find(|def| {
            def.keywords
                .iter()
                .any(|keyword| normalized.contains(keyword))
        })
        .map(|def| def.windows_key)
}

fn find_cursor_from_line(line: &str, cursor_files: &[PathBuf]) -> Option<PathBuf> {
    find_cursors_from_line(line, cursor_files)
        .into_iter()
        .next()
}

fn find_cursors_from_line(line: &str, cursor_files: &[PathBuf]) -> Vec<PathBuf> {
    let lower = line.to_ascii_lowercase().replace('\\', "/");
    let mut found = cursor_files
        .iter()
        .filter_map(|path| {
            let name = path.file_name()?.to_string_lossy().to_ascii_lowercase();
            lower.find(&name).map(|index| (index, path.clone()))
        })
        .collect::<Vec<_>>();
    found.sort_by_key(|(index, _)| *index);
    found.into_iter().map(|(_, path)| path).collect()
}

fn line_mentions_windows_key(line: &str, windows_key: &str) -> bool {
    let key = windows_key.to_ascii_lowercase();
    line.split(|ch: char| !ch.is_ascii_alphanumeric())
        .any(|token| token == key)
}

fn collect_files(root: &Path, extensions: &[&str]) -> io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    collect_files_inner(root, extensions, &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_files_inner(
    path: &Path,
    extensions: &[&str],
    files: &mut Vec<PathBuf>,
) -> io::Result<()> {
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_files_inner(&path, extensions, files)?;
        } else if path
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|ext| {
                extensions
                    .iter()
                    .any(|expected| ext.eq_ignore_ascii_case(expected))
            })
        {
            files.push(path);
        }
    }
    Ok(())
}

fn copy_dir(source: &Path, destination: &Path) -> io::Result<()> {
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if source_path.is_dir() {
            fs::create_dir_all(&destination_path)?;
            copy_dir(&source_path, &destination_path)?;
        } else {
            fs::copy(source_path, destination_path)?;
        }
    }
    Ok(())
}

fn copy_inf_package(source: &Path, destination: &Path) -> io::Result<()> {
    let parent = source.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "cannot read .inf parent directory",
        )
    })?;
    let file_name = source
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "cannot read .inf file name"))?;

    fs::copy(source, destination.join(file_name))?;

    let cursor_files = collect_files(parent, &["cur", "ani"])?;
    let referenced_files = cursor_files_referenced_by_inf(source, &cursor_files)?;
    for file in referenced_files {
        copy_file_preserving_base(&file, parent, destination)?;
    }

    Ok(())
}

fn copy_file_preserving_base(source: &Path, base: &Path, destination: &Path) -> io::Result<()> {
    let relative = source.strip_prefix(base).unwrap_or(source);
    let output = destination.join(relative);
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(source, output)?;
    Ok(())
}

fn cursor_files_referenced_by_inf(
    inf_path: &Path,
    cursor_files: &[PathBuf],
) -> io::Result<Vec<PathBuf>> {
    let content = read_text_lossy(inf_path)?;
    let strings = inf_strings(&content);
    let mut files = Vec::<PathBuf>::new();

    for raw_line in content.lines() {
        let expanded = expand_inf_strings(raw_line, &strings);
        let lower = expanded.to_ascii_lowercase();
        if !lower.contains(".cur") && !lower.contains(".ani") {
            continue;
        }

        for file in find_cursors_from_line(&expanded, cursor_files) {
            if !files.iter().any(|existing| same_path(existing, &file)) {
                files.push(file);
            }
        }
    }

    Ok(files)
}

fn extract_zip(source: &Path, destination: &Path) -> io::Result<()> {
    let file = fs::File::open(source)?;
    let mut archive = zip::ZipArchive::new(file).map_err(zip_to_io)?;

    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(zip_to_io)?;
        let Some(safe_name) = entry.enclosed_name().map(|value| value.to_owned()) else {
            continue;
        };
        let output_path = destination.join(safe_name);
        if entry.is_dir() {
            fs::create_dir_all(&output_path)?;
        } else {
            if let Some(parent) = output_path.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut output = fs::File::create(output_path)?;
            io::copy(&mut entry, &mut output)?;
        }
    }

    Ok(())
}

fn read_library(app: &AppHandle) -> Result<Vec<SkinPackage>, String> {
    migrate_legacy_app_data(app)?;
    let path = library_path(app)?;
    let mut library = read_library_with_recovery(app, &path)?;
    let mut repaired_inf_skin = false;
    for skin in &mut library {
        let needs_inf_repair =
            skin.has_inf && skin.roles.iter().all(|role| role.file_path.is_none());
        if needs_inf_repair {
            let source = PathBuf::from(&skin.source_path);
            let storage = PathBuf::from(&skin.storage_path);
            if let Ok(mut repaired) = analyze_skin(&skin.id, &source, &storage) {
                if repaired.roles.iter().any(|role| role.file_path.is_some()) {
                    repaired.imported_at = skin.imported_at.clone();
                    repaired.is_applied = skin.is_applied;
                    *skin = repaired;
                    repaired_inf_skin = true;
                }
            }
        }
        refresh_skin_file_state(skin);
        ensure_skin_preview_paths(skin);
        hydrate_skin_previews(skin);
    }
    if repaired_inf_skin {
        write_library(app, &library)?;
    }
    sync_applied_state(&mut library);
    Ok(library)
}

fn write_library(app: &AppHandle, library: &[SkinPackage]) -> Result<(), String> {
    let path = library_path(app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(to_string)?;
    }
    let mut stored_library = library.to_vec();
    for skin in &mut stored_library {
        clear_skin_preview_data(skin);
    }
    let content = serde_json::to_string_pretty(&stored_library).map_err(to_string)?;
    write_file_with_backup(&path, content.as_bytes()).map_err(to_string)
}

fn read_app_settings(app: &AppHandle) -> Result<AppSettings, String> {
    let path = settings_path(app)?;
    let backup = path.with_file_name("settings.bak.json");

    if path.exists() {
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(settings) = serde_json::from_str(&content) {
                return Ok(settings);
            }
        }
    }

    if backup.exists() {
        if let Ok(content) = fs::read_to_string(&backup) {
            if let Ok(settings) = serde_json::from_str(&content) {
                preserve_corrupt_file(&path, "settings.corrupt").map_err(to_string)?;
                fs::copy(&backup, &path).map_err(to_string)?;
                return Ok(settings);
            }
        }
    }

    preserve_corrupt_file(&path, "settings.corrupt").map_err(to_string)?;
    preserve_corrupt_file(&backup, "settings.backup-corrupt").map_err(to_string)?;
    let settings = AppSettings::default();
    write_app_settings(app, &settings)?;
    Ok(settings)
}

fn write_app_settings(app: &AppHandle, settings: &AppSettings) -> Result<(), String> {
    let path = settings_path(app)?;
    let backup = path.with_file_name("settings.bak.json");
    let content = serde_json::to_string_pretty(settings).map_err(to_string)?;
    write_file_with_named_backup(&path, &backup, content.as_bytes()).map_err(to_string)
}

fn read_library_with_recovery(app: &AppHandle, path: &Path) -> Result<Vec<SkinPackage>, String> {
    let backup = library_backup_path(path);

    if path.exists() {
        match read_library_file(path) {
            Ok(library) => return Ok(library),
            Err(error) => append_log(app, &format!("library.json is invalid: {error}")),
        }
    } else if !backup.exists() {
        return Ok(Vec::new());
    }

    if backup.exists() {
        match read_library_file(&backup) {
            Ok(library) => {
                preserve_corrupt_file(path, "library.corrupt").map_err(to_string)?;
                fs::copy(&backup, path).map_err(to_string)?;
                append_log(app, "library.json restored from backup");
                return Ok(library);
            }
            Err(error) => append_log(app, &format!("library backup is invalid: {error}")),
        }
    }

    preserve_corrupt_file(path, "library.corrupt").map_err(to_string)?;
    preserve_corrupt_file(&backup, "library.backup-corrupt").map_err(to_string)?;
    write_file_with_backup(path, b"[]").map_err(to_string)?;
    append_log(
        app,
        "library files were invalid; created a new empty library",
    );
    Ok(Vec::new())
}

fn read_library_file(path: &Path) -> Result<Vec<SkinPackage>, String> {
    let content = fs::read_to_string(path).map_err(to_string)?;
    serde_json::from_str(&content).map_err(to_string)
}

fn library_backup_path(path: &Path) -> PathBuf {
    path.with_file_name("library.bak.json")
}

fn write_file_with_backup(path: &Path, content: &[u8]) -> io::Result<()> {
    let backup = library_backup_path(path);
    write_file_with_named_backup(path, &backup, content)
}

fn write_file_with_named_backup(path: &Path, backup: &Path, content: &[u8]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let temp = path.with_extension(format!("tmp-{}", new_id()));
    let mut file = fs::File::create(&temp)?;
    file.write_all(content)?;
    file.sync_all()?;
    drop(file);

    if path.exists() {
        fs::copy(path, backup)?;
        fs::remove_file(path)?;
    }

    if let Err(error) = fs::rename(&temp, path) {
        let _ = fs::remove_file(&temp);
        if backup.exists() && !path.exists() {
            let _ = fs::copy(backup, path);
        }
        return Err(error);
    }

    Ok(())
}

fn preserve_corrupt_file(path: &Path, label: &str) -> io::Result<Option<PathBuf>> {
    if !path.exists() {
        return Ok(None);
    }

    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let target = parent.join(format!("{label}-{}.json", new_id()));
    fs::rename(path, &target)?;
    Ok(Some(target))
}

fn library_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app_data_dir(app)?.join("library.json"))
}

fn settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app_data_dir(app)?.join("settings.json"))
}

fn log_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app_data_dir(app)?.join("app.log"))
}

fn app_data_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let path = fixed_app_data_dir()
        .or_else(|| app.path().app_data_dir().ok())
        .ok_or("无法读取应用数据目录。")?;
    fs::create_dir_all(&path).map_err(to_string)?;
    Ok(path)
}

fn fixed_app_data_dir() -> Option<PathBuf> {
    std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .map(|path| path.join("CursorSkinManager"))
}

fn migrate_legacy_app_data(app: &AppHandle) -> Result<(), String> {
    let Some(target_dir) = fixed_app_data_dir() else {
        return Ok(());
    };
    let legacy_dir = app.path().app_data_dir().map_err(to_string)?;
    if same_path(&legacy_dir, &target_dir) || !legacy_dir.exists() {
        return Ok(());
    }

    fs::create_dir_all(&target_dir).map_err(to_string)?;
    let legacy_library = legacy_dir.join("library.json");
    let target_library = target_dir.join("library.json");
    let legacy_skins = legacy_dir.join("skins");
    let target_skins = target_dir.join("skins");

    if legacy_skins.exists() && !target_skins.exists() {
        copy_dir(&legacy_skins, &target_skins).map_err(to_string)?;
    }

    if legacy_library.exists() && !target_library.exists() {
        let content = fs::read_to_string(&legacy_library).map_err(to_string)?;
        let mut library: Vec<SkinPackage> = serde_json::from_str(&content).map_err(to_string)?;
        for skin in &mut library {
            rewrite_legacy_paths(skin, &legacy_dir, &target_dir);
            refresh_skin_file_state(skin);
            ensure_skin_preview_paths(skin);
        }
        let content = serde_json::to_string_pretty(&library).map_err(to_string)?;
        write_file_with_backup(&target_library, content.as_bytes()).map_err(to_string)?;
    }

    Ok(())
}

fn rewrite_legacy_paths(skin: &mut SkinPackage, legacy_dir: &Path, target_dir: &Path) {
    skin.storage_path = rewrite_path_string(&skin.storage_path, legacy_dir, target_dir);
    for role in &mut skin.roles {
        if let Some(file_path) = &role.file_path {
            role.file_path = Some(rewrite_path_string(file_path, legacy_dir, target_dir));
        }
        if let Some(preview_path) = &role.preview_path {
            role.preview_path = Some(rewrite_path_string(preview_path, legacy_dir, target_dir));
        }
    }
    for file in &mut skin.unassigned_files {
        file.file_path = rewrite_path_string(&file.file_path, legacy_dir, target_dir);
        if let Some(preview_path) = &file.preview_path {
            file.preview_path = Some(rewrite_path_string(preview_path, legacy_dir, target_dir));
        }
    }
}

fn rewrite_path_string(value: &str, legacy_dir: &Path, target_dir: &Path) -> String {
    let path = PathBuf::from(value);
    if let Ok(relative) = path.strip_prefix(legacy_dir) {
        return target_dir.join(relative).to_string_lossy().to_string();
    }
    value.to_string()
}

fn infer_skin_name(source: &Path, inf_path: Option<&PathBuf>) -> String {
    if let Some(path) = inf_path {
        if let Ok(content) = read_text_lossy(path) {
            for line in content.lines() {
                let lower = line.to_ascii_lowercase();
                if (lower.contains("schemename") || lower.contains("displayname"))
                    && line.contains('=')
                {
                    if let Some((_, value)) = line.split_once('=') {
                        let cleaned = clean_inf_value(value);
                        if !cleaned.is_empty() {
                            return cleaned;
                        }
                    }
                }
            }
        }
    }

    source
        .file_stem()
        .or_else(|| source.file_name())
        .map(|value| value.to_string_lossy().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "未命名皮肤包".to_string())
}

fn clean_inf_value(value: &str) -> String {
    value
        .split(';')
        .next()
        .unwrap_or(value)
        .trim()
        .trim_matches('"')
        .to_string()
}

fn expand_inf_strings(line: &str, strings: &HashMap<String, String>) -> String {
    let mut expanded = String::with_capacity(line.len());
    let mut remainder = line;

    while let Some(start) = remainder.find('%') {
        expanded.push_str(&remainder[..start]);
        let after_start = &remainder[start + 1..];
        let Some(end) = after_start.find('%') else {
            expanded.push_str(&remainder[start..]);
            return expanded;
        };
        let token = &after_start[..end];
        if let Some(value) = strings.get(&token.to_ascii_lowercase()) {
            expanded.push_str(value);
        } else {
            expanded.push('%');
            expanded.push_str(token);
            expanded.push('%');
        }
        remainder = &after_start[end + 1..];
    }
    expanded.push_str(remainder);
    expanded
}

fn extension_is(path: &Path, extension: &str) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case(extension))
}

fn validate_cursor_file(path: &Path) -> Result<Vec<u8>, String> {
    if !path.exists() {
        return Err("所选光标文件不存在，请重新选择。".into());
    }
    let metadata = path
        .metadata()
        .map_err(|error| format!("无法读取所选光标文件：{error}"))?;
    if !metadata.is_file() {
        return Err("所选路径不是文件。".into());
    }
    if metadata.len() == 0 {
        return Err("所选光标文件为空，无法使用。".into());
    }
    if !extension_is(path, "cur") && !extension_is(path, "ani") {
        return Err("仅支持 .cur 和 .ani 光标文件。".into());
    }

    let bytes = fs::read(path).map_err(|error| format!("无法读取所选光标文件：{error}"))?;
    if extension_is(path, "cur") {
        if bytes.len() < 6
            || read_u16(&bytes, 0).unwrap_or(1) != 0
            || read_u16(&bytes, 2).unwrap_or(0) != 2
            || read_u16(&bytes, 4).unwrap_or(0) == 0
        {
            return Err("所选 .cur 文件结构无效或已经损坏。".into());
        }
    } else if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"ACON" {
        return Err("所选 .ani 文件结构无效或已经损坏。".into());
    }

    let preview = cursor_preview_png(path)
        .map_err(|error| format!("光标文件解析失败：{error}"))?
        .ok_or("无法从所选光标文件生成有效预览，文件可能已损坏或格式不受支持。")?;
    let image = image::load_from_memory(&preview)
        .map_err(|_| "生成的光标预览无效，文件可能已经损坏。")?
        .to_rgba8();
    if image.width() == 0 || image.height() == 0 || !image.pixels().any(|pixel| pixel[3] > 1) {
        return Err("所选光标文件没有可显示的有效图像。".into());
    }
    Ok(preview)
}

fn validated_skin_storage(app: &AppHandle, skin: &SkinPackage) -> Result<PathBuf, String> {
    let skins_root = app_data_dir(app)?.join("skins");
    fs::create_dir_all(&skins_root).map_err(to_string)?;
    let skins_root = skins_root.canonicalize().map_err(to_string)?;
    let storage_path = PathBuf::from(&skin.storage_path)
        .canonicalize()
        .map_err(|_| "当前皮肤的应用内部目录不存在。")?;
    if !storage_path.starts_with(&skins_root) {
        return Err("当前皮肤目录不在应用数据范围内。".into());
    }
    Ok(storage_path)
}

fn copy_cursor_to_internal_storage(source: &Path, storage: &Path) -> io::Result<PathBuf> {
    let destination_dir = storage.join("custom");
    fs::create_dir_all(&destination_dir)?;
    let original_name = source
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "missing file name"))?;
    let stem = source
        .file_stem()
        .map(|value| value.to_string_lossy().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "cursor".to_string());
    let extension = source
        .extension()
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_else(|| "cur".to_string());

    for index in 0..10_000usize {
        let candidate = if index == 0 {
            destination_dir.join(original_name)
        } else {
            destination_dir.join(format!("{stem}-{}.{extension}", index + 1))
        };
        let mut output = match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        };

        let copy_result = (|| -> io::Result<()> {
            let mut input = fs::File::open(source)?;
            io::copy(&mut input, &mut output)?;
            output.sync_all()
        })();
        if let Err(error) = copy_result {
            drop(output);
            let _ = fs::remove_file(&candidate);
            return Err(error);
        }
        return Ok(candidate);
    }

    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "too many files with the same name",
    ))
}

fn cursor_file_from_path(path: &Path, preview_path: Option<&Path>) -> CursorFile {
    CursorFile {
        file_path: path.to_string_lossy().to_string(),
        file_name: path
            .file_name()
            .map(|value| value.to_string_lossy().to_string())
            .unwrap_or_else(|| "cursor.cur".to_string()),
        preview_path: preview_path.map(|value| value.to_string_lossy().to_string()),
        preview_data_url: None,
        cursor_type: path
            .extension()
            .map(|value| value.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_else(|| "cur".to_string()),
        exists: path.exists(),
    }
}

fn replace_cursor_role_transaction<F>(
    library: &mut Vec<SkinPackage>,
    skin_index: usize,
    role_index: usize,
    source: &Path,
    storage_path: &Path,
    persist: F,
) -> Result<CursorMutation, String>
where
    F: FnOnce(&[SkinPackage]) -> Result<(), String>,
{
    let preview_png = validate_cursor_file(source)?;
    let original_library = library.clone();
    let destination = copy_cursor_to_internal_storage(source, storage_path)
        .map_err(|error| format!("无法将新光标复制到应用内部目录：{error}"))?;
    let preview_path =
        match write_cursor_preview_png(&destination, &storage_path.join(".previews"), &preview_png)
        {
            Ok(path) => path,
            Err(error) => {
                let _ = fs::remove_file(&destination);
                remove_dir_if_empty(&storage_path.join("custom"));
                return Err(format!("无法保存新光标预览：{error}"));
            }
        };

    let new_file = cursor_file_from_path(&destination, Some(&preview_path));
    let old_path = replace_role_mapping(&mut library[skin_index], role_index, new_file);
    refresh_skin_file_state(&mut library[skin_index]);
    recalculate_cursor_count(&mut library[skin_index]);
    sync_applied_state(library);
    library[skin_index].is_applied = false;

    if let Err(error) = persist(library) {
        *library = original_library;
        let _ = fs::remove_file(&destination);
        let _ = fs::remove_file(&preview_path);
        remove_dir_if_empty(&storage_path.join("custom"));
        return Err(error);
    }

    Ok(CursorMutation {
        old_path,
        new_path: destination.to_string_lossy().to_string(),
    })
}

fn assign_unassigned_cursor_transaction<F>(
    library: &mut Vec<SkinPackage>,
    skin_index: usize,
    role_index: usize,
    unassigned_index: usize,
    storage_path: &Path,
    persist: F,
) -> Result<CursorMutation, String>
where
    F: FnOnce(&[SkinPackage]) -> Result<(), String>,
{
    let original_library = library.clone();
    let mut new_file = library[skin_index]
        .unassigned_files
        .get(unassigned_index)
        .cloned()
        .ok_or("该文件已不在未分配列表中，请刷新后重试。")?;
    let source = PathBuf::from(&new_file.file_path);
    let preview_png = validate_cursor_file(&source)?;

    let mut created_preview = None;
    let preview_exists = new_file
        .preview_path
        .as_deref()
        .is_some_and(|path| Path::new(path).exists());
    if !preview_exists {
        let preview_dir = storage_path.join(".previews");
        let expected_preview = cursor_preview_output_path(&source, &preview_dir);
        let existed_before = expected_preview.exists();
        let preview_path = write_cursor_preview_png(&source, &preview_dir, &preview_png)
            .map_err(|error| format!("无法保存光标预览：{error}"))?;
        new_file.preview_path = Some(preview_path.to_string_lossy().to_string());
        if !existed_before {
            created_preview = Some(preview_path);
        }
    }
    new_file.preview_data_url = None;
    new_file.exists = true;
    library[skin_index]
        .unassigned_files
        .remove(unassigned_index);

    let new_path = new_file.file_path.clone();
    let old_path = replace_role_mapping(&mut library[skin_index], role_index, new_file);
    refresh_skin_file_state(&mut library[skin_index]);
    recalculate_cursor_count(&mut library[skin_index]);
    sync_applied_state(library);
    library[skin_index].is_applied = false;

    if let Err(error) = persist(library) {
        *library = original_library;
        if let Some(path) = created_preview {
            let _ = fs::remove_file(path);
        }
        return Err(error);
    }

    Ok(CursorMutation { old_path, new_path })
}

fn cursor_file_from_role(role: &CursorRole) -> Option<CursorFile> {
    let file_path = role.file_path.as_deref()?;
    let path = Path::new(file_path);
    Some(CursorFile {
        file_path: file_path.to_string(),
        file_name: role
            .file_name
            .clone()
            .or_else(|| {
                path.file_name()
                    .map(|value| value.to_string_lossy().to_string())
            })
            .unwrap_or_else(|| "cursor.cur".to_string()),
        preview_path: role.preview_path.clone(),
        preview_data_url: None,
        cursor_type: role.cursor_type.clone().unwrap_or_else(|| {
            path.extension()
                .map(|value| value.to_string_lossy().to_ascii_lowercase())
                .unwrap_or_else(|| "cur".to_string())
        }),
        exists: role.exists && path.exists(),
    })
}

fn replace_role_mapping(
    skin: &mut SkinPackage,
    role_index: usize,
    new_file: CursorFile,
) -> Option<String> {
    let old_file = cursor_file_from_role(&skin.roles[role_index]);
    let old_path = old_file.as_ref().map(|file| file.file_path.clone());

    let role = &mut skin.roles[role_index];
    role.file_path = Some(new_file.file_path.clone());
    role.file_name = Some(new_file.file_name.clone());
    role.preview_path = new_file.preview_path.clone();
    role.preview_data_url = None;
    role.cursor_type = Some(new_file.cursor_type.clone());
    role.exists = new_file.exists;

    let old_file = old_file?;
    let referenced_elsewhere = skin.roles.iter().enumerate().any(|(index, role)| {
        index != role_index
            && role
                .file_path
                .as_deref()
                .is_some_and(|path| stored_paths_match(path, &old_file.file_path))
    });
    let already_unassigned = skin
        .unassigned_files
        .iter()
        .any(|file| stored_paths_match(&file.file_path, &old_file.file_path));
    let replaced_with_same_file = stored_paths_match(&old_file.file_path, &new_file.file_path);
    if old_file.exists && !referenced_elsewhere && !already_unassigned && !replaced_with_same_file {
        skin.unassigned_files.push(old_file);
    }
    old_path
}

fn stored_paths_match(left: &str, right: &str) -> bool {
    if left.eq_ignore_ascii_case(right) {
        return true;
    }
    let left_path = Path::new(left);
    let right_path = Path::new(right);
    left_path.exists() && right_path.exists() && same_path(left_path, right_path)
}

fn recalculate_cursor_count(skin: &mut SkinPackage) {
    if let Ok(files) = collect_files(Path::new(&skin.storage_path), &["cur", "ani"]) {
        skin.cursor_count = files.len();
    }
}

fn hydrate_library_previews(library: &mut [SkinPackage]) {
    for skin in library {
        refresh_skin_file_state(skin);
        ensure_skin_preview_paths(skin);
        hydrate_skin_previews(skin);
    }
}

fn cleanup_unused_preview_cache(skin: &SkinPackage) -> io::Result<()> {
    let preview_dir = PathBuf::from(&skin.storage_path).join(".previews");
    if !preview_dir.exists() {
        return Ok(());
    }
    let used = skin
        .roles
        .iter()
        .filter_map(|role| role.preview_path.as_deref())
        .chain(
            skin.unassigned_files
                .iter()
                .filter_map(|file| file.preview_path.as_deref()),
        )
        .map(PathBuf::from)
        .collect::<HashSet<_>>();

    for entry in fs::read_dir(&preview_dir)? {
        let path = entry?.path();
        let is_preview_cache = path
            .file_name()
            .and_then(|value| value.to_str())
            .is_some_and(|value| value.ends_with(PREVIEW_CACHE_SUFFIX));
        if path.is_file() && is_preview_cache && !used.contains(&path) {
            fs::remove_file(path)?;
        }
    }
    Ok(())
}

fn remove_dir_if_empty(path: &Path) {
    if path
        .read_dir()
        .ok()
        .is_some_and(|mut entries| entries.next().is_none())
    {
        let _ = fs::remove_dir(path);
    }
}

fn refresh_skin_file_state(skin: &mut SkinPackage) {
    for role in &mut skin.roles {
        role.exists = role
            .file_path
            .as_ref()
            .is_some_and(|file_path| Path::new(file_path).exists());
        if let Some(preview_path) = &role.preview_path {
            if !Path::new(preview_path).exists() {
                role.preview_path = None;
            }
        }
    }
    for file in &mut skin.unassigned_files {
        file.exists = Path::new(&file.file_path).exists();
        if let Some(preview_path) = &file.preview_path {
            if !Path::new(preview_path).exists() {
                file.preview_path = None;
            }
        }
    }
    skin.is_complete = skin.roles.iter().all(|role| role.exists);
    let assigned_count = skin
        .roles
        .iter()
        .filter(|role| role.file_path.is_some())
        .count();
    if skin.has_inf && assigned_count > 0 && assigned_count < ROLE_DEFS.len() {
        skin.import_note = Some(format!(
            "安装配置已匹配 {assigned_count} / {} 个光标角色，未设置角色应用时保持当前系统配置。",
            ROLE_DEFS.len()
        ));
    } else if skin.has_inf && assigned_count == ROLE_DEFS.len() {
        skin.import_note = None;
    }
}

fn ensure_skin_preview_paths(skin: &mut SkinPackage) {
    let preview_dir = PathBuf::from(&skin.storage_path).join(".previews");
    for role in &mut skin.roles {
        let needs_refresh = role
            .preview_path
            .as_deref()
            .is_none_or(|path| !path.ends_with(PREVIEW_CACHE_SUFFIX));
        if role.exists && needs_refresh {
            if let Some(file_path) = &role.file_path {
                role.preview_path = build_cursor_preview(Path::new(file_path), &preview_dir)
                    .ok()
                    .flatten()
                    .map(|path| path.to_string_lossy().to_string());
            }
        }
    }
    for file in &mut skin.unassigned_files {
        let needs_refresh = file
            .preview_path
            .as_deref()
            .is_none_or(|path| !path.ends_with(PREVIEW_CACHE_SUFFIX));
        if file.exists && needs_refresh {
            file.preview_path = build_cursor_preview(Path::new(&file.file_path), &preview_dir)
                .ok()
                .flatten()
                .map(|path| path.to_string_lossy().to_string());
        }
    }
}

fn hydrate_skin_previews(skin: &mut SkinPackage) {
    for role in &mut skin.roles {
        role.preview_data_url = role
            .preview_path
            .as_deref()
            .and_then(|path| preview_data_url(Path::new(path)).ok().flatten());
    }
    for file in &mut skin.unassigned_files {
        file.preview_data_url = file
            .preview_path
            .as_deref()
            .and_then(|path| preview_data_url(Path::new(path)).ok().flatten());
    }
}

fn clear_skin_preview_data(skin: &mut SkinPackage) {
    for role in &mut skin.roles {
        role.preview_data_url = None;
    }
    for file in &mut skin.unassigned_files {
        file.preview_data_url = None;
    }
}

fn preview_data_url(path: &Path) -> io::Result<Option<String>> {
    if !path.exists() {
        return Ok(None);
    }
    let bytes = fs::read(path)?;
    let encoded = general_purpose::STANDARD.encode(bytes);
    Ok(Some(format!("data:image/png;base64,{encoded}")))
}

fn run_logged<T, F>(app: &AppHandle, action: &str, f: F) -> Result<T, String>
where
    F: FnOnce(&AppHandle) -> Result<T, String>,
{
    append_log(app, &format!("{action} started"));
    match f(app) {
        Ok(value) => {
            append_log(app, &format!("{action} succeeded"));
            Ok(value)
        }
        Err(error) => {
            append_log(app, &format!("{action} failed: {error}"));
            Err(error)
        }
    }
}

fn append_log(app: &AppHandle, message: &str) {
    let Ok(path) = log_path(app) else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "[{}] {}", imported_at(), message);
    }
}

#[cfg(target_os = "windows")]
fn sync_applied_state(library: &mut [SkinPackage]) {
    let Some(current) = current_windows_cursor_values() else {
        return;
    };

    let applied_id = library
        .iter()
        .find(|skin| skin_matches_current_cursors(skin, &current))
        .map(|skin| skin.id.clone());

    for skin in library {
        skin.is_applied = applied_id.as_ref().is_some_and(|id| id == &skin.id);
    }
}

#[cfg(not(target_os = "windows"))]
fn sync_applied_state(_library: &mut [SkinPackage]) {}

#[cfg(target_os = "windows")]
fn current_windows_cursor_values() -> Option<HashMap<String, String>> {
    use winreg::{enums::HKEY_CURRENT_USER, RegKey};

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let cursors = hkcu
        .open_subkey_with_flags("Control Panel\\Cursors", winreg::enums::KEY_READ)
        .ok()?;

    let mut values = HashMap::new();
    for def in ROLE_DEFS {
        let value: String = cursors.get_value(def.windows_key).unwrap_or_default();
        values.insert(def.windows_key.to_string(), value);
    }
    Some(values)
}

#[cfg(target_os = "windows")]
fn skin_matches_current_cursors(skin: &SkinPackage, current: &HashMap<String, String>) -> bool {
    let assigned_roles = skin
        .roles
        .iter()
        .filter(|role| role.file_path.is_some())
        .collect::<Vec<_>>();

    !assigned_roles.is_empty()
        && assigned_roles.iter().all(|role| {
            let Some(expected) = role.file_path.as_deref() else {
                return false;
            };
            if !role.exists {
                return false;
            }
            let Some(actual) = current.get(&role.windows_key) else {
                return false;
            };
            paths_match(actual, expected)
        })
}

#[cfg(target_os = "windows")]
fn paths_match(actual: &str, expected: &str) -> bool {
    if actual.eq_ignore_ascii_case(expected) {
        return true;
    }

    let actual_path = Path::new(actual);
    let expected_path = Path::new(expected);
    if actual_path.exists() && expected_path.exists() {
        return same_path(actual_path, expected_path);
    }

    false
}

fn source_path_matches(stored: &str, source: &Path) -> bool {
    let stored_path = Path::new(stored);
    if stored_path.exists() && source.exists() {
        return same_path(stored_path, source);
    }

    stored.eq_ignore_ascii_case(&source.to_string_lossy())
}

fn same_path(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(left), Ok(right)) => left == right,
        _ => a == b,
    }
}

fn build_cursor_preview(cursor_path: &Path, preview_dir: &Path) -> io::Result<Option<PathBuf>> {
    if !extension_is(cursor_path, "cur") && !extension_is(cursor_path, "ani") {
        return Ok(None);
    }
    let Some(png) = cursor_preview_png(cursor_path)? else {
        return Ok(None);
    };
    write_cursor_preview_png(cursor_path, preview_dir, &png).map(Some)
}

fn cursor_preview_png(cursor_path: &Path) -> io::Result<Option<Vec<u8>>> {
    let png = if extension_is(cursor_path, "ani") {
        extract_preview_from_ani(cursor_path)?
    } else {
        extract_preview_from_cur(cursor_path)?
    };
    let Some(png) = png else {
        return Ok(None);
    };
    center_preview_png(&png).map(Some)
}

fn write_cursor_preview_png(
    cursor_path: &Path,
    preview_dir: &Path,
    png: &[u8],
) -> io::Result<PathBuf> {
    fs::create_dir_all(preview_dir)?;
    let output = cursor_preview_output_path(cursor_path, preview_dir);
    fs::write(&output, png)?;
    Ok(output)
}

fn cursor_preview_output_path(cursor_path: &Path, preview_dir: &Path) -> PathBuf {
    let stem = cursor_path
        .file_name()
        .map(|value| {
            value
                .to_string_lossy()
                .replace(['\\', '/', ':', '*', '?', '"', '<', '>', '|'], "_")
        })
        .unwrap_or_else(|| "cursor".to_string());
    let path_hash = stable_path_hash(cursor_path);
    preview_dir.join(format!("{stem}-{path_hash:016x}{PREVIEW_CACHE_SUFFIX}"))
}

fn stable_path_hash(path: &Path) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in path.to_string_lossy().as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn center_preview_png(png: &[u8]) -> io::Result<Vec<u8>> {
    let source = image::load_from_memory(png)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?
        .to_rgba8();
    let (width, height) = source.dimensions();
    let mut min_x = width;
    let mut min_y = height;
    let mut max_x = 0;
    let mut max_y = 0;
    let mut visible = false;

    for (x, y, pixel) in source.enumerate_pixels() {
        if pixel[3] > 1 {
            visible = true;
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
        }
    }

    if !visible {
        return Ok(png.to_vec());
    }

    let visible_width = max_x - min_x + 1;
    let visible_height = max_y - min_y + 1;
    let visible_size = visible_width.max(visible_height);
    let padding = ((visible_size as f32 * 0.12).ceil() as u32).max(1);
    let canvas_size = visible_size + padding * 2;
    let cropped =
        image::imageops::crop_imm(&source, min_x, min_y, visible_width, visible_height).to_image();
    let mut canvas = image::RgbaImage::new(canvas_size, canvas_size);
    let offset_x = (canvas_size - visible_width) / 2;
    let offset_y = (canvas_size - visible_height) / 2;
    image::imageops::replace(
        &mut canvas,
        &cropped,
        i64::from(offset_x),
        i64::from(offset_y),
    );

    let mut output = Vec::new();
    PngEncoder::new(&mut output)
        .write_image(
            canvas.as_raw(),
            canvas_size,
            canvas_size,
            ColorType::Rgba8.into(),
        )
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    Ok(output)
}

fn extract_preview_from_cur(cursor_path: &Path) -> io::Result<Option<Vec<u8>>> {
    let bytes = fs::read(cursor_path)?;
    extract_preview_from_icon_bytes(&bytes)
}

fn extract_preview_from_ani(cursor_path: &Path) -> io::Result<Option<Vec<u8>>> {
    let bytes = fs::read(cursor_path)?;
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"ACON" {
        return Ok(None);
    }

    let mut offset = 12;
    while offset + 8 <= bytes.len() {
        let chunk_id = &bytes[offset..offset + 4];
        let chunk_size = read_u32(&bytes, offset + 4)? as usize;
        let chunk_start = offset + 8;
        let chunk_end = chunk_start.saturating_add(chunk_size);
        if chunk_end > bytes.len() {
            break;
        }

        if chunk_id == b"icon" {
            if let Some(png) = extract_preview_from_icon_bytes(&bytes[chunk_start..chunk_end])? {
                return Ok(Some(png));
            }
        } else if chunk_id == b"LIST" && chunk_size >= 4 {
            if let Some(png) = extract_preview_from_ani_list(&bytes[chunk_start..chunk_end])? {
                return Ok(Some(png));
            }
        }

        offset = chunk_end + (chunk_size % 2);
    }

    Ok(None)
}

fn extract_preview_from_ani_list(list: &[u8]) -> io::Result<Option<Vec<u8>>> {
    if list.len() < 4 {
        return Ok(None);
    }

    let mut offset = 4;
    while offset + 8 <= list.len() {
        let chunk_id = &list[offset..offset + 4];
        let chunk_size = read_u32(list, offset + 4)? as usize;
        let chunk_start = offset + 8;
        let chunk_end = chunk_start.saturating_add(chunk_size);
        if chunk_end > list.len() {
            break;
        }

        if chunk_id == b"icon" {
            if let Some(png) = extract_preview_from_icon_bytes(&list[chunk_start..chunk_end])? {
                return Ok(Some(png));
            }
        }

        offset = chunk_end + (chunk_size % 2);
    }

    Ok(None)
}

fn extract_preview_from_icon_bytes(bytes: &[u8]) -> io::Result<Option<Vec<u8>>> {
    if bytes.len() < 22 {
        return Ok(None);
    }
    let reserved = u16::from_le_bytes([bytes[0], bytes[1]]);
    let image_type = u16::from_le_bytes([bytes[2], bytes[3]]);
    let count = u16::from_le_bytes([bytes[4], bytes[5]]) as usize;
    if reserved != 0 || !matches!(image_type, 1 | 2) || count == 0 {
        return Ok(None);
    }

    let mut entries = Vec::new();
    for index in 0..count {
        let entry = 6 + index * 16;
        if entry + 16 > bytes.len() {
            break;
        }
        let width = if bytes[entry] == 0 {
            256
        } else {
            bytes[entry] as usize
        };
        let height = if bytes[entry + 1] == 0 {
            256
        } else {
            bytes[entry + 1] as usize
        };
        let bit_count = u16::from_le_bytes([bytes[entry + 6], bytes[entry + 7]]);
        let size = u32::from_le_bytes([
            bytes[entry + 8],
            bytes[entry + 9],
            bytes[entry + 10],
            bytes[entry + 11],
        ]) as usize;
        let offset = u32::from_le_bytes([
            bytes[entry + 12],
            bytes[entry + 13],
            bytes[entry + 14],
            bytes[entry + 15],
        ]) as usize;
        if offset + size > bytes.len() {
            continue;
        }
        entries.push((width * height, bit_count, offset, size));
    }

    entries.sort_by(|left, right| right.0.cmp(&left.0).then(right.1.cmp(&left.1)));

    for (_, _, offset, size) in entries {
        let image = &bytes[offset..offset + size];
        if image.starts_with(b"\x89PNG\r\n\x1a\n") {
            return Ok(Some(image.to_vec()));
        }
        if let Some(png) = dib_to_png(image)? {
            return Ok(Some(png));
        }
    }

    Ok(None)
}

fn dib_to_png(dib: &[u8]) -> io::Result<Option<Vec<u8>>> {
    if dib.len() < 40 {
        return Ok(None);
    }

    let header_size = read_u32(dib, 0)? as usize;
    if header_size < 40 || header_size > dib.len() {
        return Ok(None);
    }

    let width = read_i32(dib, 4)?.unsigned_abs() as usize;
    let raw_height = read_i32(dib, 8)?;
    let stored_height = raw_height.unsigned_abs() as usize;
    let height = if stored_height >= 2 {
        stored_height / 2
    } else {
        stored_height
    };
    let bit_count = read_u16(dib, 14)?;
    let compression = read_u32(dib, 16)?;
    let colors_used = read_u32(dib, 32).unwrap_or(0) as usize;

    if width == 0 || height == 0 || compression != 0 || !matches!(bit_count, 1 | 4 | 8 | 24 | 32) {
        return Ok(None);
    }

    let palette_len = if bit_count <= 8 {
        let maximum = 1usize << bit_count;
        if colors_used > 0 {
            colors_used.min(maximum)
        } else {
            maximum
        }
    } else {
        0
    };
    let palette_size = palette_len.checked_mul(4).ok_or_else(invalid_data)?;
    let pixel_offset = header_size
        .checked_add(palette_size)
        .ok_or_else(invalid_data)?;
    let stride = (width * bit_count as usize).div_ceil(32) * 4;
    let xor_size = stride.checked_mul(height).ok_or_else(invalid_data)?;
    if pixel_offset + xor_size > dib.len() {
        return Ok(None);
    }

    let mask_offset = pixel_offset + xor_size;
    let mask_stride = width.div_ceil(32) * 4;
    let has_mask = mask_offset + mask_stride * height <= dib.len();
    let bottom_up = raw_height > 0;
    let mut rgba = vec![0u8; width * height * 4];
    let mut has_alpha = false;

    for y in 0..height {
        let src_y = if bottom_up { height - 1 - y } else { y };
        let row = pixel_offset + src_y * stride;
        for x in 0..width {
            let dest = (y * width + x) * 4;
            match bit_count {
                1 | 4 | 8 => {
                    let palette_index = indexed_pixel(dib, row, x, bit_count)?;
                    if palette_index >= palette_len {
                        return Ok(None);
                    }
                    let palette_entry = header_size + palette_index * 4;
                    if palette_entry + 3 >= dib.len() {
                        return Ok(None);
                    }
                    rgba[dest] = dib[palette_entry + 2];
                    rgba[dest + 1] = dib[palette_entry + 1];
                    rgba[dest + 2] = dib[palette_entry];
                    rgba[dest + 3] = 255;
                }
                32 => {
                    let src = row + x * 4;
                    if src + 3 >= dib.len() {
                        return Ok(None);
                    }
                    rgba[dest] = dib[src + 2];
                    rgba[dest + 1] = dib[src + 1];
                    rgba[dest + 2] = dib[src];
                    rgba[dest + 3] = dib[src + 3];
                    has_alpha |= dib[src + 3] != 0;
                }
                24 => {
                    let src = row + x * 3;
                    if src + 2 >= dib.len() {
                        return Ok(None);
                    }
                    rgba[dest] = dib[src + 2];
                    rgba[dest + 1] = dib[src + 1];
                    rgba[dest + 2] = dib[src];
                    rgba[dest + 3] = 255;
                }
                _ => return Ok(None),
            }
        }
    }

    if bit_count == 32 && !has_alpha {
        for pixel in rgba.chunks_exact_mut(4) {
            pixel[3] = 255;
        }
    }

    if has_mask {
        for y in 0..height {
            let src_y = if bottom_up { height - 1 - y } else { y };
            let row = mask_offset + src_y * mask_stride;
            for x in 0..width {
                let byte = dib[row + x / 8];
                let bit = 7 - (x % 8);
                if ((byte >> bit) & 1) == 1 {
                    rgba[(y * width + x) * 4 + 3] = 0;
                }
            }
        }
    }

    let mut output = Vec::new();
    PngEncoder::new(&mut output)
        .write_image(&rgba, width as u32, height as u32, ColorType::Rgba8.into())
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    Ok(Some(output))
}

fn indexed_pixel(dib: &[u8], row: usize, x: usize, bit_count: u16) -> io::Result<usize> {
    let index = match bit_count {
        8 => *dib.get(row + x).ok_or_else(invalid_data)? as usize,
        4 => {
            let byte = *dib.get(row + x / 2).ok_or_else(invalid_data)?;
            if x.is_multiple_of(2) {
                (byte >> 4) as usize
            } else {
                (byte & 0x0f) as usize
            }
        }
        1 => {
            let byte = *dib.get(row + x / 8).ok_or_else(invalid_data)?;
            ((byte >> (7 - (x % 8))) & 1) as usize
        }
        _ => return Err(invalid_data()),
    };
    Ok(index)
}

fn read_u16(bytes: &[u8], offset: usize) -> io::Result<u16> {
    let Some(slice) = bytes.get(offset..offset + 2) else {
        return Err(invalid_data());
    };
    Ok(u16::from_le_bytes([slice[0], slice[1]]))
}

fn read_u32(bytes: &[u8], offset: usize) -> io::Result<u32> {
    let Some(slice) = bytes.get(offset..offset + 4) else {
        return Err(invalid_data());
    };
    Ok(u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

fn read_i32(bytes: &[u8], offset: usize) -> io::Result<i32> {
    let Some(slice) = bytes.get(offset..offset + 4) else {
        return Err(invalid_data());
    };
    Ok(i32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

fn invalid_data() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, "invalid cursor image data")
}

fn read_text_lossy(path: &Path) -> io::Result<String> {
    let bytes = fs::read(path)?;
    if bytes.starts_with(&[0xff, 0xfe]) {
        let units: Vec<u16> = bytes[2..]
            .chunks_exact(2)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
            .collect();
        return Ok(String::from_utf16_lossy(&units));
    }

    if bytes.starts_with(&[0xfe, 0xff]) {
        let units: Vec<u16> = bytes[2..]
            .chunks_exact(2)
            .map(|chunk| u16::from_be_bytes([chunk[0], chunk[1]]))
            .collect();
        return Ok(String::from_utf16_lossy(&units));
    }

    if let Ok(content) = String::from_utf8(bytes.clone()) {
        return Ok(content);
    }

    let (decoded, _, _) = encoding_rs::GBK.decode(&bytes);
    Ok(decoded.into_owned())
}

fn imported_at() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_string())
}

fn new_id() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default();
    let sequence = ID_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("skin-{}-{}-{}", millis, std::process::id(), sequence)
}

fn zip_to_io(error: zip::result::ZipError) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, error)
}

fn to_string<E: std::fmt::Display>(error: E) -> String {
    error.to_string()
}

#[cfg(target_os = "windows")]
fn apply_to_windows_current_user(skin: &SkinPackage) -> Result<(), String> {
    use winreg::{enums::HKEY_CURRENT_USER, RegKey};

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let cursors = hkcu
        .open_subkey_with_flags("Control Panel\\Cursors", winreg::enums::KEY_SET_VALUE)
        .map_err(to_string)?;

    for role in skin.roles.iter().filter(|role| role.exists) {
        if let Some(file_path) = &role.file_path {
            cursors
                .set_value(&role.windows_key, file_path)
                .map_err(to_string)?;
        }
    }

    drop(cursors);
    let _ = try_refresh_windows_cursors();

    Ok(())
}

#[cfg(target_os = "windows")]
fn reset_windows_default_cursors() -> Result<(), String> {
    use winreg::{enums::HKEY_CURRENT_USER, RegKey};

    let values = windows_aero_cursor_values();
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let cursors = hkcu
        .open_subkey_with_flags("Control Panel\\Cursors", winreg::enums::KEY_SET_VALUE)
        .map_err(to_string)?;

    cursors.set_value("", &"Windows Aero").map_err(to_string)?;
    for (key, value) in values {
        cursors.set_value(key, &value).map_err(to_string)?;
    }
    cursors
        .set_value("Scheme Source", &2u32)
        .map_err(to_string)?;

    drop(cursors);
    let _ = try_refresh_windows_cursors();

    Ok(())
}

#[cfg(target_os = "windows")]
fn windows_aero_cursor_values() -> Vec<(&'static str, String)> {
    let keys = [
        "Arrow",
        "Help",
        "AppStarting",
        "Wait",
        "Crosshair",
        "IBeam",
        "NWPen",
        "No",
        "SizeNS",
        "SizeWE",
        "SizeNWSE",
        "SizeNESW",
        "SizeAll",
        "UpArrow",
        "Hand",
    ];

    if let Some(values) = read_windows_aero_scheme_from_registry() {
        return keys
            .iter()
            .enumerate()
            .map(|(index, key)| (*key, values.get(index).cloned().unwrap_or_default()))
            .collect();
    }

    let cursor_dir = std::env::var("SystemRoot")
        .map(|root| PathBuf::from(root).join("Cursors"))
        .unwrap_or_else(|_| PathBuf::from(r"C:\Windows\Cursors"));

    [
        ("Arrow", "aero_arrow.cur"),
        ("Help", "aero_helpsel.cur"),
        ("AppStarting", "aero_working.ani"),
        ("Wait", "aero_busy.ani"),
        ("Crosshair", ""),
        ("IBeam", ""),
        ("NWPen", "aero_pen.cur"),
        ("No", "aero_unavail.cur"),
        ("SizeNS", "aero_ns.cur"),
        ("SizeWE", "aero_ew.cur"),
        ("SizeNWSE", "aero_nwse.cur"),
        ("SizeNESW", "aero_nesw.cur"),
        ("SizeAll", "aero_move.cur"),
        ("UpArrow", "aero_up.cur"),
        ("Hand", "aero_link.cur"),
    ]
    .into_iter()
    .map(|(key, file_name)| {
        let value = if file_name.is_empty() {
            String::new()
        } else {
            cursor_dir.join(file_name).to_string_lossy().to_string()
        };
        (key, value)
    })
    .collect()
}

#[cfg(target_os = "windows")]
fn read_windows_aero_scheme_from_registry() -> Option<Vec<String>> {
    use winreg::{enums::HKEY_LOCAL_MACHINE, RegKey};

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let schemes = hklm
        .open_subkey_with_flags(
            "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Control Panel\\Cursors\\Schemes",
            winreg::enums::KEY_READ,
        )
        .ok()?;
    let scheme: String = schemes.get_value("Windows Aero").ok()?;
    Some(
        scheme
            .split(',')
            .map(|value| value.trim().to_string())
            .collect(),
    )
}

#[cfg(target_os = "windows")]
fn refresh_windows_cursors() -> Result<(), String> {
    if try_refresh_windows_cursors() {
        Ok(())
    } else {
        Err("Windows did not confirm the cursor refresh. The registry values may already be written; reopen Mouse Settings, sign out, or sign in again if the cursor does not update.".into())
    }
}

#[cfg(target_os = "windows")]
fn try_refresh_windows_cursors() -> bool {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Foundation::LPARAM;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        SendMessageTimeoutW, SystemParametersInfoW, HWND_BROADCAST, SMTO_ABORTIFHUNG,
        SPIF_SENDCHANGE, SPIF_UPDATEINIFILE, SPI_SETCURSORS, WM_SETTINGCHANGE,
    };

    let flags = SPIF_UPDATEINIFILE | SPIF_SENDCHANGE;
    let setting = OsStr::new("Control Panel\\Cursors")
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<u16>>();

    for _ in 0..3 {
        let reload_ok =
            unsafe { SystemParametersInfoW(SPI_SETCURSORS, 0, std::ptr::null_mut(), flags) != 0 };
        let mut broadcast_result = 0usize;
        let broadcast_ok = unsafe {
            SendMessageTimeoutW(
                HWND_BROADCAST,
                WM_SETTINGCHANGE,
                0,
                setting.as_ptr() as LPARAM,
                SMTO_ABORTIFHUNG,
                2000,
                &mut broadcast_result,
            ) != 0
        };

        if reload_ok || broadcast_ok {
            return true;
        }

        std::thread::sleep(std::time::Duration::from_millis(180));
    }

    false
}

#[cfg(not(target_os = "windows"))]
fn apply_to_windows_current_user(_skin: &SkinPackage) -> Result<(), String> {
    Err("应用光标只支持 Windows。".into())
}

#[cfg(not(target_os = "windows"))]
fn reset_windows_default_cursors() -> Result<(), String> {
    Err("恢复默认光标只支持 Windows。".into())
}

#[cfg(not(target_os = "windows"))]
fn refresh_windows_cursors() -> Result<(), String> {
    Err("刷新光标只支持 Windows。".into())
}

fn startup_log_path() -> PathBuf {
    std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("CursorSkinManager")
        .join("startup.log")
}

fn append_startup_log(message: &str) {
    let path = startup_log_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "[{}] {}", imported_at(), message);
    }
}

#[cfg(target_os = "windows")]
fn show_native_startup_error(message: &str) {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK};

    let title = OsStr::new("Cursor Skin Manager - 启动失败")
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<u16>>();
    let body = OsStr::new(message)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<u16>>();
    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            body.as_ptr(),
            title.as_ptr(),
            MB_OK | MB_ICONERROR,
        );
    }
}

#[cfg(not(target_os = "windows"))]
fn show_native_startup_error(message: &str) {
    eprintln!("Cursor Skin Manager startup failed: {message}");
}

pub fn report_startup_error(message: &str) {
    append_startup_log(message);
    show_native_startup_error(&format!(
        "应用无法启动。\n\n{message}\n\n诊断日志：{}",
        startup_log_path().to_string_lossy()
    ));
}

pub fn install_startup_diagnostics() {
    append_startup_log("application process started");
    std::panic::set_hook(Box::new(|info| {
        let message = format!("panic during startup or runtime: {info}");
        append_startup_log(&message);
        show_native_startup_error(&format!(
            "应用发生异常并需要关闭。\n\n{message}\n\n诊断日志：{}",
            startup_log_path().to_string_lossy()
        ));
    }));
}

fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn build_tray(app: &mut tauri::App) -> tauri::Result<()> {
    let show_item = MenuItem::with_id(app, "show", "显示主窗口", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show_item, &quit_item])?;

    let mut tray_builder = TrayIconBuilder::new();
    if let Some(icon) = app.default_window_icon() {
        tray_builder = tray_builder.icon(icon.clone());
    }

    tray_builder
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => show_main_window(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let result = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_main_window(app);
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .setup(|app| {
            let settings = read_app_settings(app.handle()).unwrap_or_default();
            let tray_available = match build_tray(app) {
                Ok(()) => true,
                Err(error) => {
                    let message = format!("system tray initialization failed: {error}");
                    append_startup_log(&message);
                    append_log(app.handle(), &message);
                    false
                }
            };
            app.manage(SettingsState {
                close_to_tray: AtomicBool::new(settings.close_to_tray && tray_available),
                tray_available: AtomicBool::new(tray_available),
            });
            append_startup_log("tauri setup completed");
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let close_to_tray = window
                    .app_handle()
                    .state::<SettingsState>()
                    .close_to_tray
                    .load(Ordering::Relaxed);
                if close_to_tray {
                    api.prevent_close();
                    let _ = window.hide();
                } else {
                    window.app_handle().exit(0);
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            load_library,
            import_skin,
            replace_cursor_role,
            assign_unassigned_cursor,
            delete_skin,
            apply_skin,
            reset_system_cursors,
            load_app_settings,
            set_close_to_tray,
            clear_all_skins,
            open_skin_dir,
            open_log_file,
            refresh_system_cursors
        ])
        .run(tauri::generate_context!());

    if let Err(error) = result {
        report_startup_error(&format!("Tauri runtime error: {error}"));
    }
}

#[cfg(test)]
mod preview_tests {
    use super::*;

    #[test]
    fn decodes_indexed_cursor_dibs() {
        for bit_count in [1u16, 4, 8] {
            let png = dib_to_png(&indexed_dib(bit_count))
                .expect("DIB decoding should not fail")
                .expect("indexed DIB should produce a PNG");
            let image = image::load_from_memory(&png)
                .expect("generated PNG should decode")
                .to_rgba8();
            assert_eq!(image.dimensions(), (2, 2));
            assert_eq!(image.get_pixel(0, 0).0, [255, 0, 0, 255]);
        }
    }

    #[test]
    fn writes_library_with_a_valid_backup() {
        let directory = std::env::temp_dir().join(new_id());
        fs::create_dir_all(&directory).expect("temporary directory should be created");
        let path = directory.join("library.json");

        write_file_with_backup(&path, b"[]").expect("initial write should succeed");
        write_file_with_backup(&path, b"[1]").expect("replacement write should succeed");

        assert_eq!(fs::read(&path).expect("library should exist"), b"[1]");
        assert_eq!(
            fs::read(library_backup_path(&path)).expect("backup should exist"),
            b"[]"
        );
        fs::remove_dir_all(directory).expect("temporary directory should be removed");
    }

    #[test]
    fn centers_visible_cursor_pixels() {
        let mut source = image::RgbaImage::new(32, 32);
        for y in 1..13 {
            for x in 2..8 {
                source.put_pixel(x, y, image::Rgba([255, 255, 255, 255]));
            }
        }
        let mut png = Vec::new();
        PngEncoder::new(&mut png)
            .write_image(source.as_raw(), 32, 32, ColorType::Rgba8.into())
            .expect("test PNG should encode");

        let centered = image::load_from_memory(
            &center_preview_png(&png).expect("preview centering should succeed"),
        )
        .expect("centered PNG should decode")
        .to_rgba8();
        assert_eq!(centered.dimensions(), (16, 16));

        let visible: Vec<_> = centered
            .enumerate_pixels()
            .filter(|(_, _, pixel)| pixel[3] > 1)
            .map(|(x, y, _)| (x, y))
            .collect();
        let min_x = visible.iter().map(|(x, _)| *x).min().unwrap();
        let max_x = visible.iter().map(|(x, _)| *x).max().unwrap();
        let min_y = visible.iter().map(|(_, y)| *y).min().unwrap();
        let max_y = visible.iter().map(|(_, y)| *y).max().unwrap();
        assert!((min_x as i32 - (15 - max_x) as i32).abs() <= 1);
        assert!((min_y as i32 - (15 - max_y) as i32).abs() <= 1);
    }

    #[test]
    fn parses_gbk_inf_with_case_insensitive_variables() {
        let directory = std::env::temp_dir().join(new_id());
        fs::create_dir_all(&directory).expect("temporary directory should be created");
        let cursor_path = directory.join("正常选择.ani");
        fs::write(&cursor_path, []).expect("cursor placeholder should be created");
        let inf_path = directory.join("AutoSetup.inf");
        let inf = "[Wreg]\r\nHKCU,\"Control Panel\\Cursors\",Arrow,0x00020000,\"%PoInTeR%\"\r\n[Strings]\r\npointer=\"正常选择.ani\"\r\n";
        let (encoded, _, _) = encoding_rs::GBK.encode(inf);
        fs::write(&inf_path, encoded.as_ref()).expect("GBK INF should be written");

        let parsed =
            parse_inf(&inf_path, std::slice::from_ref(&cursor_path)).expect("GBK INF should parse");
        assert_eq!(parsed.get("Arrow"), Some(&cursor_path));

        fs::remove_dir_all(directory).expect("temporary directory should be removed");
    }

    #[test]
    fn replaces_role_with_external_cur_and_preserves_source() {
        let directory = test_directory("中文 路径 [cur]");
        let storage = directory.join("皮肤 #1");
        fs::create_dir_all(&storage).expect("skin directory should be created");
        let source = directory.join("新的 正常选择 #1.cur");
        let old = storage.join("old.cur");
        write_valid_cur(&source);
        write_valid_cur(&old);

        let mut skin = test_skin(&storage);
        set_role_file(&mut skin, "Arrow", &old);
        skin.is_applied = true;
        let mut library = vec![skin];
        let index = role_index(&library[0], "Arrow");
        let mutation =
            replace_cursor_role_transaction(&mut library, 0, index, &source, &storage, |_| Ok(()))
                .expect("external CUR replacement should succeed");

        let new_path = PathBuf::from(&mutation.new_path);
        assert!(source.exists(), "external source must not be modified");
        assert!(new_path.exists(), "replacement copy should exist");
        assert!(new_path.starts_with(storage.join("custom")));
        assert_eq!(role_path(&library[0], "Arrow"), Some(mutation.new_path));
        assert!(library[0]
            .unassigned_files
            .iter()
            .any(|file| stored_paths_match(&file.file_path, &old.to_string_lossy())));
        assert!(!library[0].is_applied);

        fs::remove_dir_all(directory).expect("temporary directory should be removed");
    }

    #[test]
    fn replaces_role_with_external_ani() {
        let directory = test_directory("ani replacement");
        let storage = directory.join("skin");
        fs::create_dir_all(&storage).expect("skin directory should be created");
        let source = directory.join("busy animation.ani");
        write_valid_ani(&source);
        let mut library = vec![test_skin(&storage)];
        let index = role_index(&library[0], "Wait");

        replace_cursor_role_transaction(&mut library, 0, index, &source, &storage, |_| Ok(()))
            .expect("external ANI replacement should succeed");

        let role = &library[0].roles[index];
        assert_eq!(role.cursor_type.as_deref(), Some("ani"));
        assert!(role
            .preview_path
            .as_deref()
            .is_some_and(|path| Path::new(path).exists()));

        fs::remove_dir_all(directory).expect("temporary directory should be removed");
    }

    #[test]
    fn damaged_cursor_replacement_preserves_mapping() {
        let directory = test_directory("damaged replacement");
        let storage = directory.join("skin");
        fs::create_dir_all(&storage).expect("skin directory should be created");
        let old = storage.join("old.cur");
        let damaged = directory.join("damaged.cur");
        write_valid_cur(&old);
        fs::write(&damaged, b"not a cursor").expect("damaged file should be written");
        let mut skin = test_skin(&storage);
        set_role_file(&mut skin, "Arrow", &old);
        let mut library = vec![skin];
        let before = serde_json::to_string(&library).expect("library should serialize");
        let index = role_index(&library[0], "Arrow");

        let result =
            replace_cursor_role_transaction(&mut library, 0, index, &damaged, &storage, |_| Ok(()));

        assert!(result.is_err());
        assert_eq!(
            serde_json::to_string(&library).expect("library should serialize"),
            before
        );
        fs::remove_dir_all(directory).expect("temporary directory should be removed");
    }

    #[test]
    fn duplicate_names_never_overwrite_internal_files() {
        let directory = test_directory("duplicate names");
        let storage = directory.join("skin");
        fs::create_dir_all(&storage).expect("skin directory should be created");
        let source = directory.join("same.cur");
        write_valid_cur(&source);

        let first =
            copy_cursor_to_internal_storage(&source, &storage).expect("first copy should succeed");
        fs::write(&source, valid_cur_bytes_with_palette([0, 255, 0]))
            .expect("source should be updated");
        let second =
            copy_cursor_to_internal_storage(&source, &storage).expect("second copy should succeed");

        assert_ne!(first, second);
        assert_ne!(
            fs::read(&first).expect("first copy should remain readable"),
            fs::read(&second).expect("second copy should be readable")
        );
        fs::remove_dir_all(directory).expect("temporary directory should be removed");
    }

    #[test]
    fn replaced_role_file_moves_to_unassigned() {
        let directory = test_directory("old file to unassigned");
        let storage = directory.join("skin");
        fs::create_dir_all(&storage).expect("skin directory should be created");
        let old = storage.join("old.cur");
        let new = storage.join("new.cur");
        write_valid_cur(&old);
        write_valid_cur(&new);
        let mut skin = test_skin(&storage);
        set_role_file(&mut skin, "Arrow", &old);

        let index = role_index(&skin, "Arrow");
        replace_role_mapping(&mut skin, index, cursor_file_from_path(&new, None));

        assert_eq!(skin.unassigned_files.len(), 1);
        assert!(stored_paths_match(
            &skin.unassigned_files[0].file_path,
            &old.to_string_lossy()
        ));
        fs::remove_dir_all(directory).expect("temporary directory should be removed");
    }

    #[test]
    fn shared_role_file_is_not_moved_or_deleted() {
        let directory = test_directory("shared role file");
        let storage = directory.join("skin");
        fs::create_dir_all(&storage).expect("skin directory should be created");
        let shared = storage.join("shared.cur");
        let new = storage.join("new.cur");
        write_valid_cur(&shared);
        write_valid_cur(&new);
        let mut skin = test_skin(&storage);
        set_role_file(&mut skin, "Arrow", &shared);
        set_role_file(&mut skin, "Help", &shared);

        let index = role_index(&skin, "Arrow");
        replace_role_mapping(&mut skin, index, cursor_file_from_path(&new, None));

        assert!(shared.exists());
        assert_eq!(
            role_path(&skin, "Help"),
            Some(shared.to_string_lossy().to_string())
        );
        assert!(skin.unassigned_files.is_empty());
        fs::remove_dir_all(directory).expect("temporary directory should be removed");
    }

    #[test]
    fn assigns_unassigned_cursor_to_empty_role() {
        let directory = test_directory("assign empty role");
        let storage = directory.join("skin");
        fs::create_dir_all(&storage).expect("skin directory should be created");
        let source = storage.join("unassigned.cur");
        write_valid_cur(&source);
        let mut skin = test_skin(&storage);
        skin.unassigned_files
            .push(cursor_file_from_path(&source, None));
        let mut library = vec![skin];
        let index = role_index(&library[0], "Crosshair");

        assign_unassigned_cursor_transaction(&mut library, 0, index, 0, &storage, |_| Ok(()))
            .expect("assignment to empty role should succeed");

        assert_eq!(
            role_path(&library[0], "Crosshair"),
            Some(source.to_string_lossy().to_string())
        );
        assert!(library[0].unassigned_files.is_empty());
        fs::remove_dir_all(directory).expect("temporary directory should be removed");
    }

    #[test]
    fn swaps_unassigned_cursor_with_existing_role() {
        let directory = test_directory("swap existing role");
        let storage = directory.join("skin");
        fs::create_dir_all(&storage).expect("skin directory should be created");
        let old = storage.join("old.cur");
        let source = storage.join("unassigned.cur");
        write_valid_cur(&old);
        write_valid_cur(&source);
        let mut skin = test_skin(&storage);
        set_role_file(&mut skin, "Arrow", &old);
        skin.unassigned_files
            .push(cursor_file_from_path(&source, None));
        let mut library = vec![skin];
        let index = role_index(&library[0], "Arrow");

        assign_unassigned_cursor_transaction(&mut library, 0, index, 0, &storage, |_| Ok(()))
            .expect("swap should succeed");

        assert_eq!(
            role_path(&library[0], "Arrow"),
            Some(source.to_string_lossy().to_string())
        );
        assert_eq!(library[0].unassigned_files.len(), 1);
        assert!(stored_paths_match(
            &library[0].unassigned_files[0].file_path,
            &old.to_string_lossy()
        ));
        fs::remove_dir_all(directory).expect("temporary directory should be removed");
    }

    #[test]
    fn failed_library_write_rolls_back_file_and_mapping() {
        let directory = test_directory("rollback transaction");
        let storage = directory.join("skin");
        fs::create_dir_all(&storage).expect("skin directory should be created");
        let old = storage.join("old.cur");
        let source = directory.join("new.cur");
        write_valid_cur(&old);
        write_valid_cur(&source);
        let mut skin = test_skin(&storage);
        set_role_file(&mut skin, "Arrow", &old);
        let mut library = vec![skin];
        let before = serde_json::to_string(&library).expect("library should serialize");
        let index = role_index(&library[0], "Arrow");

        let result =
            replace_cursor_role_transaction(&mut library, 0, index, &source, &storage, |_| {
                Err("simulated library write failure".to_string())
            });

        assert!(result.is_err());
        assert_eq!(
            serde_json::to_string(&library).expect("library should serialize"),
            before
        );
        let custom = storage.join("custom");
        assert!(!custom.exists() || custom.read_dir().unwrap().next().is_none());
        fs::remove_dir_all(directory).expect("temporary directory should be removed");
    }

    #[test]
    fn modifying_applied_skin_marks_it_unapplied() {
        let directory = test_directory("applied state");
        let storage = directory.join("skin");
        fs::create_dir_all(&storage).expect("skin directory should be created");
        let source = directory.join("new.cur");
        write_valid_cur(&source);
        let mut skin = test_skin(&storage);
        skin.is_applied = true;
        let mut library = vec![skin];
        let index = role_index(&library[0], "Arrow");

        replace_cursor_role_transaction(&mut library, 0, index, &source, &storage, |_| Ok(()))
            .expect("replacement should succeed");

        assert!(!library[0].is_applied);
        fs::remove_dir_all(directory).expect("temporary directory should be removed");
    }

    #[test]
    fn validates_real_cursor_samples_when_configured() {
        for variable in ["CSM_REAL_CUR", "CSM_REAL_ANI"] {
            let Some(path) = std::env::var_os(variable).map(PathBuf::from) else {
                continue;
            };
            validate_cursor_file(&path).unwrap_or_else(|error| {
                panic!(
                    "{variable} sample {} should validate: {error}",
                    path.display()
                )
            });
        }
    }

    fn test_directory(label: &str) -> PathBuf {
        let directory = std::env::temp_dir().join(format!("{}-{label}", new_id()));
        fs::create_dir_all(&directory).expect("temporary directory should be created");
        directory
    }

    fn test_skin(storage: &Path) -> SkinPackage {
        SkinPackage {
            id: new_id(),
            name: "Test Skin".to_string(),
            source_path: storage.to_string_lossy().to_string(),
            storage_path: storage.to_string_lossy().to_string(),
            imported_at: imported_at(),
            has_inf: false,
            is_complete: false,
            cursor_count: 0,
            is_applied: false,
            import_note: None,
            roles: ROLE_DEFS
                .iter()
                .map(|definition| CursorRole {
                    role: definition.role.to_string(),
                    windows_key: definition.windows_key.to_string(),
                    file_path: None,
                    file_name: None,
                    preview_path: None,
                    preview_data_url: None,
                    cursor_type: None,
                    exists: false,
                })
                .collect(),
            unassigned_files: Vec::new(),
        }
    }

    fn role_index(skin: &SkinPackage, windows_key: &str) -> usize {
        skin.roles
            .iter()
            .position(|role| role.windows_key == windows_key)
            .expect("role should exist")
    }

    fn role_path(skin: &SkinPackage, windows_key: &str) -> Option<String> {
        skin.roles
            .iter()
            .find(|role| role.windows_key == windows_key)
            .and_then(|role| role.file_path.clone())
    }

    fn set_role_file(skin: &mut SkinPackage, windows_key: &str, path: &Path) {
        let index = role_index(skin, windows_key);
        let file = cursor_file_from_path(path, None);
        skin.roles[index].file_path = Some(file.file_path);
        skin.roles[index].file_name = Some(file.file_name);
        skin.roles[index].cursor_type = Some(file.cursor_type);
        skin.roles[index].exists = true;
    }

    fn write_valid_cur(path: &Path) {
        fs::write(path, valid_cur_bytes_with_palette([255, 0, 0]))
            .expect("valid CUR should be written");
    }

    fn write_valid_ani(path: &Path) {
        let cursor = valid_cur_bytes_with_palette([255, 0, 0]);
        let padding = cursor.len() % 2;
        let riff_size = 4 + 8 + cursor.len() + padding;
        let mut ani = Vec::with_capacity(riff_size + 8);
        ani.extend_from_slice(b"RIFF");
        ani.extend_from_slice(&(riff_size as u32).to_le_bytes());
        ani.extend_from_slice(b"ACON");
        ani.extend_from_slice(b"icon");
        ani.extend_from_slice(&(cursor.len() as u32).to_le_bytes());
        ani.extend_from_slice(&cursor);
        if padding == 1 {
            ani.push(0);
        }
        fs::write(path, ani).expect("valid ANI should be written");
    }

    fn valid_cur_bytes_with_palette(rgb: [u8; 3]) -> Vec<u8> {
        let mut dib = indexed_dib(8);
        dib[44] = rgb[2];
        dib[45] = rgb[1];
        dib[46] = rgb[0];
        let mut cursor = vec![0u8; 22];
        cursor[2..4].copy_from_slice(&2u16.to_le_bytes());
        cursor[4..6].copy_from_slice(&1u16.to_le_bytes());
        cursor[6] = 2;
        cursor[7] = 2;
        cursor[14..18].copy_from_slice(&(dib.len() as u32).to_le_bytes());
        cursor[18..22].copy_from_slice(&22u32.to_le_bytes());
        cursor.extend_from_slice(&dib);
        cursor
    }

    fn indexed_dib(bit_count: u16) -> Vec<u8> {
        let width = 2usize;
        let height = 2usize;
        let palette_len = 1usize << bit_count;
        let stride = (width * bit_count as usize).div_ceil(32) * 4;
        let mask_stride = width.div_ceil(32) * 4;
        let pixel_offset = 40 + palette_len * 4;
        let mut dib = vec![0u8; pixel_offset + stride * height + mask_stride * height];

        dib[0..4].copy_from_slice(&40u32.to_le_bytes());
        dib[4..8].copy_from_slice(&(width as i32).to_le_bytes());
        dib[8..12].copy_from_slice(&((height * 2) as i32).to_le_bytes());
        dib[12..14].copy_from_slice(&1u16.to_le_bytes());
        dib[14..16].copy_from_slice(&bit_count.to_le_bytes());
        dib[32..36].copy_from_slice(&(palette_len as u32).to_le_bytes());

        let red = 40 + 4;
        dib[red..red + 4].copy_from_slice(&[0, 0, 255, 0]);
        for row in 0..height {
            let start = pixel_offset + row * stride;
            match bit_count {
                8 => dib[start..start + width].fill(1),
                4 => dib[start] = 0x11,
                1 => dib[start] = 0b1100_0000,
                _ => unreachable!(),
            }
        }

        dib
    }
}
