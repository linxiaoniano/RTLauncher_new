use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 版本类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum VersionType {
    Release,
    Snapshot,
    OldAlpha,
    OldBeta,
}

/// 版本清单中的版本信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionManifest {
    pub latest: LatestVersions,
    pub versions: Vec<ManifestVersion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatestVersions {
    pub release: String,
    pub snapshot: String,
}

/// 清单中的版本条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestVersion {
    pub id: String,
    #[serde(rename = "type")]
    pub version_type: VersionType,
    pub url: String,
    pub time: String,
    #[serde(rename = "releaseTime")]
    pub release_time: String,
    #[serde(default)]
    pub sha1: Option<String>,
}

/// 版本详情（完整的版本JSON）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionDetail {
    pub id: String,
    #[serde(rename = "type")]
    pub version_type: VersionType,
    #[serde(rename = "mainClass")]
    pub main_class: String,
    pub libraries: Vec<Library>,
    #[serde(rename = "assetIndex")]
    pub asset_index: Option<AssetIndexInfo>,
    pub assets: Option<String>,
    #[serde(rename = "javaVersion")]
    pub java_version: Option<JavaVersion>,
    #[serde(rename = "downloads")]
    pub downloads: Option<VersionDownloads>,
    #[serde(rename = "arguments")]
    pub arguments: Option<Arguments>,
    #[serde(rename = "minecraftArguments")]
    pub minecraft_arguments: Option<String>,
    #[serde(rename = "releaseTime")]
    pub release_time: String,
    pub time: String,
    #[serde(rename = "inheritsFrom")]
    pub inherits_from: Option<String>,
}

/// 库文件信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Library {
    pub name: String,
    pub downloads: Option<LibraryDownloads>,
    pub natives: Option<HashMap<String, String>>,
    pub extract: Option<ExtractConfig>,
    pub rules: Option<Vec<Rule>>,
}

impl Library {
    /// 解析库名获取路径
    /// 格式: group:artifact:version[:classifier]
    pub fn get_path(&self, native: Option<&str>) -> Option<String> {
        let parts: Vec<&str> = self.name.split(':').collect();
        if parts.len() < 3 {
            return None;
        }
        
        let group = parts[0].replace('.', "/");
        let artifact = parts[1];
        let version = parts[2];
        
        let filename = if let Some(n) = native {
            format!("{}-{}-{}.jar", artifact, version, n)
        } else if parts.len() > 3 {
            format!("{}-{}-{}.jar", artifact, version, parts[3])
        } else {
            format!("{}-{}.jar", artifact, version)
        };
        
        Some(format!("{}/{}/{}/{}", group, artifact, version, filename))
    }
    
    /// 获取当前平台的 native 分类器
    pub fn get_native_classifier(&self) -> Option<String> {
        let natives = self.natives.as_ref()?;
        
        #[cfg(target_os = "windows")]
        let key = "windows";
        #[cfg(target_os = "macos")]
        let key = "osx";
        #[cfg(target_os = "linux")]
        let key = "linux";
        
        natives.get(key).map(|s| {
            s.replace("${arch}", std::env::consts::ARCH)
        })
    }
    
    /// 检查规则是否允许使用此库
    pub fn is_allowed(&self) -> bool {
        let rules = match &self.rules {
            Some(r) => r,
            None => return true,
        };
        
        for rule in rules {
            if rule.is_allowed() {
                return true;
            }
        }
        false
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryDownloads {
    pub artifact: Option<ArtifactInfo>,
    pub classifiers: Option<HashMap<String, ArtifactInfo>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactInfo {
    pub url: String,
    pub size: u64,
    pub sha1: String,
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractConfig {
    pub exclude: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub action: String,
    pub os: Option<OsRule>,
    pub features: Option<HashMap<String, bool>>,
}

impl Rule {
    pub fn is_allowed(&self) -> bool {
        let mut allowed = self.action == "allow";
        
        if let Some(os_rule) = &self.os {
            let os_matches = os_rule.matches_current_os();
            if os_matches {
                allowed = self.action == "allow";
            } else {
                allowed = self.action == "disallow";
            }
        }
        
        allowed
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OsRule {
    pub name: Option<String>,
    pub arch: Option<String>,
    pub version: Option<String>,
}

impl OsRule {
    pub fn matches_current_os(&self) -> bool {
        if let Some(name) = &self.name {
            let current_os = match std::env::consts::OS {
                "windows" => "windows",
                "macos" => "osx",
                "linux" => "linux",
                _ => "unknown",
            };
            
            if !name.to_lowercase().contains(current_os) {
                return false;
            }
        }
        
        if let Some(arch) = &self.arch {
            if !arch.to_lowercase().contains(std::env::consts::ARCH) {
                return false;
            }
        }
        
        true
    }
}

/// 资源索引信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetIndexInfo {
    pub id: String,
    pub sha1: String,
    pub size: u64,
    #[serde(rename = "totalSize")]
    pub total_size: u64,
    pub url: String,
}

/// 资源索引内容
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetIndex {
    pub objects: HashMap<String, AssetObject>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetObject {
    pub hash: String,
    pub size: u64,
}

impl AssetObject {
    /// 获取资源文件存储路径（基于hash前两位）
    pub fn get_path(&self) -> String {
        format!("{}/{}", &self.hash[..2], self.hash)
    }
}

/// Java 版本要求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JavaVersion {
    pub component: String,
    #[serde(rename = "majorVersion")]
    pub major_version: u32,
}

/// 版本下载信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionDownloads {
    pub client: Option<DownloadInfo>,
    pub server: Option<DownloadInfo>,
    #[serde(rename = "client_mappings")]
    pub client_mappings: Option<DownloadInfo>,
    #[serde(rename = "server_mappings")]
    pub server_mappings: Option<DownloadInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadInfo {
    pub sha1: String,
    pub size: u64,
    pub url: String,
}

/// 启动参数（新版本格式）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Arguments {
    pub game: Option<Vec<ArgumentValue>>,
    pub jvm: Option<Vec<ArgumentValue>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ArgumentValue {
    String(String),
    Object {
        value: ArgumentValueInner,
        rules: Vec<Rule>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ArgumentValueInner {
    String(String),
    Strings(Vec<String>),
}

/// 已安装的版本信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledVersion {
    pub id: String,
    pub version_type: VersionType,
    pub main_class: String,
    pub java_version: u32,
    pub assets_id: String,
    pub libraries_count: usize,
    pub installed_at: i64,
}

/// 下载进度
#[derive(Debug, Clone, Serialize)]
pub struct DownloadProgress {
    pub current: u64,
    pub total: u64,
    pub speed: u64, // bytes per second
    pub filename: String,
}

/// 下载任务状态
#[derive(Debug, Clone, Serialize)]
pub struct DownloadTask {
    pub name: String,
    pub progress: f64,
    pub status: DownloadStatus,
    pub speed: String,
    pub eta: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DownloadStatus {
    Pending,
    Downloading,
    Extracting,
    Completed,
    Failed,
}
