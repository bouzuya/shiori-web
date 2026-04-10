#[async_trait::async_trait]
pub trait UserRepository: Send + Sync {
    async fn find(&self, id: &str) -> anyhow::Result<Option<crate::entities::User>>;
    async fn store(&self, user: crate::entities::User) -> anyhow::Result<()>;
}
