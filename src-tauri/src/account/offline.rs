use super::{Account, AccountType};
use uuid::Uuid;

/// 创建离线账户
pub fn create_offline_account(username: &str) -> Account {
    Account {
        id: Uuid::new_v4(),
        account_type: AccountType::Offline,
        username: username.to_string(),
        uuid: generate_offline_uuid(username),
        skin_url: None,
        cape_url: None,
        access_token: None,
        refresh_token: None,
        expires_at: None,
        auth_server_url: None,
    }
}

/// 根据用户名生成离线UUID
/// 使用UUID v5算法，命名空间为 "OfflinePlayer"
fn generate_offline_uuid(username: &str) -> Uuid {
    Uuid::new_v5(&Uuid::NAMESPACE_OID, format!("OfflinePlayer:{}", username).as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_offline_account() {
        let account = create_offline_account("Steve");
        assert_eq!(account.username, "Steve");
        assert_eq!(account.account_type, AccountType::Offline);
        assert!(account.access_token.is_none());
    }

    #[test]
    fn test_consistent_uuid() {
        let uuid1 = generate_offline_uuid("Steve");
        let uuid2 = generate_offline_uuid("Steve");
        assert_eq!(uuid1, uuid2);
        
        let uuid3 = generate_offline_uuid("Alex");
        assert_ne!(uuid1, uuid3);
    }
}
