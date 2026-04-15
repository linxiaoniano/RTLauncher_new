use super::types::*;
use reqwest::Client;
use thiserror::Error;

const VERSION_MANIFEST_URL: &str = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";

#[derive(Error, Debug)]
pub enum ManifestError {
    #[error("HTTP请求失败: {0}")]
    HttpError(#[from] reqwest::Error),
    
    #[error("解析JSON失败: {0}")]
    ParseError(#[from] serde_json::Error),
    
    #[error("版本不存在: {0}")]
    VersionNotFound(String),
}

/// 获取版本清单
pub async fn get_version_manifest(client: &Client) -> Result<VersionManifest, ManifestError> {
    let response = client
        .get(VERSION_MANIFEST_URL)
        .send()
        .await?
        .json::<VersionManifest>()
        .await?;
    
    Ok(response)
}

/// 获取指定版本的详情
pub async fn get_version_detail(
    client: &Client,
    version_id: &str,
    manifest: &VersionManifest,
) -> Result<VersionDetail, ManifestError> {
    // 查找版本信息
    let version_info = manifest.versions
        .iter()
        .find(|v| v.id == version_id)
        .ok_or_else(|| ManifestError::VersionNotFound(version_id.to_string()))?;
    
    // 获取版本详情
    let response = client
        .get(&version_info.url)
        .send()
        .await?
        .json::<VersionDetail>()
        .await?;
    
    Ok(response)
}

/// 获取版本详情（通过URL）
pub async fn get_version_detail_by_url(
    client: &Client,
    url: &str,
) -> Result<VersionDetail, ManifestError> {
    let response = client
        .get(url)
        .send()
        .await?
        .json::<VersionDetail>()
        .await?;
    
    Ok(response)
}

/// 获取资源索引
pub async fn get_asset_index(
    client: &Client,
    asset_index: &AssetIndexInfo,
) -> Result<AssetIndex, ManifestError> {
    let response = client
        .get(&asset_index.url)
        .send()
        .await?
        .json::<AssetIndex>()
        .await?;
    
    Ok(response)
}

/// 按类型筛选版本
pub fn filter_versions_by_type(
    manifest: &VersionManifest,
    version_type: VersionType,
) -> Vec<&ManifestVersion> {
    manifest.versions
        .iter()
        .filter(|v| v.version_type == version_type)
        .collect()
}

/// 搜索版本
pub fn search_versions<'a>(
    manifest: &'a VersionManifest,
    query: &str,
) -> Vec<&'a ManifestVersion> {
    let query_lower = query.to_lowercase();
    manifest.versions
        .iter()
        .filter(|v| v.id.to_lowercase().contains(&query_lower))
        .collect()
}
