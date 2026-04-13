pub(crate) use kernel::BookmarkRepository;

#[derive(serde::Deserialize, serde::Serialize)]
struct BookmarkDocumentData {
    bookmark_id: String,
    comment: String,
    created_at: String,
    title: String,
    updated_at: String,
    url: String,
}

fn try_bookmark_from_data(
    data: BookmarkDocumentData,
    user_id: kernel::UserId,
) -> anyhow::Result<kernel::Bookmark> {
    Ok(kernel::Bookmark::new(
        data.comment.parse::<kernel::Comment>()?,
        kernel::DateTime::from_rfc3339(&data.created_at)?,
        data.bookmark_id.parse::<kernel::BookmarkId>()?,
        data.title.parse::<kernel::Title>()?,
        kernel::DateTime::from_rfc3339(&data.updated_at)?,
        data.url.parse::<kernel::Url>()?,
        user_id,
    ))
}

fn bookmark_to_data(bookmark: &kernel::Bookmark) -> BookmarkDocumentData {
    BookmarkDocumentData {
        bookmark_id: bookmark.id().to_string(),
        comment: bookmark.comment().to_string(),
        created_at: bookmark.created_at().to_rfc3339(),
        title: bookmark.title().to_string(),
        updated_at: bookmark.updated_at().to_rfc3339(),
        url: bookmark.url().to_string(),
    }
}

pub(crate) struct FirestoreBookmarkRepository {
    firestore: bouzuya_firestore_client::Firestore,
}

impl FirestoreBookmarkRepository {
    pub(crate) fn new(firestore: bouzuya_firestore_client::Firestore) -> Self {
        Self { firestore }
    }
}

#[async_trait::async_trait]
impl BookmarkRepository for FirestoreBookmarkRepository {
    async fn find(
        &self,
        user_id: kernel::UserId,
        bookmark_id: kernel::BookmarkId,
    ) -> anyhow::Result<Option<kernel::Bookmark>> {
        let doc_ref = self
            .firestore
            .doc(format!("users/{user_id}/bookmarks/{bookmark_id}"))
            .map_err(|e| anyhow::anyhow!(e))?;
        let snapshot = doc_ref.get().await.map_err(|e| anyhow::anyhow!(e))?;
        if !snapshot.exists() {
            return Ok(None);
        }
        let data = snapshot
            .data::<BookmarkDocumentData>()
            .ok_or_else(|| anyhow::anyhow!("document data is missing"))?
            .map_err(|e| anyhow::anyhow!(e))?;
        Ok(Some(try_bookmark_from_data(data, user_id)?))
    }

    async fn store(
        &self,
        updated_at: Option<kernel::DateTime>,
        bookmark: kernel::Bookmark,
    ) -> anyhow::Result<()> {
        let user_id = bookmark.user_id();
        let bookmark_id = bookmark.id();
        let doc_ref = self
            .firestore
            .doc(format!("users/{user_id}/bookmarks/{bookmark_id}"))
            .map_err(|e| anyhow::anyhow!(e))?;
        let data = bookmark_to_data(&bookmark);
        self.firestore
            .run_transaction(
                move |tx| {
                    let doc_ref = doc_ref.clone();
                    Box::pin(async move {
                        let to_fs_err = |e: anyhow::Error| {
                            bouzuya_firestore_client::Error::custom(e.to_string())
                        };
                        match updated_at {
                            None => {
                                tx.create(&doc_ref, &data)?;
                            }
                            Some(t) => {
                                let snapshot = tx.get(&doc_ref).await?;
                                let stored = snapshot
                                    .data::<BookmarkDocumentData>()
                                    .ok_or_else(|| {
                                        bouzuya_firestore_client::Error::custom(
                                            "document not found",
                                        )
                                    })?
                                    .map_err(|e| {
                                        bouzuya_firestore_client::Error::custom(e.to_string())
                                    })?;
                                let existing_updated_at =
                                    kernel::DateTime::from_rfc3339(&stored.updated_at)
                                        .map_err(&to_fs_err)?;
                                if existing_updated_at != t {
                                    return Err(bouzuya_firestore_client::Error::custom(
                                        "optimistic lock conflict",
                                    ));
                                }
                                tx.set(&doc_ref, &data)?;
                            }
                        }
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

    fn firestore_repo() -> anyhow::Result<FirestoreBookmarkRepository> {
        let firestore = bouzuya_firestore_client::Firestore::new(
            bouzuya_firestore_client::FirestoreOptions::default(),
        )?;
        Ok(FirestoreBookmarkRepository::new(firestore))
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_firestore_find_returns_none_for_unknown() -> anyhow::Result<()> {
        let repo = firestore_repo()?;
        let user_id = kernel::UserId::new();
        let bookmark_id = kernel::BookmarkId::new();
        let result = repo.find(user_id, bookmark_id).await?;
        assert!(result.is_none());
        Ok(())
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_firestore_store_new_then_find() -> anyhow::Result<()> {
        let repo = firestore_repo()?;
        let user_id = kernel::UserId::new();
        let bookmark = kernel::Bookmark::create(
            user_id,
            "https://example.com".parse::<kernel::Url>()?,
            "test title".parse::<kernel::Title>()?,
            "test comment".parse::<kernel::Comment>()?,
        );
        let bookmark_id = bookmark.id();
        repo.store(None, bookmark).await?;
        let found = repo.find(user_id, bookmark_id).await?;
        assert!(found.is_some());
        assert_eq!(
            found.ok_or_else(|| anyhow::anyhow!("not found"))?.id(),
            bookmark_id
        );
        Ok(())
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_firestore_store_new_fails_if_already_exists() -> anyhow::Result<()> {
        let repo = firestore_repo()?;
        let user_id = kernel::UserId::new();
        let bookmark = kernel::Bookmark::create(
            user_id,
            "https://example.com".parse::<kernel::Url>()?,
            "test title".parse::<kernel::Title>()?,
            "test comment".parse::<kernel::Comment>()?,
        );
        repo.store(None, bookmark.clone()).await?;
        assert!(repo.store(None, bookmark).await.is_err());
        Ok(())
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_firestore_store_update_succeeds_when_updated_at_matches() -> anyhow::Result<()> {
        let repo = firestore_repo()?;
        let user_id = kernel::UserId::new();
        let bookmark = kernel::Bookmark::create(
            user_id,
            "https://example.com".parse::<kernel::Url>()?,
            "test title".parse::<kernel::Title>()?,
            "test comment".parse::<kernel::Comment>()?,
        );
        let original_updated_at = bookmark.updated_at();
        let id = bookmark.id();
        repo.store(None, bookmark).await?;

        let updated = kernel::Bookmark::new(
            "updated comment".parse::<kernel::Comment>()?,
            kernel::DateTime::now(),
            id,
            "updated title".parse::<kernel::Title>()?,
            kernel::DateTime::now(),
            "https://updated.example.com".parse::<kernel::Url>()?,
            user_id,
        );
        repo.store(Some(original_updated_at), updated).await?;
        Ok(())
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_firestore_store_update_fails_when_updated_at_mismatches() -> anyhow::Result<()> {
        let repo = firestore_repo()?;
        let user_id = kernel::UserId::new();
        let bookmark = kernel::Bookmark::create(
            user_id,
            "https://example.com".parse::<kernel::Url>()?,
            "test title".parse::<kernel::Title>()?,
            "test comment".parse::<kernel::Comment>()?,
        );
        let id = bookmark.id();
        repo.store(None, bookmark).await?;

        let stale_updated_at = kernel::DateTime::from_rfc3339("2000-01-01T00:00:00.000Z")?;
        let updated = kernel::Bookmark::new(
            "updated comment".parse::<kernel::Comment>()?,
            kernel::DateTime::now(),
            id,
            "updated title".parse::<kernel::Title>()?,
            kernel::DateTime::now(),
            "https://updated.example.com".parse::<kernel::Url>()?,
            user_id,
        );
        assert!(repo.store(Some(stale_updated_at), updated).await.is_err());
        Ok(())
    }
}
