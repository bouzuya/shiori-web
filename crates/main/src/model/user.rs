pub(crate) use kernel::UserRepository;

fn google_user_id_to_document_id(id: &str) -> String {
    id.bytes().map(|b| format!("{b:02x}")).collect()
}

#[derive(serde::Deserialize, serde::Serialize)]
struct GoogleUserIdDocumentData {
    user_id: String,
}

#[derive(serde::Deserialize, serde::Serialize)]
struct UserDocumentData {
    created_at: String,
    google_user_id: String,
    user_id: String,
}

fn try_user_from_doc(doc: UserDocumentData) -> anyhow::Result<crate::model::User> {
    Ok(crate::model::User::new(
        crate::model::DateTime::from_rfc3339(&doc.created_at)?,
        doc.google_user_id.parse::<crate::model::GoogleUserId>()?,
        doc.user_id.parse::<crate::model::UserId>()?,
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
    async fn find(&self, id: &crate::model::UserId) -> anyhow::Result<Option<crate::model::User>> {
        let user_document_id = id.to_string();
        let user_doc_ref = self
            .firestore
            .doc(format!("users/{user_document_id}"))
            .map_err(|e| anyhow::anyhow!(e))?;
        let snapshot = user_doc_ref.get().await.map_err(|e| anyhow::anyhow!(e))?;
        if !snapshot.exists() {
            return Ok(None);
        }
        let doc: UserDocumentData = snapshot
            .data::<UserDocumentData>()
            .ok_or_else(|| anyhow::anyhow!("document data is missing"))?
            .map_err(|e| anyhow::anyhow!(e))?;
        Ok(Some(try_user_from_doc(doc)?))
    }

    async fn find_by_google_user_id(
        &self,
        id: &crate::model::GoogleUserId,
    ) -> anyhow::Result<Option<crate::model::User>> {
        let google_user_id_document_id = google_user_id_to_document_id(&id.to_string());
        let google_user_id_doc_ref = self
            .firestore
            .doc(format!("google_user_ids/{google_user_id_document_id}"))
            .map_err(|e| anyhow::anyhow!(e))?;
        let snapshot = google_user_id_doc_ref
            .get()
            .await
            .map_err(|e| anyhow::anyhow!(e))?;
        if !snapshot.exists() {
            return Ok(None);
        }
        let google_user_id_doc: GoogleUserIdDocumentData = snapshot
            .data::<GoogleUserIdDocumentData>()
            .ok_or_else(|| anyhow::anyhow!("document data is missing"))?
            .map_err(|e| anyhow::anyhow!(e))?;

        let user_document_id = google_user_id_doc.user_id;
        let user_doc_ref = self
            .firestore
            .doc(format!("users/{user_document_id}"))
            .map_err(|e| anyhow::anyhow!(e))?;
        let snapshot = user_doc_ref.get().await.map_err(|e| anyhow::anyhow!(e))?;
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
        let user_document_id = user.id().to_string();
        let google_user_id_document_id =
            google_user_id_to_document_id(&user.google_user_id().to_string());
        let user_doc_ref = self
            .firestore
            .doc(format!("users/{user_document_id}"))
            .map_err(|e| anyhow::anyhow!(e))?;
        let google_user_id_doc_ref = self
            .firestore
            .doc(format!("google_user_ids/{google_user_id_document_id}"))
            .map_err(|e| anyhow::anyhow!(e))?;
        let user_data = UserDocumentData {
            created_at: user.created_at().to_rfc3339(),
            google_user_id: user.google_user_id().to_string(),
            user_id: user.id().to_string(),
        };
        let google_user_id_data = GoogleUserIdDocumentData {
            user_id: user.id().to_string(),
        };
        self.firestore
            .run_transaction(
                move |tx| {
                    let user_doc_ref = user_doc_ref.clone();
                    let google_user_id_doc_ref = google_user_id_doc_ref.clone();
                    Box::pin(async move {
                        tx.set(&user_doc_ref, &user_data)?;
                        tx.set(&google_user_id_doc_ref, &google_user_id_data)?;
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
    fn test_google_user_id_to_document_id() -> anyhow::Result<()> {
        // a=0x61, b=0x62, c=0x63, 1=0x31, 2=0x32, 3=0x33
        assert_eq!(google_user_id_to_document_id("abc123"), "616263313233");
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
        let id = crate::model::UserId::new();
        let result = repo.find(&id).await?;
        assert!(result.is_none());
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    #[serial_test::serial]
    async fn test_firestore_store_then_find_returns_user() -> anyhow::Result<()> {
        let repo = firestore_repo()?;
        let google_user_id = "firestore_test_find_user1".parse::<crate::model::GoogleUserId>()?;
        let user = crate::model::User::create(google_user_id);
        let user_id = user.id().clone();
        repo.store(user).await?;
        let result = repo.find(&user_id).await?;
        assert!(result.is_some());
        assert_eq!(result.as_ref().map(|u| u.id().clone()), Some(user_id));
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    #[serial_test::serial]
    async fn test_firestore_find_by_google_user_id_returns_none_for_unknown() -> anyhow::Result<()>
    {
        let repo = firestore_repo()?;
        let result = repo
            .find_by_google_user_id(&"unknown_user_for_test".parse::<crate::model::GoogleUserId>()?)
            .await?;
        assert!(result.is_none());
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    #[serial_test::serial]
    async fn test_firestore_store_then_find_by_google_user_id_returns_user() -> anyhow::Result<()> {
        let repo = firestore_repo()?;
        let id = "firestore_test_user1".parse::<crate::model::GoogleUserId>()?;
        repo.store(crate::model::User::create(id.clone())).await?;
        let result = repo.find_by_google_user_id(&id).await?;
        assert!(result.is_some());
        assert_eq!(
            result.as_ref().map(|u| u.google_user_id().to_string()),
            Some(id.to_string())
        );
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    #[serial_test::serial]
    async fn test_firestore_store_is_idempotent() -> anyhow::Result<()> {
        let repo = firestore_repo()?;
        let id = "firestore_test_user2".parse::<crate::model::GoogleUserId>()?;
        repo.store(crate::model::User::create(id.clone())).await?;
        repo.store(crate::model::User::create(id.clone())).await?;
        let result = repo.find_by_google_user_id(&id).await?;
        assert!(result.is_some());
        Ok(())
    }
}
