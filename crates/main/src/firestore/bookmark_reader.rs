use crate::BookmarkDocumentData;
use crate::BookmarksCollection;
use crate::FirestoreCollection;
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
        page_token: Option<kernel::PageToken>,
    ) -> anyhow::Result<kernel::BookmarkList> {
        let collection_ref = self
            .firestore
            .collection(BookmarksCollection::collection_path(&user_id))
            .map_err(|e| anyhow::anyhow!(e))?;
        // Prev は表示順 (desc) の逆向きへ進むため asc で取得し、表示用に reverse する。
        let direction = match page_token {
            None | Some(kernel::PageToken::Next(_)) => "desc",
            Some(kernel::PageToken::Prev(_)) => "asc",
        };
        let mut query = collection_ref
            .order_by("created_at", direction)
            .map_err(|e| anyhow::anyhow!(e))?
            .limit(i32::try_from(PAGE_SIZE + 1)?)
            .map_err(|e| anyhow::anyhow!(e))?;
        if let Some(kernel::PageToken::Next(t) | kernel::PageToken::Prev(t)) = &page_token {
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
        let mut page: Vec<_> = views.into_iter().take(PAGE_SIZE).collect();
        match page_token {
            None | Some(kernel::PageToken::Next(_)) => {
                // desc で取得しているのでこのままでOK
            }
            Some(kernel::PageToken::Prev(_)) => {
                // asc で取得しているので表示用に created_at 降順へ戻す
                page.reverse();
            }
        }
        // first = 表示先頭 (最新側), last = 表示末尾 (最古側)
        let next_of = |page: &[kernel::BookmarkView]| {
            page.last()
                .map(|v| kernel::PageToken::Next(v.created_at.clone()).to_string())
        };
        let prev_of = |page: &[kernel::BookmarkView]| {
            page.first()
                .map(|v| kernel::PageToken::Prev(v.created_at.clone()).to_string())
        };
        let (next_page_token, prev_page_token) = match page_token {
            None => (has_more.then(|| next_of(&page)).flatten(), None),
            Some(kernel::PageToken::Next(_)) => {
                (has_more.then(|| next_of(&page)).flatten(), prev_of(&page))
            }
            Some(kernel::PageToken::Prev(_)) => {
                (next_of(&page), has_more.then(|| prev_of(&page)).flatten())
            }
        };
        Ok(kernel::BookmarkList {
            items: page,
            next_page_token,
            prev_page_token,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::FirestoreBookmarkRepository;

    use super::*;

    fn firestore_reader_and_repo()
    -> anyhow::Result<(FirestoreBookmarkReader, FirestoreBookmarkRepository)> {
        let firestore = bouzuya_firestore_client::Firestore::new(
            bouzuya_firestore_client::FirestoreOptions::default(),
        )?;
        Ok((
            FirestoreBookmarkReader::new(firestore.clone()),
            FirestoreBookmarkRepository::new(firestore),
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

    async fn insert_n(
        repo: &FirestoreBookmarkRepository,
        user_id: kernel::UserId,
        n: usize,
    ) -> anyhow::Result<()> {
        use kernel::BookmarkRepository as _;
        for i in 0..n {
            let created_at =
                kernel::DateTime::from_rfc3339(&format!("2024-01-{:02}T00:00:00.000Z", i + 1))?;
            let bookmark = kernel::Bookmark::new(
                "c".parse::<kernel::Comment>()?,
                created_at,
                None,
                kernel::BookmarkId::new(),
                "t".parse::<kernel::Title>()?,
                created_at,
                format!("https://example.com/{i}").parse::<kernel::Url>()?,
                user_id,
            );
            repo.store(None, bookmark).await?;
        }
        Ok(())
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_list_first_page_has_no_prev_page_token() -> anyhow::Result<()> {
        let (reader, repo) = firestore_reader_and_repo()?;
        let user_id = kernel::UserId::new();
        insert_n(&repo, user_id, 15).await?;
        let result = reader.list(user_id, None).await?;
        assert!(result.prev_page_token.is_none());
        Ok(())
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_list_next_page_has_prev_page_token() -> anyhow::Result<()> {
        let (reader, repo) = firestore_reader_and_repo()?;
        let user_id = kernel::UserId::new();
        insert_n(&repo, user_id, 15).await?;
        let first = reader.list(user_id, None).await?;
        let next = first
            .next_page_token
            .ok_or_else(|| anyhow::anyhow!("expected next_page_token"))?;
        let second = reader
            .list(user_id, Some(next.parse::<kernel::PageToken>()?))
            .await?;
        assert!(second.prev_page_token.is_some());
        Ok(())
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_list_prev_page_token_returns_previous_page() -> anyhow::Result<()> {
        let (reader, repo) = firestore_reader_and_repo()?;
        let user_id = kernel::UserId::new();
        insert_n(&repo, user_id, 15).await?;
        let first = reader.list(user_id, None).await?;
        let next = first
            .next_page_token
            .clone()
            .ok_or_else(|| anyhow::anyhow!("expected next_page_token"))?;
        let second = reader
            .list(user_id, Some(next.parse::<kernel::PageToken>()?))
            .await?;
        let prev = second
            .prev_page_token
            .clone()
            .ok_or_else(|| anyhow::anyhow!("expected prev_page_token"))?;
        let back = reader
            .list(user_id, Some(prev.parse::<kernel::PageToken>()?))
            .await?;
        let first_ids: Vec<_> = first.items.iter().map(|v| v.id.clone()).collect();
        let back_ids: Vec<_> = back.items.iter().map(|v| v.id.clone()).collect();
        assert_eq!(back_ids, first_ids);
        Ok(())
    }
}
