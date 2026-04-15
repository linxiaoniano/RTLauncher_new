use super::types::*;
use std::path::PathBuf;
use std::fs;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 账户存储
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AccountStore {
    pub accounts: Vec<Account>,
    pub selected_account_id: Option<Uuid>,
}

impl AccountStore {
    /// 获取存储文件路径
    fn get_storage_path() -> Result<PathBuf, String> {
        let data_dir = dirs::data_dir()
            .ok_or_else(|| "无法获取用户数据目录".to_string())?
            .join("RTLauncher")
            .join("accounts.json");
        Ok(data_dir)
    }

    /// 加载账户存储
    pub fn load() -> Result<Self, String> {
        let path = Self::get_storage_path()?;
        
        if !path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(&path)
            .map_err(|e| format!("读取账户文件失败: {}", e))?;

        serde_json::from_str(&content)
            .map_err(|e| format!("解析账户文件失败: {}", e))
    }

    /// 保存账户存储
    pub fn save(&self) -> Result<(), String> {
        let path = Self::get_storage_path()?;
        
        // 确保目录存在
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("创建目录失败: {}", e))?;
        }

        let content = serde_json::to_string_pretty(self)
            .map_err(|e| format!("序列化账户失败: {}", e))?;

        fs::write(&path, content)
            .map_err(|e| format!("写入账户文件失败: {}", e))
    }

    /// 添加账户
    pub fn add_account(&mut self, account: Account) {
        // 检查是否已存在相同类型的账户
        // 对于微软和Yggdrasil，检查UUID
        if account.account_type != AccountType::Offline {
            self.accounts.retain(|a| a.uuid != account.uuid);
        } else {
            // 对于离线账户，检查用户名
            self.accounts.retain(|a| {
                !(a.account_type == AccountType::Offline && a.username == account.username)
            });
        }
        
        self.accounts.push(account);
    }

    /// 删除账户
    pub fn remove_account(&mut self, account_id: Uuid) -> bool {
        let initial_len = self.accounts.len();
        self.accounts.retain(|a| a.id != account_id);
        
        // 如果删除的是当前选中的账户，清除选择
        if self.selected_account_id == Some(account_id) {
            self.selected_account_id = None;
        }
        
        self.accounts.len() < initial_len
    }

    /// 获取选中的账户
    pub fn get_selected_account(&self) -> Option<&Account> {
        self.selected_account_id
            .and_then(|id| self.accounts.iter().find(|a| a.id == id))
    }

    /// 设置选中的账户
    pub fn set_selected_account(&mut self, account_id: Uuid) -> bool {
        if self.accounts.iter().any(|a| a.id == account_id) {
            self.selected_account_id = Some(account_id);
            true
        } else {
            false
        }
    }

    /// 更新账户
    pub fn update_account(&mut self, account: Account) -> bool {
        if let Some(existing) = self.accounts.iter_mut().find(|a| a.id == account.id) {
            *existing = account;
            true
        } else {
            false
        }
    }

    /// 根据ID获取账户
    pub fn get_account(&self, id: Uuid) -> Option<&Account> {
        self.accounts.iter().find(|a| a.id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_offline_account() {
        let mut store = AccountStore::default();
        let account = Account::offline("Steve".to_string());
        
        store.add_account(account.clone());
        
        assert_eq!(store.accounts.len(), 1);
        assert_eq!(store.accounts[0].username, "Steve");
    }

    #[test]
    fn test_remove_account() {
        let mut store = AccountStore::default();
        let account = Account::offline("Steve".to_string());
        let id = account.id;
        
        store.add_account(account);
        assert!(store.remove_account(id));
        assert_eq!(store.accounts.len(), 0);
    }

    #[test]
    fn test_selected_account() {
        let mut store = AccountStore::default();
        let account = Account::offline("Steve".to_string());
        let id = account.id;
        
        store.add_account(account);
        store.set_selected_account(id);
        
        assert!(store.get_selected_account().is_some());
        assert_eq!(store.get_selected_account().unwrap().username, "Steve");
    }
}
