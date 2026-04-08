#[derive(Clone, Debug)]
#[allow(dead_code)]
pub(crate) struct User {
    created_at: crate::model::DateTime,
    id: crate::model::UserId,
}

impl User {
    pub(crate) fn create(id: crate::model::UserId) -> Self {
        Self {
            created_at: crate::model::DateTime::now(),
            id,
        }
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
        users.entry(user.id.to_string()).or_insert(user);
        Ok(())
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
struct UserDocumentData {
    created_at: String,
    id: String,
}

impl TryFrom<UserDocumentData> for User {
    type Error = anyhow::Error;

    fn try_from(doc: UserDocumentData) -> Result<Self, Self::Error> {
        Ok(Self {
            created_at: crate::model::DateTime::from_rfc3339(&doc.created_at)?,
            id: doc.id.parse::<crate::model::UserId>()?,
        })
    }
}

pub(crate) struct FirestoreUserRepository {
    firestore: bouzuya_firestore_client::Firestore,
}

impl FirestoreUserRepository {
    pub(crate) fn new(firestore: bouzuya_firestore_client::Firestore) -> Self {
        Self { firestore }
    }
}

#[async_trait::async_trait]
impl UserRepository for FirestoreUserRepository {
    async fn find(&self, id: &str) -> anyhow::Result<Option<User>> {
        let doc_ref = self
            .firestore
            // FIXME: unsafe
            .doc(format!("users/{id}"))
            .map_err(|e| anyhow::anyhow!(e))?;
        let snapshot = doc_ref.get().await.map_err(|e| anyhow::anyhow!(e))?;
        if !snapshot.exists() {
            return Ok(None);
        }
        let doc: UserDocumentData = snapshot
            .data::<UserDocumentData>()
            .ok_or_else(|| anyhow::anyhow!("document data is missing"))?
            .map_err(|e| anyhow::anyhow!(e))?;
        Ok(Some(User::try_from(doc)?))
    }

    async fn store(&self, user: User) -> anyhow::Result<()> {
        let doc_ref = self
            .firestore
            .doc(format!("users/{}", user.id))
            .map_err(|e| anyhow::anyhow!(e))?;
        let data = UserDocumentData {
            created_at: user.created_at.to_rfc3339(),
            id: user.id.to_string(),
        };
        self.firestore
            .run_transaction(
                move |tx| {
                    let doc_ref = doc_ref.clone();
                    Box::pin(async move {
                        tx.set(&doc_ref, &data)?;
                        Ok(())
                    })
                },
                bouzuya_firestore_client::TransactionOptions::default(),
            )
            .await
            .map_err(|e| anyhow::anyhow!(e))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_create() -> anyhow::Result<()> {
        let user = User::create("user1".parse::<crate::model::UserId>()?);
        assert_eq!(user.id.to_string(), "user1");
        Ok(())
    }

    #[test]
    fn test_user_create_has_created_at() -> anyhow::Result<()> {
        let before = crate::model::DateTime::now();
        let user = User::create("user1".parse::<crate::model::UserId>()?);
        let after = crate::model::DateTime::now();
        assert!(user.created_at >= before);
        assert!(user.created_at <= after);
        Ok(())
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
        repo.store(User::create("user1".parse()?)).await?;
        let result = repo.find("user1").await?;
        assert!(result.is_some());
        assert_eq!(
            result.as_ref().map(|u| u.id.to_string()),
            Some("user1".to_string())
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_store_is_idempotent() -> anyhow::Result<()> {
        let repo = InMemoryUserRepository::new();
        repo.store(User::create("user1".parse()?)).await?;
        repo.store(User::create("user1".parse()?)).await?;
        let result = repo.find("user1").await?;
        assert!(result.is_some());
        Ok(())
    }

    fn firestore_repo() -> anyhow::Result<FirestoreUserRepository> {
        let firestore = bouzuya_firestore_client::Firestore::new(
            bouzuya_firestore_client::FirestoreOptions::default(),
        )?;
        Ok(FirestoreUserRepository::new(firestore))
    }

    #[tokio::test]
    #[ignore]
    #[serial_test::serial]
    async fn test_firestore_find_returns_none_for_unknown() -> anyhow::Result<()> {
        let repo = firestore_repo()?;
        let result = repo.find("unknown_user_for_test").await?;
        assert!(result.is_none());
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    #[serial_test::serial]
    async fn test_firestore_store_then_find_returns_user() -> anyhow::Result<()> {
        let repo = firestore_repo()?;
        let id = "firestore_test_user1";
        repo.store(User::create(id.parse::<crate::model::UserId>()?))
            .await?;
        let result = repo.find(id).await?;
        assert!(result.is_some());
        assert_eq!(
            result.as_ref().map(|u| u.id.to_string()),
            Some(id.to_string())
        );
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    #[serial_test::serial]
    async fn test_firestore_store_is_idempotent() -> anyhow::Result<()> {
        let repo = firestore_repo()?;
        let id = "firestore_test_user2";
        repo.store(User::create(id.parse::<crate::model::UserId>()?))
            .await?;
        repo.store(User::create(id.parse::<crate::model::UserId>()?))
            .await?;
        let result = repo.find(id).await?;
        assert!(result.is_some());
        Ok(())
    }
}
