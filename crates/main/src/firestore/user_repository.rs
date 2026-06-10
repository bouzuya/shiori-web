use crate::firestore::FirestoreCollection;
use crate::firestore::GoogleUserIdDocumentData;
use crate::firestore::GoogleUserIds;
use crate::firestore::UserDocumentData;
use crate::firestore::Users;
use kernel::UserRepository;

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
        match crate::firestore::document::get::<Users>(&self.firestore, &(), id).await? {
            None => Ok(None),
            Some(data) => Ok(Some(data.into_user()?)),
        }
    }

    async fn find_by_google_user_id(
        &self,
        id: &crate::model::GoogleUserId,
    ) -> anyhow::Result<Option<crate::model::User>> {
        let user_id =
            match crate::firestore::document::get::<GoogleUserIds>(&self.firestore, &(), id).await?
            {
                None => return Ok(None),
                Some(data) => data.into_user_id()?,
            };
        match crate::firestore::document::get::<Users>(&self.firestore, &(), &user_id).await? {
            None => Ok(None),
            Some(user_data) => Ok(Some(user_data.into_user()?)),
        }
    }

    async fn store(&self, user: crate::model::User) -> anyhow::Result<()> {
        let user_doc_ref = self
            .firestore
            .doc(Users::document_path(&(), &user.id()))
            .map_err(|e| anyhow::anyhow!(e))?;
        let google_user_id_doc_ref = self
            .firestore
            .doc(GoogleUserIds::document_path(&(), user.google_user_id()))
            .map_err(|e| anyhow::anyhow!(e))?;
        let user_data = UserDocumentData::from_user(&user);
        let google_user_id_data = GoogleUserIdDocumentData::from_user(&user);
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

    fn firestore_repo() -> anyhow::Result<FirestoreUserRepository> {
        let firestore = bouzuya_firestore_client::Firestore::new(
            bouzuya_firestore_client::FirestoreOptions::default(),
        )?;
        Ok(FirestoreUserRepository::new(firestore))
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_firestore_find_returns_none_for_unknown() -> anyhow::Result<()> {
        let repo = firestore_repo()?;
        let id = crate::model::UserId::new();
        let result = repo.find(&id).await?;
        assert!(result.is_none());
        Ok(())
    }

    #[tokio::test]
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
