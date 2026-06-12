use crate::GoogleUserId;
use crate::User;
use crate::UserId;

#[async_trait::async_trait]
pub trait UserRepository: Send + Sync {
    async fn find(&self, id: &UserId) -> anyhow::Result<Option<User>>;
    async fn find_by_google_user_id(&self, id: &GoogleUserId) -> anyhow::Result<Option<User>>;
    async fn store(&self, user: User) -> anyhow::Result<()>;
}
