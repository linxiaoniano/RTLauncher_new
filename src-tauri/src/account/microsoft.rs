use super::types::*;
use reqwest::Client;
use serde::Deserialize;
use thiserror::Error;
use chrono::Utc;

const AZURE_CLIENT_ID: &str = "00000000402b5328"; // Minecraft 官方客户端ID
const MICROSOFT_AUTH_URL: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode";
const MICROSOFT_TOKEN_URL: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/token";
const XBOX_AUTH_URL: &str = "https://user.auth.xboxlive.com/user/authenticate";
const XBOX_XSTS_URL: &str = "https://xsts.auth.xboxlive.com/xsts/authorize";
const MC_AUTH_URL: &str = "https://api.minecraftservices.com/authentication/login_with_xbox";
const MC_PROFILE_URL: &str = "https://api.minecraftservices.com/minecraft/profile";

#[derive(Error, Debug)]
pub enum MicrosoftAuthError {
    #[error("HTTP请求失败: {0}")]
    HttpError(#[from] reqwest::Error),
    
    #[error("用户取消了授权")]
    UserCancelled,
    
    #[error("设备码已过期")]
    DeviceCodeExpired,
    
    #[error("授权正在等待中")]
    AuthorizationPending,
    
    #[error("Xbox认证失败: {0}")]
    XboxAuthError(String),
    
    #[error("用户没有Minecraft")]
    NoMinecraftOwnership,
    
    #[error("解析响应失败: {0}")]
    ParseError(String),
}

/// 步骤1：获取设备码
pub async fn get_device_code(client: &Client) -> Result<MicrosoftDeviceCodeResponse, MicrosoftAuthError> {
    let response = client
        .post(MICROSOFT_AUTH_URL)
        .form(&[
            ("client_id", AZURE_CLIENT_ID),
            ("scope", "XboxLive.signin offline_access"),
        ])
        .send()
        .await?;

    let device_code = response.json::<MicrosoftDeviceCodeResponse>().await?;
    Ok(device_code)
}

/// 步骤2：轮询获取令牌
pub async fn poll_for_token(client: &Client, device_code: &str) -> Result<MicrosoftTokenResponse, MicrosoftAuthError> {
    let response = client
        .post(MICROSOFT_TOKEN_URL)
        .form(&[
            ("client_id", AZURE_CLIENT_ID),
            ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ("device_code", device_code),
        ])
        .send()
        .await?;

    let status = response.status();
    let body = response.text().await?;

    if status.is_success() {
        let token: MicrosoftTokenResponse = serde_json::from_str(&body)
            .map_err(|e| MicrosoftAuthError::ParseError(e.to_string()))?;
        Ok(token)
    } else {
        // 解析错误响应
        #[derive(Deserialize)]
        struct TokenErrorResponse {
            error: String,
        }
        let error: TokenErrorResponse = serde_json::from_str(&body)
            .map_err(|e| MicrosoftAuthError::ParseError(e.to_string()))?;
        
        match error.error.as_str() {
            "authorization_pending" => Err(MicrosoftAuthError::AuthorizationPending),
            "expired_token" => Err(MicrosoftAuthError::DeviceCodeExpired),
            _ => Err(MicrosoftAuthError::ParseError(error.error)),
        }
    }
}

/// 步骤3：获取 Xbox Live 令牌
pub async fn get_xbox_live_token(client: &Client, ms_access_token: &str) -> Result<(String, String), MicrosoftAuthError> {
    #[derive(Serialize)]
    #[serde(rename_all = "PascalCase")]
    struct XboxAuthRequest {
        relying_party: String,
        token_type: String,
        properties: XboxAuthProperties,
    }

    #[derive(Serialize)]
    #[serde(rename_all = "PascalCase")]
    struct XboxAuthProperties {
        auth_method: String,
        site_name: String,
        rps_ticket: String,
    }

    let request = XboxAuthRequest {
        relying_party: "http://auth.xboxlive.com".to_string(),
        token_type: "JWT".to_string(),
        properties: XboxAuthProperties {
            auth_method: "RPS".to_string(),
            site_name: "user.auth.xboxlive.com".to_string(),
            rps_ticket: format!("d={}", ms_access_token),
        },
    };

    let response = client
        .post(XBOX_AUTH_URL)
        .json(&request)
        .send()
        .await?;

    if !response.status().is_success() {
        let body = response.text().await?;
        return Err(MicrosoftAuthError::XboxAuthError(body));
    }

    let xbox_response: XboxLiveAuthResponse = response.json().await?;
    let user_hash = xbox_response.display_claims.xui[0].uhs.clone();
    
    Ok((xbox_response.token, user_hash))
}

use serde::Serialize;

/// 步骤4：获取 Xbox XSTS 令牌
pub async fn get_xsts_token(client: &Client, xbox_token: &str) -> Result<(String, String), MicrosoftAuthError> {
    #[derive(Serialize)]
    #[serde(rename_all = "PascalCase")]
    struct XSTSRequest {
        relying_party: String,
        token_type: String,
        properties: XSTSProperties,
    }

    #[derive(Serialize)]
    #[serde(rename_all = "PascalCase")]
    struct XSTSProperties {
        sandbox_id: String,
        user_tokens: Vec<String>,
    }

    let request = XSTSRequest {
        relying_party: "rp://api.minecraftservices.com/".to_string(),
        token_type: "JWT".to_string(),
        properties: XSTSProperties {
            sandbox_id: "RETAIL".to_string(),
            user_tokens: vec![xbox_token.to_string()],
        },
    };

    let response = client
        .post(XBOX_XSTS_URL)
        .json(&request)
        .send()
        .await?;

    if !response.status().is_success() {
        let body = response.text().await?;
        return Err(MicrosoftAuthError::XboxAuthError(body));
    }

    let xsts_response: XboxLiveAuthResponse = response.json().await?;
    let user_hash = xsts_response.display_claims.xui[0].uhs.clone();
    
    Ok((xsts_response.token, user_hash))
}

/// 步骤5：获取 Minecraft 令牌
pub async fn get_minecraft_token(client: &Client, user_hash: &str, xsts_token: &str) -> Result<String, MicrosoftAuthError> {
    #[derive(Serialize)]
    struct MCAuthRequest {
        identity_token: String,
    }

    let request = MCAuthRequest {
        identity_token: format!("XBL3.0 x={};{}", user_hash, xsts_token),
    };

    let response = client
        .post(MC_AUTH_URL)
        .json(&request)
        .send()
        .await?;

    #[derive(Deserialize)]
    struct MCAuthResponse {
        access_token: String,
        expires_in: i64,
    }

    let mc_response: MCAuthResponse = response.json().await?;
    Ok(mc_response.access_token)
}

/// 步骤6：获取 Minecraft Profile
pub async fn get_minecraft_profile(client: &Client, mc_token: &str) -> Result<MinecraftProfileResponse, MicrosoftAuthError> {
    let response = client
        .get(MC_PROFILE_URL)
        .bearer_auth(mc_token)
        .send()
        .await?;

    if response.status() == 404 {
        return Err(MicrosoftAuthError::NoMinecraftOwnership);
    }

    let profile: MinecraftProfileResponse = response.json().await?;
    Ok(profile)
}

/// 完整的微软登录流程
pub async fn login_with_microsoft(
    client: &Client,
    device_code: &str,
    refresh_token: Option<&str>,
) -> Result<Account, MicrosoftAuthError> {
    // 如果有refresh_token，直接刷新
    let ms_token = if let Some(refresh) = refresh_token {
        refresh_microsoft_token(client, refresh).await?
    } else {
        // 轮询获取令牌
        poll_for_token(client, device_code).await?
    };

    // Xbox Live 认证
    let (xbox_token, _user_hash) = get_xbox_live_token(client, &ms_token.access_token).await?;
    
    // XSTS 认证
    let (xsts_token, user_hash) = get_xsts_token(client, &xbox_token).await?;
    
    // 获取 Minecraft 令牌
    let mc_token = get_minecraft_token(client, &user_hash, &xsts_token).await?;
    
    // 获取 Minecraft Profile
    let profile = get_minecraft_profile(client, &mc_token).await?;

    // 构建账户信息
    let expires_at = Utc::now().timestamp() + ms_token.expires_in;
    
    let skin_url = profile.skins.iter()
        .find(|s| s.state == "ACTIVE")
        .map(|s| s.url.clone());
    
    let cape_url = profile.capes.iter()
        .find(|c| c.state == "ACTIVE")
        .map(|c| c.url.clone());

    Ok(Account {
        id: uuid::Uuid::new_v4(),
        account_type: AccountType::Microsoft,
        username: profile.name,
        uuid: profile.id,
        skin_url,
        cape_url,
        access_token: Some(mc_token),
        refresh_token: Some(ms_token.refresh_token),
        expires_at: Some(expires_at),
        auth_server_url: None,
    })
}

/// 刷新 Microsoft 令牌
pub async fn refresh_microsoft_token(
    client: &Client,
    refresh_token: &str,
) -> Result<MicrosoftTokenResponse, MicrosoftAuthError> {
    let response = client
        .post(MICROSOFT_TOKEN_URL)
        .form(&[
            ("client_id", AZURE_CLIENT_ID),
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("scope", "XboxLive.signin offline_access"),
        ])
        .send()
        .await?;

    let token = response.json::<MicrosoftTokenResponse>().await?;
    Ok(token)
}

/// 刷新账户令牌
pub async fn refresh_account(client: &Client, account: &Account) -> Result<Account, MicrosoftAuthError> {
    let refresh_token = account.refresh_token.as_ref()
        .ok_or(MicrosoftAuthError::UserCancelled)?;

    login_with_microsoft(client, "", Some(refresh_token)).await
}
