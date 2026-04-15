use crate::account::Account;
use super::version::*;
use super::version::download::{get_game_dir, get_version_dir, get_natives_dir, get_libraries_dir, get_assets_dir};
use super::java::{JavaInstallation, check_java_version};
use std::path::{PathBuf, Path};
use std::process::{Command, Stdio};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LaunchError {
    #[error("IO错误: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("JSON解析错误: {0}")]
    JsonError(#[from] serde_json::Error),
    
    #[error("版本不存在: {0}")]
    VersionNotFound(String),
    
    #[error("Java版本不满足要求: 需要 {required}, 当前 {current}")]
    JavaVersionMismatch { required: u32, current: u32 },
    
    #[error("缺少必要文件: {0}")]
    MissingFile(String),
    
    #[error("启动失败: {0}")]
    LaunchFailed(String),
}

/// 启动配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaunchConfig {
    /// 游戏版本ID
    pub version_id: String,
    /// Java 安装
    pub java: JavaInstallation,
    /// 最小内存 (MB)
    pub min_memory: u32,
    /// 最大内存 (MB)
    pub max_memory: u32,
    /// 窗口宽度
    pub width: u32,
    /// 窗口高度
    pub height: u32,
    /// 全屏
    pub fullscreen: bool,
    /// 自定义JVM参数
    pub jvm_args: Vec<String>,
    /// 自定义游戏参数
    pub game_args: Vec<String>,
    /// 服务器地址（自动加入服务器）
    pub server: Option<String>,
    /// 显示控制台窗口（调试模式）
    pub show_console: bool,
}

impl Default for LaunchConfig {
    fn default() -> Self {
        Self {
            version_id: String::new(),
            java: JavaInstallation {
                path: PathBuf::new(),
                version: String::new(),
                major_version: 8,
                vendor: String::new(),
                arch: String::new(),
            },
            min_memory: 512,
            max_memory: 2048,
            width: 854,
            height: 480,
            fullscreen: false,
            jvm_args: Vec::new(),
            game_args: Vec::new(),
            server: None,
            show_console: true, // 默认显示控制台，方便调试
        }
    }
}

/// 游戏启动器
pub struct GameLauncher {
    game_dir: PathBuf,
    version_detail: VersionDetail,
    config: LaunchConfig,
    account: Account,
}

impl GameLauncher {
    /// 创建启动器
    pub fn new(
        version_id: &str,
        config: LaunchConfig,
        account: Account,
    ) -> Result<Self, LaunchError> {
        let game_dir = get_game_dir();
        let version_dir = get_version_dir(version_id);
        let version_json = version_dir.join(format!("{}.json", version_id));
        
        if !version_json.exists() {
            return Err(LaunchError::VersionNotFound(version_id.to_string()));
        }
        
        let json_content = std::fs::read_to_string(&version_json)?;
        let version_detail: VersionDetail = serde_json::from_str(&json_content)?;
        
        // 检查 Java 版本
        let required_java = version_detail.java_version.as_ref()
            .map(|j| j.major_version)
            .unwrap_or(8);
        
        if !check_java_version(&config.java, required_java) {
            return Err(LaunchError::JavaVersionMismatch {
                required: required_java,
                current: config.java.major_version,
            });
        }
        
        Ok(Self {
            game_dir,
            version_detail,
            config,
            account,
        })
    }
    
    /// 构建启动命令
    pub fn build_command(&self) -> Result<Command, LaunchError> {
        let mut cmd = Command::new(self.config.java.java_exec());
        
        // JVM 参数
        let jvm_args = self.build_jvm_args()?;
        cmd.args(&jvm_args);
        
        // 主类
        cmd.arg(&self.version_detail.main_class);
        
        // 游戏参数
        let game_args = self.build_game_args()?;
        cmd.args(&game_args);
        
        // 工作目录
        cmd.current_dir(&self.game_dir);
        
        // 设置环境变量
        #[cfg(target_os = "windows")]
        {
            cmd.env("APPDATA", dirs::data_dir().unwrap_or_default());
        }
        
        Ok(cmd)
    }
    
    /// 启动游戏
    pub fn launch(&self) -> Result<(), LaunchError> {
        // 保存启动日志
        let log_file = self.save_launch_log()?;
        
        let mut cmd = self.build_command()?;
        
        if self.config.show_console {
            // 显示控制台窗口 - 不重定向输出
            // Windows 下使用 CREATE_NEW_CONSOLE 创建新控制台窗口
            #[cfg(target_os = "windows")]
            {
                use std::os::windows::process::CommandExt;
                const CREATE_NEW_CONSOLE: u32 = 0x00000010;
                cmd.creation_flags(CREATE_NEW_CONSOLE);
            }
        } else {
            // 隐藏控制台窗口
            #[cfg(target_os = "windows")]
            {
                use std::os::windows::process::CommandExt;
                const CREATE_NO_WINDOW: u32 = 0x08000000;
                cmd.creation_flags(CREATE_NO_WINDOW);
            }
            
            // 重定向输出到日志文件
            let log = File::create(&log_file)?;
            cmd.stdout(Stdio::from(log.try_clone()?));
            cmd.stderr(Stdio::from(log));
        }
        
        let java_path = self.config.java.java_exec();
        if !java_path.exists() {
            return Err(LaunchError::LaunchFailed(format!(
                "Java 可执行文件不存在: {}\n请检查 Java 路径是否正确: {}",
                java_path.to_string_lossy(),
                self.config.java.path.to_string_lossy()
            )));
        }
        
        let mut child = cmd.spawn()
            .map_err(|e| {
                // 尝试读取日志文件内容以提供更多调试信息
                let log_content = std::fs::read_to_string(&log_file).unwrap_or_default();
                LaunchError::LaunchFailed(format!(
                    "启动 Java 进程失败: {}\n\nJava 路径: {}\n工作目录: {}\n\n日志文件: {}\n\n日志内容:\n{}",
                    e,
                    java_path.to_string_lossy(),
                    self.game_dir.to_string_lossy(),
                    log_file.to_string_lossy(),
                    if log_content.is_empty() { "（空）" } else { &log_content }
                ))
            })?;
        
        // 短暂等待以检测立即失败的情况
        std::thread::sleep(std::time::Duration::from_millis(500));
        if let Ok(Some(status)) = child.try_wait() {
            // 进程已经退出，说明启动失败
            let log_content = std::fs::read_to_string(&log_file).unwrap_or_default();
            return Err(LaunchError::LaunchFailed(format!(
                "Java 进程已退出，退出码: {:?}\n\n请检查日志文件: {}\n\n日志内容:\n{}",
                status.code(),
                log_file.to_string_lossy(),
                if log_content.is_empty() { "（空，请查看控制台输出）" } else { &log_content }
            )));
        }
        
        Ok(())
    }
    
    /// 保存启动日志到文件
    fn save_launch_log(&self) -> Result<PathBuf, LaunchError> {
        let logs_dir = self.game_dir.join("logs");
        fs::create_dir_all(&logs_dir)?;
        
        let timestamp = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S");
        let log_file = logs_dir.join(format!("launch_{}.log", timestamp));
        
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&log_file)?;
        
        writeln!(file, "=== RTLauncher 启动日志 ===")?;
        writeln!(file, "时间: {}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"))?;
        writeln!(file, "版本: {}", self.config.version_id)?;
        writeln!(file, "Java: {} ({})", self.config.java.path.to_string_lossy(), self.config.java.major_version)?;
        writeln!(file, "游戏目录: {}", self.game_dir.to_string_lossy())?;
        writeln!(file, "账户: {} ({:?})", self.account.username, self.account.account_type)?;
        writeln!(file)?;
        writeln!(file, "=== JVM 参数 ===")?;
        
        let jvm_args = self.build_jvm_args()?;
        for arg in &jvm_args {
            writeln!(file, "  {}", arg)?;
        }
        
        writeln!(file)?;
        writeln!(file, "=== 主类 ===")?;
        writeln!(file, "  {}", self.version_detail.main_class)?;
        
        writeln!(file)?;
        writeln!(file, "=== 游戏参数 ===")?;
        let game_args = self.build_game_args()?;
        for arg in &game_args {
            writeln!(file, "  {}", arg)?;
        }
        
        writeln!(file)?;
        writeln!(file, "=== 游戏输出 ===")?;
        
        Ok(log_file)
    }
    
    /// 构建JVM参数
    fn build_jvm_args(&self) -> Result<Vec<String>, LaunchError> {
        let mut args = Vec::new();
        
        // 1. 内存设置（最前面）
        args.push(format!("-Xms{}M", self.config.min_memory));
        args.push(format!("-Xmx{}M", self.config.max_memory));
        
        // 2. 基础JVM参数
        args.push(format!("-Djava.library.path={}", 
            get_natives_dir(&self.config.version_id).to_string_lossy()));
        args.push("-Dminecraft.launcher.brand=RTLauncher".to_string());
        args.push("-Dminecraft.launcher.version=1.0.0".to_string());
        
        // 3. 添加版本定义的JVM参数（可能包含 -cp 等）
        if let Some(arguments) = &self.version_detail.arguments {
            if let Some(jvm_args) = &arguments.jvm {
                for arg in jvm_args {
                    match arg {
                        ArgumentValue::String(s) => {
                            args.push(self.replace_placeholders(s));
                        }
                        ArgumentValue::Object { value, rules } => {
                            if rules.iter().all(|r| r.is_allowed()) {
                                match value {
                                    ArgumentValueInner::String(s) => {
                                        args.push(self.replace_placeholders(s));
                                    }
                                    ArgumentValueInner::Strings(ss) => {
                                        for s in ss {
                                            args.push(self.replace_placeholders(s));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // 4. 如果版本没有定义 -cp，我们手动添加
        if !args.iter().any(|a| a == "-cp" || a == "-classpath") {
            let classpath = self.build_classpath()?;
            args.push("-cp".to_string());
            args.push(classpath);
        }
        
        // 5. 添加自定义JVM参数
        args.extend(self.config.jvm_args.clone());
        
        Ok(args)
    }
    
    /// 构建游戏参数
    fn build_game_args(&self) -> Result<Vec<String>, LaunchError> {
        let mut args = Vec::new();
        
        // 检查是新版参数格式还是旧版
        if let Some(arguments) = &self.version_detail.arguments {
            if let Some(game_args) = &arguments.game {
                for arg in game_args {
                    match arg {
                        ArgumentValue::String(s) => {
                            args.push(self.replace_placeholders(s));
                        }
                        ArgumentValue::Object { value, rules } => {
                            if rules.iter().all(|r| r.is_allowed()) {
                                match value {
                                    ArgumentValueInner::String(s) => {
                                        args.push(self.replace_placeholders(s));
                                    }
                                    ArgumentValueInner::Strings(ss) => {
                                        for s in ss {
                                            args.push(self.replace_placeholders(s));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        } else if let Some(mc_args) = &self.version_detail.minecraft_arguments {
            // 旧版参数格式
            for arg in mc_args.split(' ') {
                args.push(self.replace_placeholders(arg));
            }
        }
        
        // 添加自定义游戏参数
        args.extend(self.config.game_args.clone());
        
        // 添加服务器参数
        if let Some(server) = &self.config.server {
            if server.contains(':') {
                let parts: Vec<&str> = server.split(':').collect();
                args.push("--server".to_string());
                args.push(parts[0].to_string());
                args.push("--port".to_string());
                args.push(parts.get(1).unwrap_or(&"25565").to_string());
            } else {
                args.push("--server".to_string());
                args.push(server.clone());
                args.push("--port".to_string());
                args.push("25565".to_string());
            }
        }
        
        Ok(args)
    }
    
    /// 构建类路径
    fn build_classpath(&self) -> Result<String, LaunchError> {
        let mut paths = Vec::new();
        let libraries_dir = get_libraries_dir();
        
        // 添加库文件
        for library in &self.version_detail.libraries {
            if !library.is_allowed() {
                continue;
            }
            
            // 检查是否有普通 artifact（非 native）
            if let Some(downloads) = &library.downloads {
                if let Some(artifact) = &downloads.artifact {
                    let path = library.get_path(None)
                        .unwrap_or_else(|| artifact.path.clone().unwrap_or_default());
                    
                    let lib_path = libraries_dir.join(&path);
                    if lib_path.exists() {
                        paths.push(lib_path);
                        continue;
                    }
                }
            }
            
            // 如果没有 artifact 但有路径，也添加
            if let Some(path) = library.get_path(None) {
                let lib_path = libraries_dir.join(&path);
                if lib_path.exists() {
                    paths.push(lib_path);
                }
            }
        }
        
        // 添加客户端 JAR（最后）
        let client_jar = get_version_dir(&self.config.version_id)
            .join(format!("{}.jar", self.config.version_id));
        
        if !client_jar.exists() {
            return Err(LaunchError::MissingFile(format!("客户端JAR: {:?}", client_jar)));
        }
        
        paths.push(client_jar);
        
        // 构建类路径字符串
        #[cfg(target_os = "windows")]
        let separator = ";";
        #[cfg(not(target_os = "windows"))]
        let separator = ":";
        
        Ok(paths.iter()
            .map(|p: &PathBuf| p.to_string_lossy().to_string())
            .collect::<Vec<_>>()
            .join(separator))
    }
    
    /// 替换占位符
    fn replace_placeholders(&self, s: &str) -> String {
        let mut result = s.to_string();
        
        let version_name = &self.config.version_id;
        let game_dir = &self.game_dir;
        let assets_dir = get_assets_dir();
        let libraries_dir = get_libraries_dir();
        let version_dir = get_version_dir(version_name);
        let assets_id = self.version_detail.assets.as_deref().unwrap_or("legacy");
        
        // 账户相关
        let uuid = self.account.uuid.to_string();
        let uuid_no_dashes = uuid.replace('-', "");
        let access_token = self.account.access_token.as_deref().unwrap_or("");
        let username = &self.account.username;
        let user_type = match self.account.account_type {
            crate::account::types::AccountType::Offline => "Legacy",
            crate::account::types::AccountType::Microsoft => "msa",
            crate::account::types::AccountType::Yggdrasil => "mojang",
        };
        
        // 版本类型
        let version_type = match self.version_detail.version_type {
            VersionType::Release => "release",
            VersionType::Snapshot => "snapshot",
            VersionType::OldAlpha => "old_alpha",
            VersionType::OldBeta => "old_beta",
        };
        
        // 类路径分隔符
        #[cfg(target_os = "windows")]
        let cp_separator = ";";
        #[cfg(not(target_os = "windows"))]
        let cp_separator = ":";
        
        // 替换所有占位符
        result = result.replace("${auth_player_name}", username);
        result = result.replace("${version_name}", version_name);
        result = result.replace("${game_directory}", &game_dir.to_string_lossy());
        result = result.replace("${assets_root}", &assets_dir.to_string_lossy());
        result = result.replace("${assets_index_name}", assets_id);
        result = result.replace("${auth_uuid}", &uuid_no_dashes);
        result = result.replace("${auth_access_token}", access_token);
        result = result.replace("${user_type}", user_type);
        result = result.replace("${version_type}", version_type);
        result = result.replace("${natives_directory}", &get_natives_dir(version_name).to_string_lossy());
        result = result.replace("${launcher_name}", "RTLauncher");
        result = result.replace("${launcher_version}", "1.0.0");
        result = result.replace("${classpath_separator}", cp_separator);
        result = result.replace("${auth_session}", access_token);
        result = result.replace("${library_directory}", &libraries_dir.to_string_lossy());
        result = result.replace("${version_directory}", &version_dir.to_string_lossy());
        result = result.replace("${resolution_width}", &self.config.width.to_string());
        result = result.replace("${resolution_height}", &self.config.height.to_string());
        
        result
    }
}
