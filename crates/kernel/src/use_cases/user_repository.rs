#[async_trait::async_trait]
pub trait UserRepository: Send + Sync {
    async fn find_by_google_user_id(
        &self,
        id: &crate::entities::GoogleUserId,
    ) -> anyhow::Result<Option<crate::entities::User>>;
    async fn store(&self, user: crate::entities::User) -> anyhow::Result<()>;
}
