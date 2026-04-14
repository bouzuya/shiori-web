#[async_trait::async_trait]
pub trait BookmarkReader: Send + Sync {
    /// ユーザーのブックマークを `created_at` 降順で最大 10 件返す。
    /// `page_token` を渡すと、その `created_at` より古い要素を返す。
    /// 続きがある場合 `next_page_token` に次の `page_token` を入れる。
    async fn list(
        &self,
        user_id: crate::entities::UserId,
        page_token: Option<String>,
    ) -> anyhow::Result<crate::read_models::BookmarkList>;
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    const PAGE_SIZE: usize = 10;

    struct InMemoryBookmarkReader {
        store: Mutex<Vec<(crate::entities::UserId, crate::read_models::BookmarkView)>>,
    }

    impl InMemoryBookmarkReader {
        fn new() -> Self {
            Self {
                store: Mutex::new(vec![]),
            }
        }

        fn insert(
            &self,
            user_id: crate::entities::UserId,
            view: crate::read_models::BookmarkView,
        ) -> anyhow::Result<()> {
            self.store
                .lock()
                .map_err(|e| anyhow::anyhow!("{e}"))?
                .push((user_id, view));
            Ok(())
        }
    }

    #[async_trait::async_trait]
    impl BookmarkReader for InMemoryBookmarkReader {
        async fn list(
            &self,
            user_id: crate::entities::UserId,
            page_token: Option<String>,
        ) -> anyhow::Result<crate::read_models::BookmarkList> {
            let store = self.store.lock().map_err(|e| anyhow::anyhow!("{e}"))?;
            let mut items: Vec<crate::read_models::BookmarkView> = store
                .iter()
                .filter(|(uid, _)| *uid == user_id)
                .map(|(_, v)| v.clone())
                .collect();
            items.sort_by(|a, b| b.created_at.cmp(&a.created_at));
            let filtered: Vec<_> = match page_token {
                None => items,
                Some(t) => items.into_iter().filter(|v| v.created_at < t).collect(),
            };
            let has_more = filtered.len() > PAGE_SIZE;
            let page: Vec<_> = filtered.into_iter().take(PAGE_SIZE).collect();
            let next_page_token = if has_more {
                page.last().map(|v| v.created_at.clone())
            } else {
                None
            };
            Ok(crate::read_models::BookmarkList {
                items: page,
                next_page_token,
            })
        }
    }

    fn make_view(id: &str, created_at: &str) -> crate::read_models::BookmarkView {
        crate::read_models::BookmarkView {
            comment: "c".to_string(),
            created_at: created_at.to_string(),
            id: id.to_string(),
            title: "t".to_string(),
            updated_at: created_at.to_string(),
            url: "https://example.com/".to_string(),
            user_id: "u".to_string(),
        }
    }

    #[tokio::test]
    async fn test_list_empty_returns_empty() -> anyhow::Result<()> {
        let reader = InMemoryBookmarkReader::new();
        let user_id = crate::entities::UserId::new();
        let result = reader.list(user_id, None).await?;
        assert!(result.items.is_empty());
        assert!(result.next_page_token.is_none());
        Ok(())
    }

    #[tokio::test]
    async fn test_list_less_than_page_size_returns_all_without_token() -> anyhow::Result<()> {
        let reader = InMemoryBookmarkReader::new();
        let user_id = crate::entities::UserId::new();
        for i in 0..5 {
            reader.insert(
                user_id,
                make_view(
                    &format!("id{i}"),
                    &format!("2024-01-0{}T00:00:00.000Z", i + 1),
                ),
            )?;
        }
        let result = reader.list(user_id, None).await?;
        assert_eq!(result.items.len(), 5);
        assert!(result.next_page_token.is_none());
        // 降順で最新が先頭
        assert_eq!(result.items[0].id, "id4");
        assert_eq!(result.items[4].id, "id0");
        Ok(())
    }

    #[tokio::test]
    async fn test_list_more_than_page_size_returns_page_and_token() -> anyhow::Result<()> {
        let reader = InMemoryBookmarkReader::new();
        let user_id = crate::entities::UserId::new();
        for i in 0..15 {
            reader.insert(
                user_id,
                make_view(
                    &format!("id{i:02}"),
                    &format!("2024-01-{:02}T00:00:00.000Z", i + 1),
                ),
            )?;
        }
        let result = reader.list(user_id, None).await?;
        assert_eq!(result.items.len(), 10);
        assert!(result.next_page_token.is_some());
        // 最新 10 件 (id14 -> id05)
        assert_eq!(result.items[0].id, "id14");
        assert_eq!(result.items[9].id, "id05");
        Ok(())
    }

    #[tokio::test]
    async fn test_list_with_page_token_returns_next_page() -> anyhow::Result<()> {
        let reader = InMemoryBookmarkReader::new();
        let user_id = crate::entities::UserId::new();
        for i in 0..15 {
            reader.insert(
                user_id,
                make_view(
                    &format!("id{i:02}"),
                    &format!("2024-01-{:02}T00:00:00.000Z", i + 1),
                ),
            )?;
        }
        let first = reader.list(user_id, None).await?;
        let token = first
            .next_page_token
            .ok_or_else(|| anyhow::anyhow!("expected next_page_token"))?;
        let second = reader.list(user_id, Some(token)).await?;
        assert_eq!(second.items.len(), 5);
        assert!(second.next_page_token.is_none());
        assert_eq!(second.items[0].id, "id04");
        assert_eq!(second.items[4].id, "id00");
        Ok(())
    }

    #[tokio::test]
    async fn test_list_filters_by_user_id() -> anyhow::Result<()> {
        let reader = InMemoryBookmarkReader::new();
        let user_a = crate::entities::UserId::new();
        let user_b = crate::entities::UserId::new();
        reader.insert(user_a, make_view("a1", "2024-01-01T00:00:00.000Z"))?;
        reader.insert(user_b, make_view("b1", "2024-01-02T00:00:00.000Z"))?;
        let result = reader.list(user_a, None).await?;
        assert_eq!(result.items.len(), 1);
        assert_eq!(result.items[0].id, "a1");
        Ok(())
    }
}
