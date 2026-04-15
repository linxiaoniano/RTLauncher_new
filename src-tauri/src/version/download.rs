use reqwest::Client;
use sha1::{Sha1, Digest};
use std::path::{Path, PathBuf};
use std::fs;
use std::sync::RwLock;
use thiserror::Error;
use tokio::io::AsyncWriteExt;
use futures::stream::{self, StreamExt};

/// 全局游戏目录（可以被外部设置）
static CUSTOM_GAME_DIR: RwLock<Option<PathBuf>> = RwLock::new(None);

/// 设置自定义游戏目录
pub fn set_custom_game_dir(path: PathBuf) {
    if let Ok(mut dir) = CUSTOM_GAME_DIR.write() {
        *dir = Some(path);
    }
}

/// 获取自定义游戏目录
pub fn get_custom_game_dir() -> Option<PathBuf> {
    CUSTOM_GAME_DIR.read().ok().and_then(|d| d.clone())
}

#[derive(Error, Debug)]
pub enum DownloadError {
    #[error("HTTP请求失败: {0}")]
    HttpError(#[from] reqwest::Error),
    
    #[error("IO错误: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("SHA1校验失败: 期望 {expected}, 实际 {actual}")]
    Sha1Mismatch { expected: String, actual: String },
    
    #[error("解压失败: {0}")]
    ExtractError(String),
}

/// 下载文件并校验SHA1
pub async fn download_file(
    client: &Client,
    url: &str,
    dest: &Path,
    expected_sha1: Option<&str>,
) -> Result<(), DownloadError> {
    // 确保父目录存在
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }
    
    // 发送请求
    let response = client.get(url).send().await?;
    
    // 获取响应流
    let mut stream = response.bytes_stream();
    
    // 创建文件
    let mut file = tokio::fs::File::create(dest).await?;
    
    // 计算SHA1
    let mut hasher = Sha1::new();
    
    // 写入文件
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        hasher.update(&chunk);
        file.write_all(&chunk).await?;
    }
    
    file.flush().await?;
    
    // 校验SHA1
    if let Some(expected) = expected_sha1 {
        let actual = format!("{:x}", hasher.finalize());
        if !expected.eq_ignore_ascii_case(&actual) {
            // 删除损坏的文件
            let _ = fs::remove_file(dest);
            return Err(DownloadError::Sha1Mismatch {
                expected: expected.to_string(),
                actual,
            });
        }
    }
    
    Ok(())
}

/// 批量下载文件
pub async fn download_files(
    client: &Client,
    files: Vec<DownloadTask>,
    concurrent: usize,
    on_progress: impl Fn(&str, u64, u64) + Send + Sync + 'static,
) -> Result<u64, DownloadError> {
    let client = client.clone();
    let on_progress = std::sync::Arc::new(on_progress);
    
    let results: Vec<Result<(), DownloadError>> = stream::iter(files)
        .map(|task| {
            let client = client.clone();
            let on_progress = on_progress.clone();
            async move {
                download_file(&client, &task.url, &task.dest, task.sha1.as_deref()).await?;
                on_progress(&task.name, task.size, task.size);
                Ok(())
            }
        })
        .buffer_unordered(concurrent)
        .collect()
        .await;
    
    let mut failed = 0;
    for result in results {
        if result.is_err() {
            failed += 1;
        }
    }
    
    Ok(failed)
}

/// 下载任务
pub struct DownloadTask {
    pub name: String,
    pub url: String,
    pub dest: PathBuf,
    pub sha1: Option<String>,
    pub size: u64,
}

/// 解压 natives 文件
pub fn extract_native(
    zip_path: &Path,
    dest_dir: &Path,
    exclude: Option<&[String]>,
) -> Result<(), DownloadError> {
    fs::create_dir_all(dest_dir)?;
    
    let file = fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| DownloadError::ExtractError(e.to_string()))?;
    
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)
            .map_err(|e| DownloadError::ExtractError(e.to_string()))?;
        
        let outpath = match file.enclosed_name() {
            Some(path) => dest_dir.join(path),
            None => continue,
        };
        
        // 检查排除列表
        if let Some(exclude_patterns) = exclude {
            let path_str = outpath.to_string_lossy();
            let should_exclude = exclude_patterns.iter().any(|pattern| {
                path_str.contains(pattern)
            });
            if should_exclude {
                continue;
            }
        }
        
        if file.name().ends_with('/') {
            fs::create_dir_all(&outpath)?;
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    fs::create_dir_all(p)?;
                }
            }
            let mut outfile = fs::File::create(&outpath)?;
            std::io::copy(&mut file, &mut outfile)?;
        }
    }
    
    // 删除 zip 文件
    fs::remove_file(zip_path)?;
    
    Ok(())
}

/// 获取游戏目录
pub fn get_game_dir() -> PathBuf {
    // 优先使用自定义游戏目录
    if let Some(custom) = get_custom_game_dir() {
        return custom;
    }
    
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("RTLauncher")
        .join("minecraft")
}

/// 获取版本目录
pub fn get_version_dir(version_id: &str) -> PathBuf {
    get_game_dir().join("versions").join(version_id)
}

/// 获取库文件目录
pub fn get_libraries_dir() -> PathBuf {
    get_game_dir().join("libraries")
}

/// 获取资源文件目录
pub fn get_assets_dir() -> PathBuf {
    get_game_dir().join("assets")
}

/// 获取 natives 目录
pub fn get_natives_dir(version_id: &str) -> PathBuf {
    get_version_dir(version_id).join("natives")
}

/// 检查文件是否存在且SHA1正确
pub fn check_file_exists(path: &Path, expected_sha1: Option<&str>) -> bool {
    if !path.exists() {
        return false;
    }
    
    if let Some(expected) = expected_sha1 {
        let content = match fs::read(path) {
            Ok(c) => c,
            Err(_) => return false,
        };
        
        let mut hasher = Sha1::new();
        hasher.update(&content);
        let actual = format!("{:x}", hasher.finalize());
        
        return expected.eq_ignore_ascii_case(&actual);
    }
    
    true
}
