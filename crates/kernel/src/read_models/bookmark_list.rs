#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BookmarkList {
    pub items: Vec<crate::read_models::BookmarkView>,
    pub next_page_token: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bookmark_list_empty() -> anyhow::Result<()> {
        let list = BookmarkList {
            items: vec![],
            next_page_token: None,
        };
        assert!(list.items.is_empty());
        assert!(list.next_page_token.is_none());
        Ok(())
    }

    #[test]
    fn test_bookmark_list_with_items_and_token() -> anyhow::Result<()> {
        let view = crate::read_models::BookmarkView::for_test();
        let list = BookmarkList {
            items: vec![view.clone()],
            next_page_token: Some("token".to_string()),
        };
        assert_eq!(list.items.len(), 1);
        assert_eq!(list.items[0], view);
        assert_eq!(list.next_page_token.as_deref(), Some("token"));
        Ok(())
    }

    #[test]
    fn test_bookmark_list_clone_eq() -> anyhow::Result<()> {
        let list = BookmarkList {
            items: vec![],
            next_page_token: Some("t".to_string()),
        };
        assert_eq!(list.clone(), list);
        Ok(())
    }
}
