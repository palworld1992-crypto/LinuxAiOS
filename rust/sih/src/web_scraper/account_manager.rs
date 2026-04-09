//! Account Manager - Quản lý tài khoản người dùng cho các platform

use crate::web_scraper::Platform;
use dashmap::DashMap;
use tracing::debug;

#[derive(Clone)]
pub struct Account {
    pub email: String,
    pub platform: Platform,
    pub priority: u32,
}

pub struct AccountManager {
    accounts: DashMap<Platform, Vec<Account>>,
}

impl AccountManager {
    pub fn new() -> Self {
        Self {
            accounts: DashMap::new(),
        }
    }

    pub fn add_account(&self, account: Account) {
        let platform = account.platform.clone();
        let mut entries = self
            .accounts
            .entry(platform.clone())
            .or_insert_with(Vec::new);
        entries.push(account);
        entries.sort_by(|a, b| b.priority.cmp(&a.priority));
        debug!("Account added for {:?}", platform);
    }

    pub fn get_account(&self, platform: &Platform) -> Option<Account> {
        self.accounts
            .get(platform)
            .and_then(|accounts| accounts.first().cloned())
    }

    pub fn rotate_account(&self, platform: &Platform) -> Option<Account> {
        if let Some(mut accounts) = self.accounts.get_mut(platform) {
            if accounts.len() > 1 {
                let account = accounts.remove(0);
                accounts.push(account.clone());
                return Some(account);
            }
        }
        self.get_account(platform)
    }

    pub fn remove_account(&self, platform: &Platform, email: &str) {
        if let Some(mut accounts) = self.accounts.get_mut(platform) {
            accounts.retain(|a| a.email != email);
        }
    }
}

impl Default for AccountManager {
    fn default() -> Self {
        Self::new()
    }
}
