use crate::BookmarkDocumentData;
use crate::BookmarksCollection;
use crate::DocumentRef;
use crate::FirestoreCollectionExt as _;
use kernel::Bookmark;
use kernel::BookmarkId;
use kernel::BookmarkRepository;
use kernel::DateTime;
use kernel::UserId;

pub(crate) struct FirestoreBookmarkRepository {
    firestore: ::bouzuya_firestore_client::Firestore,
}

impl FirestoreBookmarkRepository {
    pub(crate) fn new(firestore: ::bouzuya_firestore_client::Firestore) -> Self {
        Self { firestore }
    }
}

#[::async_trait::async_trait]
impl BookmarkRepository for FirestoreBookmarkRepository {
    async fn find(
        &self,
        user_id: UserId,
        bookmark_id: BookmarkId,
    ) -> ::anyhow::Result<Option<Bookmark>> {
        match BookmarksCollection::get(&self.firestore, &user_id, &bookmark_id).await? {
            None => Ok(None),
            Some(data) => Ok(Some(data.into_bookmark(user_id)?)),
        }
    }

    async fn store(
        &self,
        updated_at: Option<DateTime>,
        bookmark: Bookmark,
    ) -> ::anyhow::Result<()> {
        let user_id = bookmark.user_id();
        let bookmark_id = bookmark.id();
        let deleted_at = bookmark.deleted_at();
        let doc_ref =
            DocumentRef::<BookmarksCollection>::new(&self.firestore, &user_id, &bookmark_id)?;
        let data = BookmarkDocumentData::from_bookmark(&bookmark);
        self.firestore
            .run_transaction(
                move |tx| {
                    let doc_ref = doc_ref.clone();
                    Box::pin(async move {
                        let to_fs_err = |e: ::anyhow::Error| {
                            ::bouzuya_firestore_client::Error::custom(e.to_string())
                        };
                        match updated_at {
                            None => {
                                doc_ref.create(tx, &data)?;
                            }
                            Some(t) => {
                                let stored =
                                    doc_ref.get_in_transaction(tx).await?.ok_or_else(|| {
                                        ::bouzuya_firestore_client::Error::custom(
                                            "document not found",
                                        )
                                    })?;
                                let existing_updated_at =
                                    DateTime::from_rfc3339(stored.updated_at())
                                        .map_err(&to_fs_err)?;
                                if existing_updated_at != t {
                                    return Err(::bouzuya_firestore_client::Error::custom(
                                        "optimistic lock conflict",
                                    ));
                                }
                                if deleted_at.is_some() {
                                    doc_ref.delete(
                                        tx,
                                        ::bouzuya_firestore_client::Precondition::default(),
                                    )?;
                                } else {
                                    doc_ref.set(tx, &data)?;
                                }
                            }
                        }
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
    use kernel::Comment;
    use kernel::Title;
    use kernel::Url;

    fn firestore_repo() -> ::anyhow::Result<FirestoreBookmarkRepository> {
        let firestore = ::bouzuya_firestore_client::Firestore::new(
            ::bouzuya_firestore_client::FirestoreOptions::default(),
        )?;
        Ok(FirestoreBookmarkRepository::new(firestore))
    }

    #[::tokio::test]
    #[::serial_test::serial]
    async fn test_firestore_find_returns_none_for_unknown() -> ::anyhow::Result<()> {
        let repo = firestore_repo()?;
        let user_id = UserId::new();
        let bookmark_id = BookmarkId::new();
        let result = repo.find(user_id, bookmark_id).await?;
        assert!(result.is_none());
        Ok(())
    }

    #[::tokio::test]
    #[::serial_test::serial]
    async fn test_firestore_store_new_then_find() -> ::anyhow::Result<()> {
        let repo = firestore_repo()?;
        let user_id = UserId::new();
        let bookmark = Bookmark::create(
            user_id,
            "https://example.com".parse::<Url>()?,
            "test title".parse::<Title>()?,
            "test comment".parse::<Comment>()?,
        );
        let bookmark_id = bookmark.id();
        repo.store(None, bookmark).await?;
        let found = repo.find(user_id, bookmark_id).await?;
        assert!(found.is_some());
        assert_eq!(
            found.ok_or_else(|| ::anyhow::anyhow!("not found"))?.id(),
            bookmark_id
        );
        Ok(())
    }

    #[::tokio::test]
    #[::serial_test::serial]
    async fn test_firestore_store_new_fails_if_already_exists() -> ::anyhow::Result<()> {
        let repo = firestore_repo()?;
        let user_id = UserId::new();
        let bookmark = Bookmark::create(
            user_id,
            "https://example.com".parse::<Url>()?,
            "test title".parse::<Title>()?,
            "test comment".parse::<Comment>()?,
        );
        repo.store(None, bookmark.clone()).await?;
        assert!(repo.store(None, bookmark).await.is_err());
        Ok(())
    }

    #[::tokio::test]
    #[::serial_test::serial]
    async fn test_firestore_store_update_succeeds_when_updated_at_matches() -> ::anyhow::Result<()>
    {
        let repo = firestore_repo()?;
        let user_id = UserId::new();
        let bookmark = Bookmark::create(
            user_id,
            "https://example.com".parse::<Url>()?,
            "test title".parse::<Title>()?,
            "test comment".parse::<Comment>()?,
        );
        let original_updated_at = bookmark.updated_at();
        let id = bookmark.id();
        repo.store(None, bookmark).await?;

        let updated = Bookmark::new(
            "updated comment".parse::<Comment>()?,
            DateTime::now(),
            None,
            id,
            "updated title".parse::<Title>()?,
            DateTime::now(),
            "https://updated.example.com".parse::<Url>()?,
            user_id,
        );
        repo.store(Some(original_updated_at), updated).await?;
        Ok(())
    }

    #[::tokio::test]
    #[::serial_test::serial]
    async fn test_firestore_store_update_fails_when_updated_at_mismatches() -> ::anyhow::Result<()>
    {
        let repo = firestore_repo()?;
        let user_id = UserId::new();
        let bookmark = Bookmark::create(
            user_id,
            "https://example.com".parse::<Url>()?,
            "test title".parse::<Title>()?,
            "test comment".parse::<Comment>()?,
        );
        let id = bookmark.id();
        repo.store(None, bookmark).await?;

        let stale_updated_at = DateTime::from_rfc3339("2000-01-01T00:00:00.000Z")?;
        let updated = Bookmark::new(
            "updated comment".parse::<Comment>()?,
            DateTime::now(),
            None,
            id,
            "updated title".parse::<Title>()?,
            DateTime::now(),
            "https://updated.example.com".parse::<Url>()?,
            user_id,
        );
        assert!(repo.store(Some(stale_updated_at), updated).await.is_err());
        Ok(())
    }

    #[::tokio::test]
    #[::serial_test::serial]
    async fn test_firestore_store_delete_removes_bookmark() -> ::anyhow::Result<()> {
        let repo = firestore_repo()?;
        let user_id = UserId::new();
        let bookmark = Bookmark::create(
            user_id,
            "https://example.com".parse::<Url>()?,
            "test title".parse::<Title>()?,
            "test comment".parse::<Comment>()?,
        );
        let original_updated_at = bookmark.updated_at();
        let id = bookmark.id();
        repo.store(None, bookmark).await?;
        let deleted = Bookmark::new(
            "test comment".parse::<Comment>()?,
            DateTime::now(),
            Some(DateTime::now()),
            id,
            "test title".parse::<Title>()?,
            DateTime::now(),
            "https://example.com".parse::<Url>()?,
            user_id,
        );
        repo.store(Some(original_updated_at), deleted).await?;
        let found = repo.find(user_id, id).await?;
        assert!(found.is_none());
        Ok(())
    }
}
