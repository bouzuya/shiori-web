#[async_trait::async_trait]
pub trait BookmarkRepository: Send + Sync {
    async fn find(
        &self,
        user_id: crate::entities::UserId,
        bookmark_id: crate::entities::BookmarkId,
    ) -> anyhow::Result<Option<crate::entities::Bookmark>>;
    /// `updated_at` が `None` のとき新規作成を試みる（既存があればエラー）。
    /// `updated_at` が `Some(t)` のとき既存の `updated_at` と `t` が一致すれば更新する（楽観的排他制御）。
    async fn store(
        &self,
        updated_at: Option<crate::entities::DateTime>,
        bookmark: crate::entities::Bookmark,
    ) -> anyhow::Result<()>;
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Mutex;

    use super::*;

    struct InMemoryBookmarkRepository {
        store: Mutex<HashMap<crate::entities::BookmarkId, crate::entities::Bookmark>>,
    }

    impl InMemoryBookmarkRepository {
        fn new() -> Self {
            Self {
                store: Mutex::new(HashMap::new()),
            }
        }
    }

    #[async_trait::async_trait]
    impl BookmarkRepository for InMemoryBookmarkRepository {
        async fn find(
            &self,
            _user_id: crate::entities::UserId,
            bookmark_id: crate::entities::BookmarkId,
        ) -> anyhow::Result<Option<crate::entities::Bookmark>> {
            Ok(self
                .store
                .lock()
                .map_err(|e| anyhow::anyhow!("{e}"))?
                .get(&bookmark_id)
                .cloned())
        }

        async fn store(
            &self,
            updated_at: Option<crate::entities::DateTime>,
            bookmark: crate::entities::Bookmark,
        ) -> anyhow::Result<()> {
            let mut store = self.store.lock().map_err(|e| anyhow::anyhow!("{e}"))?;
            match updated_at {
                None => {
                    anyhow::ensure!(
                        !store.contains_key(&bookmark.id()),
                        "bookmark already exists"
                    );
                    store.insert(bookmark.id(), bookmark);
                }
                Some(t) => {
                    let existing = store
                        .get(&bookmark.id())
                        .ok_or_else(|| anyhow::anyhow!("bookmark not found"))?;
                    anyhow::ensure!(existing.updated_at() == t, "optimistic lock conflict");
                    store.insert(bookmark.id(), bookmark);
                }
            }
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_store_new_and_find() -> anyhow::Result<()> {
        let repo = InMemoryBookmarkRepository::new();
        let user_id = crate::entities::UserId::new();
        let bookmark = crate::entities::Bookmark::create(
            user_id,
            crate::entities::Url::for_test(),
            crate::entities::Title::for_test(),
            crate::entities::Comment::for_test(),
        );
        let id = bookmark.id();
        repo.store(None, bookmark).await?;
        let found = repo.find(user_id, id).await?;
        let found = found.ok_or_else(|| anyhow::anyhow!("bookmark not found"))?;
        assert_eq!(found.id(), id);
        Ok(())
    }

    #[tokio::test]
    async fn test_store_new_fails_if_already_exists() -> anyhow::Result<()> {
        let repo = InMemoryBookmarkRepository::new();
        let user_id = crate::entities::UserId::new();
        let bookmark = crate::entities::Bookmark::create(
            user_id,
            crate::entities::Url::for_test(),
            crate::entities::Title::for_test(),
            crate::entities::Comment::for_test(),
        );
        repo.store(None, bookmark.clone()).await?;
        assert!(repo.store(None, bookmark).await.is_err());
        Ok(())
    }

    #[tokio::test]
    async fn test_store_update_succeeds_when_updated_at_matches() -> anyhow::Result<()> {
        let repo = InMemoryBookmarkRepository::new();
        let user_id = crate::entities::UserId::new();
        let bookmark = crate::entities::Bookmark::create(
            user_id,
            crate::entities::Url::for_test(),
            crate::entities::Title::for_test(),
            crate::entities::Comment::for_test(),
        );
        let original_updated_at = bookmark.updated_at();
        let id = bookmark.id();
        repo.store(None, bookmark).await?;

        let updated = crate::entities::Bookmark::new(
            crate::entities::Comment::for_test(),
            crate::entities::DateTime::now(),
            id,
            crate::entities::Title::for_test(),
            crate::entities::DateTime::now(),
            crate::entities::Url::for_test(),
            user_id,
        );
        repo.store(Some(original_updated_at), updated).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_store_update_fails_when_updated_at_mismatches() -> anyhow::Result<()> {
        let repo = InMemoryBookmarkRepository::new();
        let user_id = crate::entities::UserId::new();
        let bookmark = crate::entities::Bookmark::create(
            user_id,
            crate::entities::Url::for_test(),
            crate::entities::Title::for_test(),
            crate::entities::Comment::for_test(),
        );
        let id = bookmark.id();
        repo.store(None, bookmark).await?;

        let stale_updated_at = crate::entities::DateTime::from_rfc3339("2000-01-01T00:00:00.000Z")?;
        let updated = crate::entities::Bookmark::new(
            crate::entities::Comment::for_test(),
            crate::entities::DateTime::now(),
            id,
            crate::entities::Title::for_test(),
            crate::entities::DateTime::now(),
            crate::entities::Url::for_test(),
            user_id,
        );
        assert!(repo.store(Some(stale_updated_at), updated).await.is_err());
        Ok(())
    }

    #[tokio::test]
    async fn test_find_returns_none_for_missing() -> anyhow::Result<()> {
        let repo = InMemoryBookmarkRepository::new();
        let user_id = crate::entities::UserId::new();
        let bookmark_id = crate::entities::BookmarkId::new();
        let found = repo.find(user_id, bookmark_id).await?;
        assert!(found.is_none());
        Ok(())
    }
}
