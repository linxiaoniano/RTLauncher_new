mod account;
mod version;
mod java;
mod launcher;
mod config;

use account::types::*;
use account::store::AccountStore;
use account::offline;
use account::microsoft;
use account::yggdrasil;
use version::types::*;
use version::manifest;
use version::install::{VersionInstaller, InstallProgress, InstallStage, get_installed_versions as load_installed_versions, delete_version};
use version::download::set_custom_game_dir;
use java::{detect_java_installations, JavaInstallation};
use launcher::{GameLauncher, LaunchConfig};
use config::{LauncherConfig, ManualJavaEntry};
use std::sync::Mutex;
use std::sync::Arc;
use tauri::{State, Emitter};
use reqwest::Client;
use std::path::PathBuf;

/// 应用状态
struct AppState {
    store: Mutex<AccountStore>,
    config: Mutex<LauncherConfig>,
    http_client: Mutex<Client>,
    /// 存储完整的设备码响应（用于微软登录轮询）
    ms_device_code: Mutex<Option<MicrosoftDeviceCodeResponse>>,
    /// 游戏目录（用户可自定义）
    game_dir: Mutex<Option<PathBuf>>,
}

// ==================== 账户相关命令 ====================

/// 获取所有账户
#[tauri::command]
fn get_accounts(state: State<AppState>) -> Result<Vec<Account>, String> {
    let store = state.store.lock().map_err(|e| e.to_string())?;
    Ok(store.accounts.clone())
}

/// 获取当前选中的账户
#[tauri::command]
fn get_selected_account(state: State<AppState>) -> Result<Option<Account>, String> {
    let store = state.store.lock().map_err(|e| e.to_string())?;
    Ok(store.get_selected_account().cloned())
}

/// 设置选中的账户
#[tauri::command]
fn select_account(state: State<AppState>, account_id: String) -> Result<(), String> {
    let account_id = uuid::Uuid::parse_str(&account_id)
        .map_err(|e| format!("无效的账户ID: {}", e))?;
    
    let mut store = state.store.lock().map_err(|e| e.to_string())?;
    
    if !store.set_selected_account(account_id) {
        return Err("账户不存在".to_string());
    }
    
    store.save()
}

/// 离线登录
#[tauri::command]
fn login_offline(state: State<AppState>, username: String) -> Result<Account, String> {
    let account = offline::create_offline_account(&username);
    
    let mut store = state.store.lock().map_err(|e| e.to_string())?;
    store.add_account(account.clone());
    store.save()?;
    
    Ok(account)
}

/// 开始微软登录 - 获取设备码
#[tauri::command]
async fn start_microsoft_login(state: State<'_, AppState>) -> Result<DeviceCodeInfo, String> {
    let client = state.http_client.lock()
        .map_err(|e| e.to_string())?
        .clone();
    
    let device_code = microsoft::get_device_code(&client)
        .await
        .map_err(|e| e.to_string())?;
    
    let info = DeviceCodeInfo {
        verification_uri: device_code.verification_uri.clone(),
        user_code: device_code.user_code.clone(),
        expires_in: device_code.expires_in,
        interval: device_code.interval,
    };
    
    // 保存完整的设备码响应供后续轮询使用
    let mut stored_code = state.ms_device_code.lock().map_err(|e| e.to_string())?;
    *stored_code = Some(device_code);
    
    Ok(info)
}

/// 轮询微软登录状态
#[tauri::command]
async fn poll_microsoft_login(state: State<'_, AppState>) -> Result<Account, String> {
    let client = state.http_client.lock()
        .map_err(|e| e.to_string())?
        .clone();
    
    // 获取存储的设备码
    let device_code = {
        let stored = state.ms_device_code.lock().map_err(|e| e.to_string())?;
        stored.clone().ok_or("没有进行中的微软登录，请先调用 start_microsoft_login")?
    };
    
    // 尝试获取令牌
    match microsoft::poll_for_token(&client, &device_code.device_code).await {
        Ok(_token) => {
            // 令牌获取成功，继续完成登录流程
            let account = microsoft::login_with_microsoft(&client, &device_code.device_code, None)
                .await
                .map_err(|e| e.to_string())?;
            
            // 清除设备码
            let mut stored = state.ms_device_code.lock().map_err(|e| e.to_string())?;
            *stored = None;
            
            // 保存账户
            let mut store = state.store.lock().map_err(|e| e.to_string())?;
            store.add_account(account.clone());
            store.save()?;
            
            Ok(account)
        }
        Err(microsoft::MicrosoftAuthError::AuthorizationPending) => {
            // 还在等待用户授权
            Err("等待用户授权...".to_string())
        }
        Err(microsoft::MicrosoftAuthError::DeviceCodeExpired) => {
            // 设备码过期，清除状态
            let mut stored = state.ms_device_code.lock().map_err(|e| e.to_string())?;
            *stored = None;
            Err("设备码已过期，请重新开始登录".to_string())
        }
        Err(e) => {
            Err(format!("登录失败: {}", e))
        }
    }
}

/// 取消微软登录
#[tauri::command]
fn cancel_microsoft_login(state: State<AppState>) -> Result<(), String> {
    let mut stored = state.ms_device_code.lock().map_err(|e| e.to_string())?;
    *stored = None;
    Ok(())
}

/// 外置登录
#[tauri::command]
async fn login_yggdrasil(
    state: State<'_, AppState>,
    email: String,
    password: String,
    server_url: String,
) -> Result<Account, String> {
    let client = state.http_client.lock()
        .map_err(|e| e.to_string())?
        .clone();
    
    let account = yggdrasil::login(&client, &email, &password, &server_url)
        .await
        .map_err(|e| e.to_string())?;
    
    let mut store = state.store.lock().map_err(|e| e.to_string())?;
    store.add_account(account.clone());
    store.save()?;
    
    Ok(account)
}

/// 刷新账户令牌
#[tauri::command]
async fn refresh_account(state: State<'_, AppState>, account_id: String) -> Result<Account, String> {
    let account_id = uuid::Uuid::parse_str(&account_id)
        .map_err(|e| format!("无效的账户ID: {}", e))?;
    
    let client = state.http_client.lock()
        .map_err(|e| e.to_string())?
        .clone();
    
    // 获取账户信息后立即释放锁
    let account = {
        let store = state.store.lock().map_err(|e| e.to_string())?;
        let account = store.get_account(account_id)
            .ok_or("账户不存在")?
            .clone();
        account
    };
    
    let refreshed = match account.account_type {
        AccountType::Microsoft => {
            microsoft::refresh_account(&client, &account)
                .await
                .map_err(|e| e.to_string())?
        }
        AccountType::Yggdrasil => {
            yggdrasil::refresh_token(&client, &account)
                .await
                .map_err(|e| e.to_string())?
        }
        AccountType::Offline => {
            return Err("离线账户无需刷新".to_string());
        }
    };
    
    // 重新获取锁来更新账户
    let mut store = state.store.lock().map_err(|e| e.to_string())?;
    store.update_account(refreshed.clone());
    store.save()?;
    
    Ok(refreshed)
}

/// 删除账户
#[tauri::command]
fn remove_account(state: State<AppState>, account_id: String) -> Result<(), String> {
    let account_id = uuid::Uuid::parse_str(&account_id)
        .map_err(|e| format!("无效的账户ID: {}", e))?;
    
    let mut store = state.store.lock().map_err(|e| e.to_string())?;
    
    if !store.remove_account(account_id) {
        return Err("账户不存在".to_string());
    }
    
    store.save()
}

// ==================== 版本管理相关命令 ====================

/// 获取远程版本清单
#[tauri::command]
async fn get_version_manifest(state: State<'_, AppState>) -> Result<VersionManifest, String> {
    let client = state.http_client.lock()
        .map_err(|e| e.to_string())?
        .clone();
    
    manifest::get_version_manifest(&client)
        .await
        .map_err(|e| e.to_string())
}

/// 获取已安装的版本列表
#[tauri::command]
fn get_installed_versions() -> Result<Vec<InstalledVersion>, String> {
    load_installed_versions().map_err(|e| e.to_string())
}

/// 安装版本
#[tauri::command]
async fn install_version(
    state: State<'_, AppState>,
    version_id: String,
    app_handle: tauri::AppHandle,
) -> Result<InstalledVersion, String> {
    let client = state.http_client.lock()
        .map_err(|e| e.to_string())?
        .clone();
    
    let progress = Arc::new(move |p: InstallProgress| {
        // 发送进度事件到前端
        let _ = app_handle.emit("install-progress", InstallProgressEvent {
            stage: match p.stage {
                InstallStage::DownloadingJson => "downloading_json",
                InstallStage::DownloadingClient => "downloading_client",
                InstallStage::DownloadingLibraries => "downloading_libraries",
                InstallStage::DownloadingAssets => "downloading_assets",
                InstallStage::ExtractingNatives => "extracting_natives",
                InstallStage::Completed => "completed",
            },
            current: p.current,
            total: p.total,
            message: p.message,
        });
    });
    
    let installer = VersionInstaller::new(client, progress);
    installer.install(&version_id).await.map_err(|e| e.to_string())
}

/// 删除版本
#[tauri::command]
fn delete_version_cmd(version_id: String) -> Result<(), String> {
    delete_version(&version_id).map_err(|e| e.to_string())
}

/// 安装进度事件
#[derive(Clone, serde::Serialize)]
struct InstallProgressEvent {
    stage: &'static str,
    current: u64,
    total: u64,
    message: String,
}

// ==================== Java 相关命令 ====================

/// 检测系统 Java 安装
#[tauri::command]
fn detect_java() -> Vec<JavaInstallation> {
    detect_java_installations()
}

// ==================== 文件操作命令 ====================

/// 获取游戏目录
#[tauri::command]
fn get_game_dir_cmd(state: State<AppState>) -> Result<String, String> {
    let game_dir = state.game_dir.lock().map_err(|e| e.to_string())?;
    
    let path = game_dir.clone().unwrap_or_else(|| {
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("RTLauncher")
            .join("minecraft")
    });
    
    Ok(path.to_string_lossy().to_string())
}

/// 从配置文件加载游戏目录
fn load_game_dir_config() -> Option<PathBuf> {
    let config_dir = dirs::config_dir()?;
    let config_path = config_dir.join("RTLauncher").join("config.json");
    
    if config_path.exists() {
        let content = std::fs::read_to_string(&config_path).ok()?;
        Some(PathBuf::from(content.trim()))
    } else {
        None
    }
}

/// 设置游戏目录
#[tauri::command]
fn set_game_dir(state: State<AppState>, path: String) -> Result<(), String> {
    let mut game_dir = state.game_dir.lock().map_err(|e| e.to_string())?;
    let path_buf = PathBuf::from(&path);
    *game_dir = Some(path_buf.clone());
    
    // 同时设置全局游戏目录（用于版本安装等）
    set_custom_game_dir(path_buf);
    
    // 持久化保存到配置文件
    if let Some(config_dir) = dirs::config_dir() {
        let config_path = config_dir.join("RTLauncher").join("config.json");
        if let Some(parent) = config_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(&config_path, &path);
    }
    
    Ok(())
}

/// 在文件管理器中打开路径
#[tauri::command]
fn open_path(path: String) -> Result<(), String> {
    let path = std::path::Path::new(&path);
    
    if !path.exists() {
        // 尝试创建目录
        std::fs::create_dir_all(path)
            .map_err(|e| format!("目录不存在且创建失败: {}", e))?;
    }
    
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        std::process::Command::new("explorer")
            .arg(path)
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .map_err(|e| format!("打开目录失败: {}", e))?;
    }
    
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(path)
            .spawn()
            .map_err(|e| format!("打开目录失败: {}", e))?;
    }
    
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(path)
            .spawn()
            .map_err(|e| format!("打开目录失败: {}", e))?;
    }
    
    Ok(())
}

// ==================== 启动器配置相关命令 ====================

/// 获取启动器配置
#[tauri::command]
fn get_launcher_config(state: State<AppState>) -> Result<LauncherConfig, String> {
    let config = state.config.lock().map_err(|e| e.to_string())?;
    Ok(config.clone())
}

/// 设置选中的 Java
#[tauri::command]
fn set_selected_java(state: State<AppState>, path: String) -> Result<(), String> {
    let mut config = state.config.lock().map_err(|e| e.to_string())?;
    config.set_selected_java(path);
    config.save()
}

/// 添加手动 Java
#[tauri::command]
fn add_manual_java(
    state: State<AppState>,
    path: String,
    version: String,
    major_version: u32,
    vendor: String,
    arch: String,
) -> Result<(), String> {
    let mut config = state.config.lock().map_err(|e| e.to_string())?;
    config.add_manual_java(ManualJavaEntry {
        path,
        version,
        major_version,
        vendor,
        arch,
    });
    config.save()
}

/// 移除手动 Java
#[tauri::command]
fn remove_manual_java(state: State<AppState>, path: String) -> Result<bool, String> {
    let mut config = state.config.lock().map_err(|e| e.to_string())?;
    Ok(config.remove_manual_java(&path))
}

/// 设置内存
#[tauri::command]
fn set_memory(state: State<AppState>, min: u32, max: u32) -> Result<(), String> {
    let mut config = state.config.lock().map_err(|e| e.to_string())?;
    config.set_memory(min, max);
    config.save()
}

/// 设置选中版本
#[tauri::command]
fn set_selected_version(state: State<AppState>, version: String) -> Result<(), String> {
    let mut config = state.config.lock().map_err(|e| e.to_string())?;
    config.set_selected_version(version);
    config.save()
}

// ==================== 启动相关命令 ====================

/// 启动游戏
#[tauri::command]
async fn launch_game(
    state: State<'_, AppState>,
    version_id: String,
    java_path: String,
    java_version: u32,
    min_memory: u32,
    max_memory: u32,
) -> Result<(), String> {
    // 获取当前选中的账户
    let account = {
        let store = state.store.lock().map_err(|e| e.to_string())?;
        store.get_selected_account()
            .cloned()
            .ok_or("请先选择一个账户")?
    };
    
    // 获取游戏目录
    let game_dir_path = {
        let gd = state.game_dir.lock().map_err(|e| e.to_string())?;
        gd.clone()
    };
    if let Some(ref path) = game_dir_path {
        set_custom_game_dir(path.clone());
    }
    
    // 构建启动配置
    let config = LaunchConfig {
        version_id: version_id.clone(),
        java: JavaInstallation {
            path: java_path.into(),
            version: String::new(),
            major_version: java_version,
            vendor: String::new(),
            arch: String::new(),
        },
        min_memory,
        max_memory,
        ..Default::default()
    };
    
    // 创建启动器并启动
    let launcher = GameLauncher::new(&version_id, config, account)
        .map_err(|e| e.to_string())?;
    
    launcher.launch().map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 加载账户存储
    let store = AccountStore::load().unwrap_or_default();
    
    // 加载保存的游戏目录
    let saved_game_dir = load_game_dir_config();
    if let Some(ref path) = saved_game_dir {
        set_custom_game_dir(path.clone());
    }
    
    // 加载启动器配置
    let launcher_config = LauncherConfig::load().unwrap_or_default();
    let saved_game_dir_from_config = launcher_config.game_dir.clone().map(PathBuf::from);
    let effective_game_dir = saved_game_dir_from_config.clone().or_else(|| {
        saved_game_dir.clone()
    });
    if let Some(ref path) = effective_game_dir {
        set_custom_game_dir(path.clone());
    }
    
    tauri::Builder::default()
        .manage(AppState {
            store: Mutex::new(store),
            config: Mutex::new(launcher_config),
            http_client: Mutex::new(Client::new()),
            ms_device_code: Mutex::new(None),
            game_dir: Mutex::new(effective_game_dir),
        })
        .invoke_handler(tauri::generate_handler![
            // 账户
            get_accounts,
            get_selected_account,
            select_account,
            login_offline,
            start_microsoft_login,
            poll_microsoft_login,
            cancel_microsoft_login,
            login_yggdrasil,
            refresh_account,
            remove_account,
            // 版本
            get_version_manifest,
            get_installed_versions,
            install_version,
            delete_version_cmd,
            // Java
            detect_java,
            // 游戏目录
            get_game_dir_cmd,
            set_game_dir,
            open_path,
            // 启动器配置
            get_launcher_config,
            set_selected_java,
            add_manual_java,
            remove_manual_java,
            set_memory,
            set_selected_version,
            // 启动
            launch_game,
        ])
        .setup(|app| {
            app.handle().plugin(tauri_plugin_opener::init())?;
            app.handle().plugin(tauri_plugin_dialog::init())?;
            if cfg!(debug_assertions) {
                app.handle().plugin(
                  tauri_plugin_log::Builder::default()
                    .level(log::LevelFilter::Info)
                    .build(),
                )?;
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
