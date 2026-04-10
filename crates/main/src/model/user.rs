pub(crate) use kernel::UserRepository;

fn user_id_to_document_id(id: &str) -> String {
    id.bytes().map(|b| format!("{b:02x}")).collect()
}

#[derive(serde::Deserialize, serde::Serialize)]
struct UserDocumentData {
    created_at: String,
    id: String,
}

fn try_user_from_doc(doc: UserDocumentData) -> anyhow::Result<crate::model::User> {
    Ok(crate::model::User::new(
        crate::model::DateTime::from_rfc3339(&doc.created_at)?,
        doc.id.parse::<crate::model::UserId>()?,
    ))
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
    async fn find(&self, id: &str) -> anyhow::Result<Option<crate::model::User>> {
        let document_id = user_id_to_document_id(id);
        let doc_ref = self
            .firestore
            .doc(format!("users/{document_id}"))
            .map_err(|e| anyhow::anyhow!(e))?;
        let snapshot = doc_ref.get().await.map_err(|e| anyhow::anyhow!(e))?;
        if !snapshot.exists() {
            return Ok(None);
        }
        let doc: UserDocumentData = snapshot
            .data::<UserDocumentData>()
            .ok_or_else(|| anyhow::anyhow!("document data is missing"))?
            .map_err(|e| anyhow::anyhow!(e))?;
        Ok(Some(try_user_from_doc(doc)?))
    }

    async fn store(&self, user: crate::model::User) -> anyhow::Result<()> {
        let document_id = user_id_to_document_id(&user.id().to_string());
        let doc_ref = self
            .firestore
            .doc(format!("users/{document_id}"))
            .map_err(|e| anyhow::anyhow!(e))?;
        let data = UserDocumentData {
            created_at: user.created_at().to_rfc3339(),
            id: user.id().to_string(),
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
    fn test_user_id_to_document_id() -> anyhow::Result<()> {
        // a=0x61, b=0x62, c=0x63, 1=0x31, 2=0x32, 3=0x33
        assert_eq!(user_id_to_document_id("abc123"), "616263313233");
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
        repo.store(crate::model::User::create(
            id.parse::<crate::model::UserId>()?,
        ))
        .await?;
        let result = repo.find(id).await?;
        assert!(result.is_some());
        assert_eq!(
            result.as_ref().map(|u| u.id().to_string()),
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
        repo.store(crate::model::User::create(
            id.parse::<crate::model::UserId>()?,
        ))
        .await?;
        repo.store(crate::model::User::create(
            id.parse::<crate::model::UserId>()?,
        ))
        .await?;
        let result = repo.find(id).await?;
        assert!(result.is_some());
        Ok(())
    }
}
