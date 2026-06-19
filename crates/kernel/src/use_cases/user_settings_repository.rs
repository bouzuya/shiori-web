use crate::UserId;
use crate::UserSettings;

#[::async_trait::async_trait]
pub trait UserSettingsRepository: Send + Sync {
    async fn find(&self, user_id: &UserId) -> ::anyhow::Result<Option<UserSettings>>;
    async fn store(&self, settings: UserSettings) -> ::anyhow::Result<()>;
}
