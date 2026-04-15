use super::types::*;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum YggdrasilError {
    #[error("HTTP请求失败: {0}")]
    HttpError(#[from] reqwest::Error),
    
    #[error("认证失败: {0}")]
    AuthFailed(String),
    
    #[error("无效的凭据")]
    InvalidCredentials,
    
    #[error("用户已迁移")]
    UserMigrated,
    
    #[error("服务器返回错误: {0}")]
    ServerError(String),
    
    #[error("解析响应失败: {0}")]
    ParseError(String),
    
    #[error("没有可用的游戏配置文件")]
    NoAvailableProfile,
}

#[derive(Debug, Deserialize)]
struct YggdrasilErrorResponse {
    error: String,
    error_message: String,
    cause: Option<String>,
}

/// Yggdrasil 登录请求
#[derive(Serialize)]
struct YggdrasilAuthRequest {
    username: String,
    password: String,
    client_token: String,
    request_user: bool,
}

/// 刷新令牌请求
#[derive(Serialize)]
struct YggdrasilRefreshRequest {
    access_token: String,
    client_token: String,
    request_user: bool,
}

/// 登录到 Yggdrasil 服务器
pub async fn login(
    client: &Client,
    email: &str,
    password: &str,
    server_url: &str,
) -> Result<Account, YggdrasilError> {
    let client_token = uuid::Uuid::new_v4().to_string();
    
    let auth_url = format!("{}/authserver/authenticate", server_url.trim_end_matches('/'));
    
    let request = YggdrasilAuthRequest {
        username: email.to_string(),
        password: password.to_string(),
        client_token: client_token.clone(),
        request_user: true,
    };

    let response = client
        .post(&auth_url)
        .json(&request)
        .send()
        .await?;

    if !response.status().is_success() {
        let error: YggdrasilErrorResponse = response.json().await?;
        return Err(map_yggdrasil_error(&error.error, &error.error_message));
    }

    let auth_response: YggdrasilAuthResponse = response.json().await?;

    // 选择一个配置文件（如果没有selected_profile，使用第一个available）
    let profile = auth_response.selected_profile
        .or_else(|| auth_response.available_profiles.first().cloned())
        .ok_or(YggdrasilError::NoAvailableProfile)?;

    // 获取皮肤信息
    let (skin_url, cape_url) = fetch_skin_info(client, server_url, &profile.id).await
        .unwrap_or((None, None));

    Ok(Account {
        id: uuid::Uuid::new_v4(),
        account_type: AccountType::Yggdrasil,
        username: profile.name,
        uuid: profile.id,
        skin_url,
        cape_url,
        access_token: Some(auth_response.access_token),
        refresh_token: Some(auth_response.client_token),
        expires_at: None, // Yggdrasil 通常没有过期时间
        auth_server_url: Some(server_url.to_string()),
    })
}

/// 刷新 Yggdrasil 令牌
pub async fn refresh_token(
    client: &Client,
    account: &Account,
) -> Result<Account, YggdrasilError> {
    let access_token = account.access_token.as_ref()
        .ok_or_else(|| YggdrasilError::AuthFailed("没有访问令牌".to_string()))?;
    let client_token = account.refresh_token.as_ref()
        .ok_or_else(|| YggdrasilError::AuthFailed("没有客户端令牌".to_string()))?;
    let server_url = account.auth_server_url.as_ref()
        .ok_or_else(|| YggdrasilError::AuthFailed("没有服务器URL".to_string()))?;

    let refresh_url = format!("{}/authserver/refresh", server_url.trim_end_matches('/'));
    
    let request = YggdrasilRefreshRequest {
        access_token: access_token.clone(),
        client_token: client_token.clone(),
        request_user: true,
    };

    let response = client
        .post(&refresh_url)
        .json(&request)
        .send()
        .await?;

    if !response.status().is_success() {
        let error: YggdrasilErrorResponse = response.json().await?;
        return Err(map_yggdrasil_error(&error.error, &error.error_message));
    }

    let auth_response: YggdrasilAuthResponse = response.json().await?;

    // 更新账户信息
    let mut updated_account = account.clone();
    updated_account.access_token = Some(auth_response.access_token);
    
    if let Some(profile) = auth_response.selected_profile {
        updated_account.username = profile.name;
        updated_account.uuid = profile.id;
    }

    Ok(updated_account)
}

/// 验证令牌是否有效
pub async fn validate_token(
    client: &Client,
    access_token: &str,
    client_token: &str,
    server_url: &str,
) -> Result<bool, YggdrasilError> {
    let validate_url = format!("{}/authserver/validate", server_url.trim_end_matches('/'));
    
    #[derive(Serialize)]
    struct ValidateRequest {
        access_token: String,
        client_token: String,
    }

    let request = ValidateRequest {
        access_token: access_token.to_string(),
        client_token: client_token.to_string(),
    };

    let response = client
        .post(&validate_url)
        .json(&request)
        .send()
        .await?;

    Ok(response.status().is_success())
}

/// 登出
pub async fn signout(
    client: &Client,
    username: &str,
    password: &str,
    server_url: &str,
) -> Result<(), YggdrasilError> {
    let signout_url = format!("{}/authserver/signout", server_url.trim_end_matches('/'));
    
    #[derive(Serialize)]
    struct SignoutRequest {
        username: String,
        password: String,
    }

    let request = SignoutRequest {
        username: username.to_string(),
        password: password.to_string(),
    };

    let response = client
        .post(&signout_url)
        .json(&request)
        .send()
        .await?;

    if !response.status().is_success() {
        let error: YggdrasilErrorResponse = response.json().await?;
        return Err(map_yggdrasil_error(&error.error, &error.error_message));
    }

    Ok(())
}

/// 从服务器获取皮肤信息
async fn fetch_skin_info(
    client: &Client,
    server_url: &str,
    uuid: &uuid::Uuid,
) -> Result<(Option<String>, Option<String>), YggdrasilError> {
    let profile_url = format!(
        "{}/sessionserver/session/minecraft/profile/{}",
        server_url.trim_end_matches('/'),
        uuid
    );

    let response = client
        .get(&profile_url)
        .send()
        .await?;

    if !response.status().is_success() {
        return Ok((None, None));
    }

    #[derive(Deserialize)]
    struct ProfileInfo {
        properties: Vec<ProfileProperty>,
    }

    let profile: ProfileInfo = response.json().await?;

    // 查找 textures 属性
    for prop in profile.properties {
        if prop.name == "textures" {
            // 解码 base64 JSON
            if let Ok(decoded) = base64_decode(&prop.value) {
                if let Ok(textures) = serde_json::from_str::<TexturesInfo>(&decoded) {
                    let skin_url = textures.textures.skin.map(|s| s.url);
                    let cape_url = textures.textures.cape.map(|c| c.url);
                    return Ok((skin_url, cape_url));
                }
            }
        }
    }

    Ok((None, None))
}

#[derive(Deserialize)]
struct TexturesInfo {
    textures: Textures,
}

#[derive(Deserialize)]
struct Textures {
    skin: Option<Texture>,
    cape: Option<Texture>,
}

#[derive(Deserialize)]
struct Texture {
    url: String,
}

fn base64_decode(input: &str) -> Result<String, YggdrasilError> {
    use base64::Engine;
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(input)
        .map_err(|e| YggdrasilError::ParseError(e.to_string()))?;
    String::from_utf8(decoded)
        .map_err(|e| YggdrasilError::ParseError(e.to_string()))
}

fn map_yggdrasil_error(error: &str, message: &str) -> YggdrasilError {
    match error {
        "ForbiddenOperationException" => {
            if message.contains("migrated") {
                YggdrasilError::UserMigrated
            } else {
                YggdrasilError::InvalidCredentials
            }
        }
        _ => YggdrasilError::ServerError(format!("{}: {}", error, message)),
    }
}
