#[derive(Clone, Debug)]
#[allow(dead_code)]
pub(crate) struct User {
    id: String,
}

impl User {
    pub(crate) fn create(id: &str) -> Self {
        Self { id: id.to_string() }
    }
}

#[async_trait::async_trait]
pub(crate) trait UserRepository: Send + Sync {
    async fn find(&self, id: &str) -> anyhow::Result<Option<User>>;
    async fn store(&self, user: User) -> anyhow::Result<()>;
}

pub(crate) struct InMemoryUserRepository {
    users: std::sync::Arc<tokio::sync::RwLock<std::collections::HashMap<String, User>>>,
}

impl InMemoryUserRepository {
    pub fn new() -> Self {
        Self {
            users: std::sync::Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
        }
    }
}

#[async_trait::async_trait]
impl UserRepository for InMemoryUserRepository {
    async fn find(&self, id: &str) -> anyhow::Result<Option<User>> {
        let users = self.users.read().await;
        Ok(users.get(id).cloned())
    }

    async fn store(&self, user: User) -> anyhow::Result<()> {
        let mut users = self.users.write().await;
        users.entry(user.id.clone()).or_insert(user);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_create() {
        let user = User::create("user1");
        assert_eq!(user.id, "user1");
    }

    #[tokio::test]
    async fn test_find_returns_none_for_unknown() -> anyhow::Result<()> {
        let repo = InMemoryUserRepository::new();
        let result = repo.find("unknown").await?;
        assert!(result.is_none());
        Ok(())
    }

    #[tokio::test]
    async fn test_store_then_find_returns_user() -> anyhow::Result<()> {
        let repo = InMemoryUserRepository::new();
        repo.store(User::create("user1")).await?;
        let result = repo.find("user1").await?;
        assert!(result.is_some());
        assert_eq!(result.as_ref().map(|u| u.id.as_str()), Some("user1"));
        Ok(())
    }

    #[tokio::test]
    async fn test_store_is_idempotent() -> anyhow::Result<()> {
        let repo = InMemoryUserRepository::new();
        repo.store(User::create("user1")).await?;
        repo.store(User::create("user1")).await?;
        let result = repo.find("user1").await?;
        assert!(result.is_some());
        Ok(())
    }
}
