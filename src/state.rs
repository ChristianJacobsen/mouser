use std::path::Path;
use std::sync::Arc;

use anyhow::{Context as _, Result};
use tokio::sync::RwLock;
use tracing::info;

use crate::config::Config;
use crate::models::PersistentState;

pub type SharedState = Arc<RwLock<AppState>>;

pub struct AppState {
    pub config: Config,
    pub persistent: PersistentState,
}

impl AppState {
    /// Load persistent state from disk, seeding the cookie from config if needed.
    pub fn load(config: Config) -> Result<Self> {
        let path = Path::new(&config.state_file);
        let mut persistent = if path.exists() {
            let data = std::fs::read_to_string(path)
                .with_context(|| format!("failed to read state file: {}", path.display()))?;
            serde_json::from_str(&data)
                .with_context(|| format!("failed to parse state file: {}", path.display()))?
        } else {
            PersistentState::default()
        };

        if persistent.cookie.is_none()
            && let Some(ref mam_id) = config.mam_id
        {
            info!("seeding cookie from MOUSER_MAM_ID");
            persistent.cookie = Some(mam_id.clone());
        }

        Ok(Self { config, persistent })
    }

    /// Atomically save persistent state to disk.
    pub fn save(&self) -> Result<()> {
        let path = Path::new(&self.config.state_file);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).with_context(|| {
                format!("failed to create state directory: {}", parent.display())
            })?;
        }

        let tmp = path.with_extension("json.tmp");
        let data =
            serde_json::to_string_pretty(&self.persistent).context("failed to serialize state")?;
        std::fs::write(&tmp, &data)
            .with_context(|| format!("failed to write temp state file: {}", tmp.display()))?;
        std::fs::rename(&tmp, path)
            .with_context(|| format!("failed to rename state file: {}", path.display()))?;

        Ok(())
    }
}
