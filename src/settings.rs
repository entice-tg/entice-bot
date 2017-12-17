use std::sync::RwLock;

use config::{Config, File};

use errors::*;

lazy_static! {
    static ref SETTINGS: RwLock<Config> = RwLock::new(Config::default());
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct TelegramClient {
    pub api_id: String,
    pub api_hash: String,
    pub server: String,
    pub phone: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct TelegramBot {
    pub auth_token: String,

    #[serde(default = "TelegramBot::default_update_interval")]
    pub update_interval: u64,
}

impl TelegramBot {
    fn default_update_interval() -> u64 {
        200
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct Database {
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct Settings {
    pub telegram_bot: TelegramBot,
    pub telegram_client: TelegramClient,
    pub database: Database,
}

impl Settings {
    pub fn try_fetch() -> Result<Self> {
        Ok(SETTINGS.read().unwrap().clone().try_into::<Settings>()?)
    }

    pub fn add_file(name: &str) -> Result<()> {
        SETTINGS.write().unwrap().merge(File::with_name(name))?;
        Ok(())
    }
}
