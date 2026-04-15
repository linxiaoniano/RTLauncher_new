use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 账户类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AccountType {
    Offline,
    Microsoft,
    Yggdrasil,
}

/// 账户信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    /// 账户唯一ID
    pub id: Uuid,
    /// 账户类型
    pub account_type: AccountType,
    /// 游戏用户名
    pub username: String,
    /// Minecraft UUID
    pub uuid: Uuid,
    /// 皮肤URL（可选）
    pub skin_url: Option<String>,
    /// 披风URL（可选）
    pub cape_url: Option<String>,
    /// 访问令牌
    pub access_token: Option<String>,
    /// 刷新令牌
    pub refresh_token: Option<String>,
    /// 令牌过期时间（Unix时间戳，秒）
    pub expires_at: Option<i64>,
    /// 外置登录服务器URL（仅Yggdrasil）
    pub auth_server_url: Option<String>,
}

impl Account {
    /// 创建新的离线账户
    pub fn offline(username: String) -> Self {
        let uuid = Self::generate_offline_uuid(&username);
        Self {
            id: Uuid::new_v4(),
            account_type: AccountType::Offline,
            username,
            uuid,
            skin_url: None,
            cape_url: None,
            access_token: None,
            refresh_token: None,
            expires_at: None,
            auth_server_url: None,
        }
    }

    /// 根据用户名生成离线UUID（使用名称空间UUID v3）
    fn generate_offline_uuid(username: &str) -> Uuid {
        // 离线模式UUID使用 "OfflinePlayer:" + username 的MD5哈希
        // 但标准做法是使用UUID v3/v5
        Uuid::new_v5(&Uuid::NAMESPACE_OID, format!("OfflinePlayer:{}", username).as_bytes())
    }

    /// 检查令牌是否过期
    pub fn is_token_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            let now = chrono::Utc::now().timestamp();
            now >= expires_at - 60 // 提前60秒认为过期
        } else {
            false
        }
    }
}

/// 登录请求
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LoginRequest {
    Offline { username: String },
    Microsoft,
    Yggdrasil { 
        email: String, 
        password: String,
        server_url: String,
    },
}

/// 登录结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginResult {
    pub success: bool,
    pub account: Option<Account>,
    pub error: Option<String>,
    /// 微软登录需要用户在浏览器中访问的URL和设备码
    pub device_code: Option<DeviceCodeInfo>,
}

/// 设备码信息（用于微软登录）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceCodeInfo {
    /// 用户需要访问的URL
    pub verification_uri: String,
    /// 用户需要输入的设备码
    pub user_code: String,
    /// 设备码过期时间（秒）
    pub expires_in: u64,
    /// 轮询间隔（秒）
    pub interval: u64,
}

/// Yggdrasil 认证响应
#[derive(Debug, Clone, Deserialize)]
pub struct YggdrasilAuthResponse {
    pub access_token: String,
    pub client_token: String,
    pub available_profiles: Vec<GameProfile>,
    pub selected_profile: Option<GameProfile>,
}

/// 游戏配置文件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameProfile {
    pub id: Uuid,
    pub name: String,
    pub properties: Option<Vec<ProfileProperty>>,
}

/// 配置文件属性
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileProperty {
    pub name: String,
    pub value: String,
    pub signature: Option<String>,
}

/// Microsoft OAuth Token Response
#[derive(Debug, Clone, Deserialize)]
pub struct MicrosoftTokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: i64,
    pub token_type: String,
}

/// Xbox Live Auth Response
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct XboxLiveAuthResponse {
    pub issue_instant: String,
    pub not_after: String,
    pub token: String,
    pub display_claims: XboxDisplayClaims,
}

#[derive(Debug, Clone, Deserialize)]
pub struct XboxDisplayClaims {
    pub xui: Vec<XboxUserInfo>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct XboxUserInfo {
    pub uhs: String,
}

/// Minecraft Profile Response
#[derive(Debug, Clone, Deserialize)]
pub struct MinecraftProfileResponse {
    pub id: Uuid,
    pub name: String,
    pub skins: Vec<MinecraftSkin>,
    pub capes: Vec<MinecraftCape>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MinecraftSkin {
    pub id: Uuid,
    pub state: String,
    pub url: String,
    pub variant: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MinecraftCape {
    pub id: Uuid,
    pub state: String,
    pub url: String,
    pub alias: String,
}

/// Microsoft Device Code Response
#[derive(Debug, Clone, Deserialize)]
pub struct MicrosoftDeviceCodeResponse {
    pub user_code: String,
    pub device_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval: u64,
}
