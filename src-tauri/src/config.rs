use std::path::PathBuf;
use std::fs;
use serde::{Deserialize, Serialize};

/// 启动器配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LauncherConfig {
    /// 选中的 Java 路径 (JAVA_HOME)
    pub selected_java_path: Option<String>,
    /// 手动添加的 Java 列表
    pub manual_java_paths: Vec<ManualJavaEntry>,
    /// 最小内存 (MB)
    pub min_memory: u32,
    /// 最大内存 (MB)
    pub max_memory: u32,
    /// 游戏目录
    pub game_dir: Option<String>,
    /// 选中的版本 ID
    pub selected_version: Option<String>,
}

/// 手动添加的 Java 条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManualJavaEntry {
    pub path: String,
    pub version: String,
    pub major_version: u32,
    pub vendor: String,
    pub arch: String,
}

impl LauncherConfig {
    /// 获取配置文件路径
    fn get_config_path() -> Result<PathBuf, String> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| "无法获取配置目录".to_string())?
            .join("RTLauncher");
        Ok(config_dir.join("config.json"))
    }

    /// 加载配置
    pub fn load() -> Result<Self, String> {
        let path = Self::get_config_path()?;
        
        if !path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(&path)
            .map_err(|e| format!("读取配置文件失败: {}", e))?;

        serde_json::from_str(&content)
            .map_err(|e| format!("解析配置文件失败: {}", e))
    }

    /// 保存配置
    pub fn save(&self) -> Result<(), String> {
        let path = Self::get_config_path()?;
        
        // 确保目录存在
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("创建配置目录失败: {}", e))?;
        }

        let content = serde_json::to_string_pretty(self)
            .map_err(|e| format!("序列化配置失败: {}", e))?;

        fs::write(&path, content)
            .map_err(|e| format!("写入配置文件失败: {}", e))
    }

    /// 设置选中的 Java
    pub fn set_selected_java(&mut self, path: String) {
        self.selected_java_path = Some(path);
    }

    /// 添加手动 Java
    pub fn add_manual_java(&mut self, entry: ManualJavaEntry) {
        // 移除已存在的相同路径
        self.manual_java_paths.retain(|j| j.path != entry.path);
        self.manual_java_paths.push(entry);
    }

    /// 移除手动 Java
    pub fn remove_manual_java(&mut self, path: &str) -> bool {
        let initial_len = self.manual_java_paths.len();
        self.manual_java_paths.retain(|j| j.path != path);
        
        // 如果移除的是当前选中的，清除选择
        if self.selected_java_path.as_deref() == Some(path) {
            self.selected_java_path = None;
        }
        
        self.manual_java_paths.len() < initial_len
    }

    /// 设置内存
    pub fn set_memory(&mut self, min: u32, max: u32) {
        self.min_memory = min;
        self.max_memory = max;
    }

    /// 设置游戏目录
    pub fn set_game_dir(&mut self, path: String) {
        self.game_dir = Some(path);
    }

    /// 设置选中版本
    pub fn set_selected_version(&mut self, version: String) {
        self.selected_version = Some(version);
    }
}
