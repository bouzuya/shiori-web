use crate::firestore::UserSettingsCollection;
use kernel::UserSettingsReader;

pub(crate) struct FirestoreUserSettingsReader {
    firestore: bouzuya_firestore_client::Firestore,
}

impl FirestoreUserSettingsReader {
    pub(crate) fn new(firestore: bouzuya_firestore_client::Firestore) -> Self {
        Self { firestore }
    }
}

#[async_trait::async_trait]
impl UserSettingsReader for FirestoreUserSettingsReader {
    async fn get(
        &self,
        user_id: kernel::UserId,
    ) -> anyhow::Result<Option<kernel::UserSettingsView>> {
        match crate::firestore::document::get::<UserSettingsCollection>(
            &self.firestore,
            &(),
            &user_id,
        )
        .await?
        {
            None => Ok(None),
            Some(data) => Ok(Some(data.into_user_settings_view(user_id)?)),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::firestore::FirestoreCollection as _;

    use super::*;

    fn firestore() -> anyhow::Result<bouzuya_firestore_client::Firestore> {
        Ok(bouzuya_firestore_client::Firestore::new(
            bouzuya_firestore_client::FirestoreOptions::default(),
        )?)
    }

    /// `store` メソッドが無いため、テストのセットアップ用にローカルの serde 形で
    /// `user_settings/{user_id}` ドキュメントを直接書き込む。
    async fn seed_settings(
        firestore: &bouzuya_firestore_client::Firestore,
        user_id: kernel::UserId,
        color_scheme: &'static str,
        utc_offset: &'static str,
    ) -> anyhow::Result<()> {
        #[derive(serde::Serialize)]
        struct Seed {
            color_scheme: &'static str,
            utc_offset: &'static str,
        }

        let doc_ref = firestore
            .doc(UserSettingsCollection::document_path(&(), &user_id))
            .map_err(|e| anyhow::anyhow!(e))?;
        firestore
            .run_transaction(
                move |tx| {
                    let doc_ref = doc_ref.clone();
                    Box::pin(async move {
                        tx.set(
                            &doc_ref,
                            &Seed {
                                color_scheme,
                                utc_offset,
                            },
                        )?;
                        Ok(())
                    })
                },
                bouzuya_firestore_client::TransactionOptions::default(),
            )
            .await
            .map_err(|e| anyhow::anyhow!(e))?;
        Ok(())
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_get_returns_none_for_unknown() -> anyhow::Result<()> {
        let reader = FirestoreUserSettingsReader::new(firestore()?);
        let user_id = kernel::UserId::new();
        assert!(reader.get(user_id).await?.is_none());
        Ok(())
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_get_returns_stored_settings() -> anyhow::Result<()> {
        let firestore = firestore()?;
        let user_id = kernel::UserId::new();
        seed_settings(&firestore, user_id, "dark", "+09:00").await?;
        let reader = FirestoreUserSettingsReader::new(firestore);
        let view = reader.get(user_id).await?;
        assert_eq!(view.as_ref().map(|v| v.color_scheme.as_str()), Some("dark"));
        assert_eq!(
            view.as_ref().map(|v| v.user_id.as_str()),
            Some(user_id.to_string().as_str())
        );
        assert_eq!(view.as_ref().map(|v| v.utc_offset.as_str()), Some("+09:00"));
        Ok(())
    }
}
