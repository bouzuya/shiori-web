#[async_trait::async_trait]
pub trait UserRepository: Send + Sync {
    async fn find(&self, id: &str) -> anyhow::Result<Option<entities::User>>;
    async fn store(&self, user: entities::User) -> anyhow::Result<()>;
}
