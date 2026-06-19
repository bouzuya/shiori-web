use crate::DocumentRef;
use crate::FirestoreCollectionExt as _;
use crate::UserSettingsCollection;
use crate::UserSettingsDocumentData;
use kernel::UserSettingsRepository;

pub(crate) struct FirestoreUserSettingsRepository {
    firestore: bouzuya_firestore_client::Firestore,
}

impl FirestoreUserSettingsRepository {
    pub(crate) fn new(firestore: bouzuya_firestore_client::Firestore) -> Self {
        Self { firestore }
    }
}

#[async_trait::async_trait]
impl UserSettingsRepository for FirestoreUserSettingsRepository {
    async fn find(&self, user_id: &kernel::UserId) -> anyhow::Result<Option<kernel::UserSettings>> {
        match UserSettingsCollection::get(&self.firestore, &(), user_id).await? {
            None => Ok(None),
            Some(data) => Ok(Some(data.into_user_settings(*user_id)?)),
        }
    }

    async fn store(&self, settings: kernel::UserSettings) -> anyhow::Result<()> {
        let user_id = settings.user_id();
        let doc_ref = DocumentRef::<UserSettingsCollection>::new(&self.firestore, &(), &user_id)?;
        let data = UserSettingsDocumentData::from_user_settings(&settings);
        self.firestore
            .run_transaction(
                move |tx| {
                    let doc_ref = doc_ref.clone();
                    Box::pin(async move {
                        doc_ref.set(tx, &data)?;
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

    fn repo() -> anyhow::Result<FirestoreUserSettingsRepository> {
        let firestore = bouzuya_firestore_client::Firestore::new(
            bouzuya_firestore_client::FirestoreOptions::default(),
        )?;
        Ok(FirestoreUserSettingsRepository::new(firestore))
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_find_returns_none_for_unknown() -> anyhow::Result<()> {
        let repo = repo()?;
        let user_id = kernel::UserId::new();
        assert!(repo.find(&user_id).await?.is_none());
        Ok(())
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_store_then_find() -> anyhow::Result<()> {
        let repo = repo()?;
        let user_id = kernel::UserId::new();
        let settings = kernel::UserSettings::new(
            kernel::ColorScheme::Dark,
            None,
            user_id,
            kernel::UtcOffset::default(),
        );
        repo.store(settings).await?;
        let found = repo.find(&user_id).await?;
        assert_eq!(
            found
                .ok_or_else(|| anyhow::anyhow!("not found"))?
                .color_scheme(),
            kernel::ColorScheme::Dark
        );
        Ok(())
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_store_overwrites_existing() -> anyhow::Result<()> {
        let repo = repo()?;
        let user_id = kernel::UserId::new();
        repo.store(kernel::UserSettings::new(
            kernel::ColorScheme::Dark,
            None,
            user_id,
            kernel::UtcOffset::default(),
        ))
        .await?;
        repo.store(kernel::UserSettings::new(
            kernel::ColorScheme::Light,
            None,
            user_id,
            kernel::UtcOffset::default(),
        ))
        .await?;
        let found = repo.find(&user_id).await?;
        assert_eq!(
            found
                .ok_or_else(|| anyhow::anyhow!("not found"))?
                .color_scheme(),
            kernel::ColorScheme::Light
        );
        Ok(())
    }
}
