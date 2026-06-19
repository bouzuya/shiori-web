use crate::DocumentRef;
use crate::FirestoreCollectionExt as _;
use crate::UserSettingsCollection;
use crate::UserSettingsDocumentData;
use kernel::UserId;
use kernel::UserSettings;
use kernel::UserSettingsRepository;

pub(crate) struct FirestoreUserSettingsRepository {
    firestore: ::bouzuya_firestore_client::Firestore,
}

impl FirestoreUserSettingsRepository {
    pub(crate) fn new(firestore: ::bouzuya_firestore_client::Firestore) -> Self {
        Self { firestore }
    }
}

#[::async_trait::async_trait]
impl UserSettingsRepository for FirestoreUserSettingsRepository {
    async fn find(&self, user_id: &UserId) -> ::anyhow::Result<Option<UserSettings>> {
        match UserSettingsCollection::get(&self.firestore, &(), user_id).await? {
            None => Ok(None),
            Some(data) => Ok(Some(data.into_user_settings(*user_id)?)),
        }
    }

    async fn store(&self, settings: UserSettings) -> ::anyhow::Result<()> {
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
                ::bouzuya_firestore_client::TransactionOptions::default(),
            )
            .await
            .map_err(|e| ::anyhow::anyhow!(e))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kernel::ColorScheme;
    use kernel::UtcOffset;

    fn repo() -> ::anyhow::Result<FirestoreUserSettingsRepository> {
        let firestore = ::bouzuya_firestore_client::Firestore::new(
            ::bouzuya_firestore_client::FirestoreOptions::default(),
        )?;
        Ok(FirestoreUserSettingsRepository::new(firestore))
    }

    #[::tokio::test]
    #[::serial_test::serial]
    async fn test_find_returns_none_for_unknown() -> ::anyhow::Result<()> {
        let repo = repo()?;
        let user_id = UserId::new();
        assert!(repo.find(&user_id).await?.is_none());
        Ok(())
    }

    #[::tokio::test]
    #[::serial_test::serial]
    async fn test_store_then_find() -> ::anyhow::Result<()> {
        let repo = repo()?;
        let user_id = UserId::new();
        let settings = UserSettings::new(ColorScheme::Dark, None, user_id, UtcOffset::default());
        repo.store(settings).await?;
        let found = repo.find(&user_id).await?;
        assert_eq!(
            found
                .ok_or_else(|| ::anyhow::anyhow!("not found"))?
                .color_scheme(),
            ColorScheme::Dark
        );
        Ok(())
    }

    #[::tokio::test]
    #[::serial_test::serial]
    async fn test_store_overwrites_existing() -> ::anyhow::Result<()> {
        let repo = repo()?;
        let user_id = UserId::new();
        repo.store(UserSettings::new(
            ColorScheme::Dark,
            None,
            user_id,
            UtcOffset::default(),
        ))
        .await?;
        repo.store(UserSettings::new(
            ColorScheme::Light,
            None,
            user_id,
            UtcOffset::default(),
        ))
        .await?;
        let found = repo.find(&user_id).await?;
        assert_eq!(
            found
                .ok_or_else(|| ::anyhow::anyhow!("not found"))?
                .color_scheme(),
            ColorScheme::Light
        );
        Ok(())
    }
}
