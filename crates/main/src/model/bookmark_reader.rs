pub(crate) use kernel::BookmarkReader;

const PAGE_SIZE: usize = 10;

#[derive(serde::Deserialize, serde::Serialize)]
struct BookmarkDocumentData {
    bookmark_id: String,
    comment: String,
    created_at: String,
    title: String,
    updated_at: String,
    url: String,
}

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
    // TODO: 現状の bouzuya-firestore-client は orderBy / limit / startAfter を伴う
    // structured query API を公開していないため、コレクション全件を取得してから
    // メモリ上で降順ソート・ページング・フィルタを行っている。件数が増えるにつれて
    // 読み取りコストとレイテンシが線形に悪化するため、クライアント側にクエリ API が
    // 追加され次第、サーバーサイドでの orderBy(created_at desc) + limit(PAGE_SIZE+1)
    // + startAfter(page_token) に置き換えること。
    async fn list(
        &self,
        user_id: kernel::UserId,
        page_token: Option<String>,
    ) -> anyhow::Result<kernel::BookmarkList> {
        let collection_ref = self
            .firestore
            .collection(format!("users/{user_id}/bookmarks"))
            .map_err(|e| anyhow::anyhow!(e))?;
        let doc_refs = collection_ref
            .list_documents()
            .await
            .map_err(|e| anyhow::anyhow!(e))?;
        if doc_refs.is_empty() {
            return Ok(kernel::BookmarkList {
                items: vec![],
                next_page_token: None,
            });
        }
        let snapshots = self
            .firestore
            .get_all(doc_refs)
            .await
            .map_err(|e| anyhow::anyhow!(e))?;
        let user_id_str = user_id.to_string();
        let mut views: Vec<kernel::BookmarkView> = Vec::new();
        for snapshot in snapshots {
            if !snapshot.exists() {
                continue;
            }
            let data = snapshot
                .data::<BookmarkDocumentData>()
                .ok_or_else(|| anyhow::anyhow!("document data is missing"))?
                .map_err(|e| anyhow::anyhow!(e))?;
            views.push(kernel::BookmarkView {
                comment: data.comment,
                created_at: data.created_at,
                id: data.bookmark_id,
                title: data.title,
                updated_at: data.updated_at,
                url: data.url,
                user_id: user_id_str.clone(),
            });
        }
        views.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        let filtered: Vec<_> = match page_token {
            None => views,
            Some(t) => views.into_iter().filter(|v| v.created_at < t).collect(),
        };
        let has_more = filtered.len() > PAGE_SIZE;
        let page: Vec<_> = filtered.into_iter().take(PAGE_SIZE).collect();
        let next_page_token = if has_more {
            page.last().map(|v| v.created_at.clone())
        } else {
            None
        };
        Ok(kernel::BookmarkList {
            items: page,
            next_page_token,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn firestore_reader_and_repo() -> anyhow::Result<(
        FirestoreBookmarkReader,
        crate::model::FirestoreBookmarkRepository,
    )> {
        let firestore = bouzuya_firestore_client::Firestore::new(
            bouzuya_firestore_client::FirestoreOptions::default(),
        )?;
        Ok((
            FirestoreBookmarkReader::new(firestore.clone()),
            crate::model::FirestoreBookmarkRepository::new(firestore),
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
            kernel::BookmarkId::new(),
            "t".parse::<kernel::Title>()?,
            kernel::DateTime::from_rfc3339("2024-01-01T00:00:00.000Z")?,
            "https://example.com/older".parse::<kernel::Url>()?,
            user_id,
        );
        let newer = kernel::Bookmark::new(
            "c".parse::<kernel::Comment>()?,
            kernel::DateTime::from_rfc3339("2024-06-01T00:00:00.000Z")?,
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
