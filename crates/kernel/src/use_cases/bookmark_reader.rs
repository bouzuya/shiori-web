use crate::BookmarkList;
use crate::BookmarkView;
use crate::PageToken;
use crate::UserId;

#[::async_trait::async_trait]
pub trait BookmarkReader: Send + Sync {
    /// ユーザーのブックマークを `created_at` 降順で最大 10 件返す。
    ///
    /// `page_token` が `None` なら最新ページ。`PageToken::Next` ならより古い側、
    /// `PageToken::Prev` ならより新しい側のページを返す。
    /// 続きがある場合、次ページ/前ページへの不透明トークン文字列を
    /// `next_page_token` / `prev_page_token` に入れる。
    async fn list(
        &self,
        user_id: UserId,
        page_token: Option<PageToken>,
    ) -> ::anyhow::Result<BookmarkList>;

    /// ユーザーの全ブックマークを `created_at` 降順で返す (ページネーションなし)。
    /// 削除は物理削除のため、生存しているブックマークのみが対象。
    async fn list_all(&self, user_id: UserId) -> ::anyhow::Result<Vec<BookmarkView>>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BookmarkView;
    use crate::PageToken;

    const PAGE_SIZE: usize = 10;

    struct InMemoryBookmarkReader {
        store: ::std::sync::Mutex<Vec<(UserId, BookmarkView)>>,
    }

    impl InMemoryBookmarkReader {
        fn new() -> Self {
            Self {
                store: ::std::sync::Mutex::new(vec![]),
            }
        }

        fn insert(&self, user_id: UserId, view: BookmarkView) -> ::anyhow::Result<()> {
            self.store
                .lock()
                .map_err(|e| ::anyhow::anyhow!("{e}"))?
                .push((user_id, view));
            Ok(())
        }
    }

    #[::async_trait::async_trait]
    impl BookmarkReader for InMemoryBookmarkReader {
        async fn list(
            &self,
            user_id: UserId,
            page_token: Option<PageToken>,
        ) -> ::anyhow::Result<BookmarkList> {
            let store = self.store.lock().map_err(|e| ::anyhow::anyhow!("{e}"))?;
            let mut items: Vec<BookmarkView> = store
                .iter()
                .filter(|(uid, _)| *uid == user_id)
                .map(|(_, v)| v.clone())
                .collect();
            items.sort_by(|a, b| b.created_at.cmp(&a.created_at));

            let next_of = |page: &[BookmarkView]| {
                page.last()
                    .map(|v| PageToken::Next(v.created_at.clone()).to_string())
            };
            let prev_of = |page: &[BookmarkView]| {
                page.first()
                    .map(|v| PageToken::Prev(v.created_at.clone()).to_string())
            };
            let (page, next_page_token, prev_page_token) = match page_token {
                None => {
                    let has_older = items.len() > PAGE_SIZE;
                    let page: Vec<_> = items.into_iter().take(PAGE_SIZE).collect();
                    let next = has_older.then(|| next_of(&page)).flatten();
                    (page, next, None)
                }
                Some(PageToken::Next(t)) => {
                    let older: Vec<_> = items.into_iter().filter(|v| v.created_at < t).collect();
                    let has_older = older.len() > PAGE_SIZE;
                    let page: Vec<_> = older.into_iter().take(PAGE_SIZE).collect();
                    let next = has_older.then(|| next_of(&page)).flatten();
                    let prev = prev_of(&page);
                    (page, next, prev)
                }
                Some(PageToken::Prev(t)) => {
                    let mut newer: Vec<_> =
                        items.into_iter().filter(|v| v.created_at > t).collect();
                    // t に近い (古い) 側から N 件取り、表示用に降順へ戻す
                    newer.sort_by(|a, b| a.created_at.cmp(&b.created_at));
                    let has_newer = newer.len() > PAGE_SIZE;
                    let mut page: Vec<_> = newer.into_iter().take(PAGE_SIZE).collect();
                    page.reverse();
                    let prev = has_newer.then(|| prev_of(&page)).flatten();
                    let next = next_of(&page);
                    (page, next, prev)
                }
            };
            Ok(BookmarkList {
                items: page,
                next_page_token,
                prev_page_token,
            })
        }

        async fn list_all(&self, user_id: UserId) -> ::anyhow::Result<Vec<BookmarkView>> {
            let store = self.store.lock().map_err(|e| ::anyhow::anyhow!("{e}"))?;
            let mut items: Vec<BookmarkView> = store
                .iter()
                .filter(|(uid, _)| *uid == user_id)
                .map(|(_, v)| v.clone())
                .collect();
            items.sort_by(|a, b| b.created_at.cmp(&a.created_at));
            Ok(items)
        }
    }

    #[::tokio::test]
    async fn test_list_empty_returns_empty() -> ::anyhow::Result<()> {
        let reader = InMemoryBookmarkReader::new();
        let user_id = UserId::new();
        let result = reader.list(user_id, None).await?;
        assert!(result.items.is_empty());
        assert!(result.next_page_token.is_none());
        Ok(())
    }

    #[::tokio::test]
    async fn test_list_less_than_page_size_returns_all_without_token() -> ::anyhow::Result<()> {
        let reader = InMemoryBookmarkReader::new();
        let user_id = UserId::new();
        for i in 0..5 {
            reader.insert(
                user_id,
                BookmarkView {
                    id: format!("id{i}"),
                    created_at: format!("2024-01-0{}T00:00:00.000Z", i + 1),
                    ..BookmarkView::for_test()
                },
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

    #[::tokio::test]
    async fn test_list_more_than_page_size_returns_page_and_token() -> ::anyhow::Result<()> {
        let reader = InMemoryBookmarkReader::new();
        let user_id = UserId::new();
        for i in 0..15 {
            reader.insert(
                user_id,
                BookmarkView {
                    id: format!("id{i:02}"),
                    created_at: format!("2024-01-{:02}T00:00:00.000Z", i + 1),
                    ..BookmarkView::for_test()
                },
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

    #[::tokio::test]
    async fn test_list_with_page_token_returns_next_page() -> ::anyhow::Result<()> {
        let reader = InMemoryBookmarkReader::new();
        let user_id = UserId::new();
        for i in 0..15 {
            reader.insert(
                user_id,
                BookmarkView {
                    id: format!("id{i:02}"),
                    created_at: format!("2024-01-{:02}T00:00:00.000Z", i + 1),
                    ..BookmarkView::for_test()
                },
            )?;
        }
        let first = reader.list(user_id, None).await?;
        let token = first
            .next_page_token
            .ok_or_else(|| ::anyhow::anyhow!("expected next_page_token"))?;
        let second = reader
            .list(user_id, Some(token.parse::<PageToken>()?))
            .await?;
        assert_eq!(second.items.len(), 5);
        assert!(second.next_page_token.is_none());
        assert_eq!(second.items[0].id, "id04");
        assert_eq!(second.items[4].id, "id00");
        Ok(())
    }

    #[::tokio::test]
    async fn test_list_filters_by_user_id() -> ::anyhow::Result<()> {
        let reader = InMemoryBookmarkReader::new();
        let user_a = UserId::new();
        let user_b = UserId::new();
        reader.insert(
            user_a,
            BookmarkView {
                id: "a1".to_string(),
                created_at: "2024-01-01T00:00:00.000Z".to_string(),
                ..BookmarkView::for_test()
            },
        )?;
        reader.insert(
            user_b,
            BookmarkView {
                id: "b1".to_string(),
                created_at: "2024-01-02T00:00:00.000Z".to_string(),
                ..BookmarkView::for_test()
            },
        )?;
        let result = reader.list(user_a, None).await?;
        assert_eq!(result.items.len(), 1);
        assert_eq!(result.items[0].id, "a1");
        Ok(())
    }

    fn insert_15(reader: &InMemoryBookmarkReader, user_id: UserId) -> ::anyhow::Result<()> {
        for i in 0..15 {
            reader.insert(
                user_id,
                BookmarkView {
                    id: format!("id{i:02}"),
                    created_at: format!("2024-01-{:02}T00:00:00.000Z", i + 1),
                    ..BookmarkView::for_test()
                },
            )?;
        }
        Ok(())
    }

    #[::tokio::test]
    async fn test_list_first_page_has_no_prev_page_token() -> ::anyhow::Result<()> {
        let reader = InMemoryBookmarkReader::new();
        let user_id = UserId::new();
        insert_15(&reader, user_id)?;
        let result = reader.list(user_id, None).await?;
        assert!(result.prev_page_token.is_none());
        Ok(())
    }

    #[::tokio::test]
    async fn test_list_next_page_has_prev_page_token() -> ::anyhow::Result<()> {
        let reader = InMemoryBookmarkReader::new();
        let user_id = UserId::new();
        insert_15(&reader, user_id)?;
        let first = reader.list(user_id, None).await?;
        let next = first
            .next_page_token
            .ok_or_else(|| ::anyhow::anyhow!("expected next_page_token"))?;
        let second = reader
            .list(user_id, Some(next.parse::<PageToken>()?))
            .await?;
        assert!(second.prev_page_token.is_some());
        Ok(())
    }

    #[::tokio::test]
    async fn test_list_prev_page_token_returns_previous_page() -> ::anyhow::Result<()> {
        let reader = InMemoryBookmarkReader::new();
        let user_id = UserId::new();
        insert_15(&reader, user_id)?;
        let first = reader.list(user_id, None).await?;
        let next = first
            .next_page_token
            .clone()
            .ok_or_else(|| ::anyhow::anyhow!("expected next_page_token"))?;
        let second = reader
            .list(user_id, Some(next.parse::<PageToken>()?))
            .await?;
        let prev = second
            .prev_page_token
            .clone()
            .ok_or_else(|| ::anyhow::anyhow!("expected prev_page_token"))?;
        let back = reader
            .list(user_id, Some(prev.parse::<PageToken>()?))
            .await?;
        let first_ids: Vec<_> = first.items.iter().map(|v| v.id.clone()).collect();
        let back_ids: Vec<_> = back.items.iter().map(|v| v.id.clone()).collect();
        assert_eq!(back_ids, first_ids);
        Ok(())
    }

    #[::tokio::test]
    async fn test_list_all_returns_all_sorted_by_created_at_desc() -> ::anyhow::Result<()> {
        let reader = InMemoryBookmarkReader::new();
        let user_id = UserId::new();
        insert_15(&reader, user_id)?;
        let all = reader.list_all(user_id).await?;
        assert_eq!(all.len(), 15);
        assert_eq!(all[0].id, "id14");
        assert_eq!(all[14].id, "id00");
        Ok(())
    }

    #[::tokio::test]
    async fn test_list_all_filters_by_user_id() -> ::anyhow::Result<()> {
        let reader = InMemoryBookmarkReader::new();
        let user_a = UserId::new();
        let user_b = UserId::new();
        reader.insert(
            user_a,
            BookmarkView {
                id: "a1".to_string(),
                created_at: "2024-01-01T00:00:00.000Z".to_string(),
                ..BookmarkView::for_test()
            },
        )?;
        reader.insert(
            user_b,
            BookmarkView {
                id: "b1".to_string(),
                created_at: "2024-01-02T00:00:00.000Z".to_string(),
                ..BookmarkView::for_test()
            },
        )?;
        let all = reader.list_all(user_a).await?;
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].id, "a1");
        Ok(())
    }
}
