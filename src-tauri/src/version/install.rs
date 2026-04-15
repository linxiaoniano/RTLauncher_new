use super::types::*;
use super::manifest::*;
use super::download::{
    DownloadTask, DownloadError,
    download_file, download_files, extract_native,
    get_game_dir, get_version_dir, get_libraries_dir, get_assets_dir, get_natives_dir,
    check_file_exists,
};
use reqwest::Client;
use std::path::PathBuf;
use std::fs;
use std::sync::Arc;
use std::time::Instant;
use thiserror::Error;
use tokio::sync::RwLock;

#[derive(Error, Debug)]
pub enum InstallError {
    #[error("下载错误: {0}")]
    DownloadError(#[from] DownloadError),
    
    #[error("清单错误: {0}")]
    ManifestError(#[from] ManifestError),
    
    #[error("IO错误: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("JSON解析错误: {0}")]
    JsonError(#[from] serde_json::Error),
    
    #[error("版本已存在: {0}")]
    VersionExists(String),
    
    #[error("缺少必要信息: {0}")]
    MissingInfo(String),
}

// 为 Tauri 实现序列化
impl serde::Serialize for InstallError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

/// 安装进度回调
pub type ProgressCallback = Arc<dyn Fn(InstallProgress) + Send + Sync>;

/// 安装进度
#[derive(Debug, Clone)]
pub struct InstallProgress {
    pub stage: InstallStage,
    pub current: u64,
    pub total: u64,
    pub message: String,
}

#[derive(Debug, Clone)]
pub enum InstallStage {
    DownloadingJson,
    DownloadingClient,
    DownloadingLibraries,
    DownloadingAssets,
    ExtractingNatives,
    Completed,
}

/// 版本安装器
pub struct VersionInstaller {
    client: Client,
    progress: ProgressCallback,
}

impl VersionInstaller {
    pub fn new(client: Client, progress: ProgressCallback) -> Self {
        Self { client, progress }
    }
    
    /// 安装版本
    pub async fn install(&self, version_id: &str) -> Result<InstalledVersion, InstallError> {
        let start_time = Instant::now();
        
        // 检查版本是否已存在
        let version_dir = get_version_dir(version_id);
        let version_json = version_dir.join(format!("{}.json", version_id));
        
        if version_json.exists() {
            return Err(InstallError::VersionExists(version_id.to_string()));
        }
        
        // 获取版本清单
        self.progress(InstallProgress {
            stage: InstallStage::DownloadingJson,
            current: 0,
            total: 1,
            message: "获取版本清单...".to_string(),
        });
        
        let manifest = get_version_manifest(&self.client).await?;
        let detail = get_version_detail(&self.client, version_id, &manifest).await?;
        
        // 保存版本 JSON
        fs::create_dir_all(&version_dir)?;
        let json_content = serde_json::to_string_pretty(&detail)?;
        fs::write(&version_json, json_content)?;
        
        // 下载客户端 JAR
        self.download_client(&detail).await?;
        
        // 下载库文件
        self.download_libraries(&detail).await?;
        
        // 下载资源文件
        self.download_assets(&detail).await?;
        
        // 解压 natives
        self.extract_natives(&detail).await?;
        
        // 构建已安装版本信息
        let installed = InstalledVersion {
            id: detail.id.clone(),
            version_type: detail.version_type.clone(),
            main_class: detail.main_class.clone(),
            java_version: detail.java_version.as_ref().map(|j| j.major_version).unwrap_or(8),
            assets_id: detail.assets.clone().unwrap_or_default(),
            libraries_count: detail.libraries.len(),
            installed_at: chrono::Utc::now().timestamp(),
        };
        
        let elapsed = start_time.elapsed();
        self.progress(InstallProgress {
            stage: InstallStage::Completed,
            current: 100,
            total: 100,
            message: format!("安装完成，耗时 {:?}", elapsed),
        });
        
        Ok(installed)
    }
    
    /// 下载客户端 JAR
    async fn download_client(&self, detail: &VersionDetail) -> Result<(), InstallError> {
        let downloads = detail.downloads.as_ref()
            .ok_or_else(|| InstallError::MissingInfo("downloads".to_string()))?;
        let client_download = downloads.client.as_ref()
            .ok_or_else(|| InstallError::MissingInfo("client download".to_string()))?;
        
        let version_dir = get_version_dir(&detail.id);
        let client_jar = version_dir.join(format!("{}.jar", detail.id));
        
        self.progress(InstallProgress {
            stage: InstallStage::DownloadingClient,
            current: 0,
            total: client_download.size,
            message: "下载客户端...".to_string(),
        });
        
        download_file(
            &self.client,
            &client_download.url,
            &client_jar,
            Some(&client_download.sha1),
        ).await?;
        
        Ok(())
    }
    
    /// 下载库文件
    async fn download_libraries(&self, detail: &VersionDetail) -> Result<(), InstallError> {
        let libraries_dir = get_libraries_dir();
        let mut tasks = Vec::new();
        
        for library in &detail.libraries {
            if !library.is_allowed() {
                continue;
            }
            
            // 下载普通库
            if let Some(downloads) = &library.downloads {
                if let Some(artifact) = &downloads.artifact {
                    let path = library.get_path(None)
                        .or_else(|| artifact.path.clone())
                        .unwrap_or_default();
                    
                    let dest = libraries_dir.join(&path);
                    
                    if !check_file_exists(&dest, Some(&artifact.sha1)) {
                        tasks.push(DownloadTask {
                            name: library.name.clone(),
                            url: artifact.url.clone(),
                            dest,
                            sha1: Some(artifact.sha1.clone()),
                            size: artifact.size,
                        });
                    }
                }
                
                // 下载 natives
                if let Some(native_classifier) = library.get_native_classifier() {
                    if let Some(classifiers) = &downloads.classifiers {
                        if let Some(native_artifact) = classifiers.get(&native_classifier) {
                            let path = library.get_path(Some(&native_classifier))
                                .unwrap_or_default();
                            
                            let dest = libraries_dir.join(&path);
                            
                            if !check_file_exists(&dest, Some(&native_artifact.sha1)) {
                                tasks.push(DownloadTask {
                                    name: format!("{} (native)", library.name),
                                    url: native_artifact.url.clone(),
                                    dest,
                                    sha1: Some(native_artifact.sha1.clone()),
                                    size: native_artifact.size,
                                });
                            }
                        }
                    }
                }
            }
        }
        
        self.progress(InstallProgress {
            stage: InstallStage::DownloadingLibraries,
            current: 0,
            total: tasks.len() as u64,
            message: format!("下载 {} 个库文件...", tasks.len()),
        });
        
        let total_tasks = tasks.len() as u64;
        let completed = Arc::new(RwLock::new(0u64));
        
        let on_progress = {
            let completed = completed.clone();
            let progress = self.progress.clone();
            move |name: &str, _current: u64, _total: u64| {
                let completed = completed.clone();
                let progress = progress.clone();
                let name = name.to_string();
                tokio::spawn(async move {
                    let mut c = completed.write().await;
                    *c += 1;
                    progress(InstallProgress {
                        stage: InstallStage::DownloadingLibraries,
                        current: *c,
                        total: total_tasks,
                        message: format!("下载: {}", name),
                    });
                });
            }
        };
        
        download_files(&self.client, tasks, 8, on_progress).await?;
        
        Ok(())
    }
    
    /// 下载资源文件
    async fn download_assets(&self, detail: &VersionDetail) -> Result<(), InstallError> {
        let asset_index_info = detail.asset_index.as_ref()
            .ok_or_else(|| InstallError::MissingInfo("assetIndex".to_string()))?;
        
        let assets_dir = get_assets_dir();
        let index_dir = assets_dir.join("indexes");
        let objects_dir = assets_dir.join("objects");
        
        // 下载资源索引
        let index_file = index_dir.join(format!("{}.json", asset_index_info.id));
        fs::create_dir_all(&index_dir)?;
        
        download_file(
            &self.client,
            &asset_index_info.url,
            &index_file,
            Some(&asset_index_info.sha1),
        ).await?;
        
        // 读取索引
        let index_content = fs::read_to_string(&index_file)?;
        let asset_index: AssetIndex = serde_json::from_str(&index_content)?;
        
        // 收集需要下载的资源
        let mut tasks = Vec::new();
        for (name, object) in &asset_index.objects {
            let path = object.get_path();
            let dest = objects_dir.join(&path);
            
            if !check_file_exists(&dest, Some(&object.hash)) {
                let url = format!("https://resources.download.minecraft.net/{}", path);
                tasks.push(DownloadTask {
                    name: name.clone(),
                    url,
                    dest,
                    sha1: Some(object.hash.clone()),
                    size: object.size,
                });
            }
        }
        
        self.progress(InstallProgress {
            stage: InstallStage::DownloadingAssets,
            current: 0,
            total: tasks.len() as u64,
            message: format!("下载 {} 个资源文件...", tasks.len()),
        });
        
        let total_tasks = tasks.len() as u64;
        let completed = Arc::new(RwLock::new(0u64));
        
        let on_progress = {
            let completed = completed.clone();
            let progress = self.progress.clone();
            move |_name: &str, _current: u64, _total: u64| {
                let completed = completed.clone();
                let progress = progress.clone();
                tokio::spawn(async move {
                    let mut c = completed.write().await;
                    *c += 1;
                    progress(InstallProgress {
                        stage: InstallStage::DownloadingAssets,
                        current: *c,
                        total: total_tasks,
                        message: format!("下载资源 {}/{}", *c, total_tasks),
                    });
                });
            }
        };
        
        download_files(&self.client, tasks, 16, on_progress).await?;
        
        Ok(())
    }
    
    /// 解压 natives
    async fn extract_natives(&self, detail: &VersionDetail) -> Result<(), InstallError> {
        let natives_dir = get_natives_dir(&detail.id);
        let libraries_dir = get_libraries_dir();
        
        fs::create_dir_all(&natives_dir)?;
        
        self.progress(InstallProgress {
            stage: InstallStage::ExtractingNatives,
            current: 0,
            total: detail.libraries.len() as u64,
            message: "解压本地库...".to_string(),
        });
        
        let mut count = 0;
        for library in &detail.libraries {
            if !library.is_allowed() {
                continue;
            }
            
            if let Some(native_classifier) = library.get_native_classifier() {
                if let Some(downloads) = &library.downloads {
                    if let Some(classifiers) = &downloads.classifiers {
                        if let Some(native_artifact) = classifiers.get(&native_classifier) {
                            let path = library.get_path(Some(&native_classifier))
                                .unwrap_or_default();
                            
                            let zip_path = libraries_dir.join(&path);
                            
                            if zip_path.exists() {
                                let exclude = library.extract.as_ref().and_then(|e| e.exclude.as_deref());
                                extract_native(
                                    &zip_path,
                                    &natives_dir,
                                    exclude,
                                )?;
                                count += 1;
                            }
                        }
                    }
                }
            }
        }
        
        self.progress(InstallProgress {
            stage: InstallStage::ExtractingNatives,
            current: count,
            total: count,
            message: "解压完成".to_string(),
        });
        
        Ok(())
    }
    
    fn progress(&self, p: InstallProgress) {
        (self.progress)(p);
    }
}

/// 获取已安装的版本列表
pub fn get_installed_versions() -> Result<Vec<InstalledVersion>, InstallError> {
    let versions_dir = get_game_dir().join("versions");
    
    if !versions_dir.exists() {
        return Ok(Vec::new());
    }
    
    let mut versions = Vec::new();
    
    for entry in std::fs::read_dir(versions_dir)? {
        let entry = entry?;
        let path = entry.path();
        
        if path.is_dir() {
            let version_id = path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default();
            
            let json_path = path.join(format!("{}.json", version_id));
            
            if json_path.exists() {
                if let Ok(content) = std::fs::read_to_string(&json_path) {
                    if let Ok(detail) = serde_json::from_str::<VersionDetail>(&content) {
                        versions.push(InstalledVersion {
                            id: detail.id,
                            version_type: detail.version_type,
                            main_class: detail.main_class,
                            java_version: detail.java_version.as_ref().map(|j| j.major_version).unwrap_or(8),
                            assets_id: detail.assets.unwrap_or_default(),
                            libraries_count: detail.libraries.len(),
                            installed_at: 0, // 我们不知道安装时间，设为0
                        });
                    }
                }
            }
        }
    }
    
    Ok(versions)
}

/// 删除已安装的版本
pub fn delete_version(version_id: &str) -> Result<(), InstallError> {
    let version_dir = get_version_dir(version_id);
    
    if version_dir.exists() {
        fs::remove_dir_all(&version_dir)?;
    }
    
    Ok(())
}
