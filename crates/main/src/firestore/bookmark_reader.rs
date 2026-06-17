use crate::firestore::BookmarkDocumentData;
use crate::firestore::BookmarksCollection;
use crate::firestore::FirestoreCollection;
use kernel::BookmarkReader;

const PAGE_SIZE: usize = 10;

pub(crate) struct FirestoreBookmarkReader {
    firestore: bouzuya_firestore_client::Firestore,
}

impl FirestoreBookmarkReader {
    pub(crate) fn new(firestore: bouzuya_firestore_client::Firestore) -> Self {
        Self { firestore }
    }
}

#[async_trait::async_trait]
impl BookmarkReader for FirestoreBookmarkReader {
    async fn list(
        &self,
        user_id: kernel::UserId,
        page_token: Option<String>,
    ) -> anyhow::Result<kernel::BookmarkList> {
        let collection_ref = self
            .firestore
            .collection(BookmarksCollection::collection_path(&user_id))
            .map_err(|e| anyhow::anyhow!(e))?;
        let mut query = collection_ref
            .order_by("created_at", "desc")
            .map_err(|e| anyhow::anyhow!(e))?
            .limit(i32::try_from(PAGE_SIZE + 1)?)
            .map_err(|e| anyhow::anyhow!(e))?;
        if let Some(t) = page_token {
            query = query.start_after([t]).map_err(|e| anyhow::anyhow!(e))?;
        }
        let snapshot = query.get().await.map_err(|e| anyhow::anyhow!(e))?;
        let mut views: Vec<kernel::BookmarkView> = Vec::new();
        for doc in snapshot {
            let data = doc
                .data::<BookmarkDocumentData>()
                .map_err(|e| anyhow::anyhow!(e))?;
            views.push(data.into_bookmark_view(user_id));
        }
        let has_more = views.len() > PAGE_SIZE;
        let page: Vec<_> = views.into_iter().take(PAGE_SIZE).collect();
        let next_page_token = if has_more {
            page.last().map(|v| v.created_at.clone())
        } else {
            None
        };
        Ok(kernel::BookmarkList {
            items: page,
            next_page_token,
            // TODO(step 4): Prev 方向を実装する
            prev_page_token: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn firestore_reader_and_repo() -> anyhow::Result<(
        FirestoreBookmarkReader,
        crate::firestore::FirestoreBookmarkRepository,
    )> {
        let firestore = bouzuya_firestore_client::Firestore::new(
            bouzuya_firestore_client::FirestoreOptions::default(),
        )?;
        Ok((
            FirestoreBookmarkReader::new(firestore.clone()),
            crate::firestore::FirestoreBookmarkRepository::new(firestore),
        ))
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_list_returns_empty_for_unknown_user() -> anyhow::Result<()> {
        let (reader, _repo) = firestore_reader_and_repo()?;
        let user_id = kernel::UserId::new();
        let result = reader.list(user_id, None).await?;
        assert!(result.items.is_empty());
        assert!(result.next_page_token.is_none());
        Ok(())
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_list_returns_stored_bookmark() -> anyhow::Result<()> {
        use kernel::BookmarkRepository as _;
        let (reader, repo) = firestore_reader_and_repo()?;
        let user_id = kernel::UserId::new();
        let bookmark = kernel::Bookmark::create(
            user_id,
            "https://example.com".parse::<kernel::Url>()?,
            "title".parse::<kernel::Title>()?,
            "comment".parse::<kernel::Comment>()?,
        );
        let bookmark_id = bookmark.id();
        repo.store(None, bookmark).await?;
        let result = reader.list(user_id, None).await?;
        assert_eq!(result.items.len(), 1);
        assert_eq!(result.items[0].id, bookmark_id.to_string());
        assert_eq!(result.items[0].user_id, user_id.to_string());
        assert!(result.next_page_token.is_none());
        Ok(())
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_list_sorts_by_created_at_desc() -> anyhow::Result<()> {
        use kernel::BookmarkRepository as _;
        let (reader, repo) = firestore_reader_and_repo()?;
        let user_id = kernel::UserId::new();
        let older = kernel::Bookmark::new(
            "c".parse::<kernel::Comment>()?,
            kernel::DateTime::from_rfc3339("2024-01-01T00:00:00.000Z")?,
            None,
            kernel::BookmarkId::new(),
            "t".parse::<kernel::Title>()?,
            kernel::DateTime::from_rfc3339("2024-01-01T00:00:00.000Z")?,
            "https://example.com/older".parse::<kernel::Url>()?,
            user_id,
        );
        let newer = kernel::Bookmark::new(
            "c".parse::<kernel::Comment>()?,
            kernel::DateTime::from_rfc3339("2024-06-01T00:00:00.000Z")?,
            None,
            kernel::BookmarkId::new(),
            "t".parse::<kernel::Title>()?,
            kernel::DateTime::from_rfc3339("2024-06-01T00:00:00.000Z")?,
            "https://example.com/newer".parse::<kernel::Url>()?,
            user_id,
        );
        let older_id = older.id();
        let newer_id = newer.id();
        repo.store(None, older).await?;
        repo.store(None, newer).await?;
        let result = reader.list(user_id, None).await?;
        assert_eq!(result.items.len(), 2);
        assert_eq!(result.items[0].id, newer_id.to_string());
        assert_eq!(result.items[1].id, older_id.to_string());
        Ok(())
    }
}
