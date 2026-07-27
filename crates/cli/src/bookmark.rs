/// export NDJSON の1レコード。server が返した生の JSON 行と、マージ (`id`)・
/// ソート (`created_at`)・`since` 導出 (`updated_at`) に必要なキーを保持する。
/// 行を生のまま保持するのは、server 側のスキーマ進化 (フィールド追加) で
/// データを欠落させず、出力を server の応答とバイト単位で一致させるため。
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CachedBookmark {
    created_at: String,
    id: String,
    line: String,
    updated_at: String,
}

#[derive(::serde::Deserialize)]
struct Keys {
    created_at: String,
    id: String,
    updated_at: String,
}

impl CachedBookmark {
    pub(crate) fn parse(line: &str) -> ::anyhow::Result<Self> {
        let keys: Keys = ::serde_json::from_str(line)?;
        Ok(Self {
            created_at: keys.created_at,
            id: keys.id,
            line: line.to_string(),
            updated_at: keys.updated_at,
        })
    }

    pub(crate) fn created_at(&self) -> &str {
        &self.created_at
    }

    pub(crate) fn id(&self) -> &str {
        &self.id
    }

    pub(crate) fn line(&self) -> &str {
        &self.line
    }

    pub(crate) fn updated_at(&self) -> &str {
        &self.updated_at
    }
}

#[cfg(test)]
impl CachedBookmark {
    pub(crate) fn for_test() -> Self {
        let nanos = ::std::time::SystemTime::now()
            .duration_since(::std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let created_at = format!(
            "2026-01-01T00:00:{:02}.{:03}Z",
            nanos % 60,
            (nanos / 60) % 1000
        );
        let id = format!("id-{nanos}");
        let updated_at = format!(
            "2026-01-02T00:00:{:02}.{:03}Z",
            nanos % 60,
            (nanos / 7) % 1000
        );
        let line = ::serde_json::json!({
            "comment": format!("comment-{nanos}"),
            "created_at": created_at,
            "id": id,
            "title": format!("title-{nanos}"),
            "updated_at": updated_at,
            "url": format!("https://example.com/{nanos}"),
        })
        .to_string();
        Self {
            created_at,
            id,
            line,
            updated_at,
        }
    }
}

/// cache と差分を id でマージする。同一 id は incoming (差分) で上書き。
pub(crate) fn merge_bookmarks(
    cache: Vec<CachedBookmark>,
    incoming: Vec<CachedBookmark>,
) -> Vec<CachedBookmark> {
    let mut map: ::std::collections::HashMap<String, CachedBookmark> =
        cache.into_iter().map(|b| (b.id().to_string(), b)).collect();
    for b in incoming {
        map.insert(b.id().to_string(), b);
    }
    map.into_values().collect()
}

/// `created_at` 降順、同時刻なら `id` 降順でソートする。
/// タイムスタンプは固定幅 RFC3339 UTC (例: `2026-07-06T23:06:49.751Z`) を前提とし、
/// 辞書順 = 時刻順が成り立つ。
pub(crate) fn sort_bookmarks(bookmarks: &mut [CachedBookmark]) {
    bookmarks.sort_by(|a, b| {
        b.created_at()
            .cmp(a.created_at())
            .then_with(|| b.id().cmp(a.id()))
    });
}

/// `updated_at` の最大値を返す。空なら `None`。
/// 次回リクエストの `since` パラメーターに使う。
pub(crate) fn max_updated_at(bookmarks: &[CachedBookmark]) -> Option<&str> {
    bookmarks.iter().map(|b| b.updated_at()).max()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bookmark(id: &str, created_at: &str, updated_at: &str) -> CachedBookmark {
        let line = ::serde_json::json!({
            "comment": "",
            "created_at": created_at,
            "id": id,
            "title": "",
            "updated_at": updated_at,
            "url": "https://example.com/",
        })
        .to_string();
        CachedBookmark::parse(&line).expect("test bookmark should parse")
    }

    #[test]
    fn max_updated_at_returns_max() {
        let bookmarks = vec![
            bookmark("a", "2026-01-01T00:00:00.000Z", "2026-01-01T00:00:00.000Z"),
            bookmark("b", "2026-01-02T00:00:00.000Z", "2026-01-03T00:00:00.000Z"),
            bookmark("c", "2026-01-03T00:00:00.000Z", "2026-01-02T00:00:00.000Z"),
        ];
        assert_eq!(max_updated_at(&bookmarks), Some("2026-01-03T00:00:00.000Z"));
    }

    #[test]
    fn max_updated_at_returns_none_for_empty() {
        let bookmarks: Vec<CachedBookmark> = vec![];
        assert_eq!(max_updated_at(&bookmarks), None);
    }

    #[test]
    fn merge_bookmarks_combines_cache_and_incoming() {
        let cache = vec![bookmark(
            "a",
            "2026-01-01T00:00:00.000Z",
            "2026-01-01T00:00:00.000Z",
        )];
        let incoming = vec![bookmark(
            "b",
            "2026-01-02T00:00:00.000Z",
            "2026-01-02T00:00:00.000Z",
        )];
        let merged = merge_bookmarks(cache, incoming);
        assert_eq!(merged.len(), 2);
        assert!(merged.iter().any(|b| b.id() == "a"));
        assert!(merged.iter().any(|b| b.id() == "b"));
    }

    #[test]
    fn merge_bookmarks_incoming_overwrites_cache_by_id() {
        let cache = vec![bookmark(
            "a",
            "2026-01-01T00:00:00.000Z",
            "2026-01-01T00:00:00.000Z",
        )];
        let incoming = vec![bookmark(
            "a",
            "2026-01-01T00:00:00.000Z",
            "2026-01-02T00:00:00.000Z",
        )];
        let merged = merge_bookmarks(cache, incoming);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].updated_at(), "2026-01-02T00:00:00.000Z");
    }

    #[test]
    fn merge_bookmarks_empty_cache() {
        let incoming = vec![bookmark(
            "a",
            "2026-01-01T00:00:00.000Z",
            "2026-01-01T00:00:00.000Z",
        )];
        let merged = merge_bookmarks(vec![], incoming);
        assert_eq!(merged.len(), 1);
    }

    #[test]
    fn merge_bookmarks_empty_incoming() {
        let cache = vec![bookmark(
            "a",
            "2026-01-01T00:00:00.000Z",
            "2026-01-01T00:00:00.000Z",
        )];
        let merged = merge_bookmarks(cache, vec![]);
        assert_eq!(merged.len(), 1);
    }

    #[test]
    fn sort_bookmarks_by_created_at_desc() {
        let mut bookmarks = vec![
            bookmark("a", "2026-01-01T00:00:00.000Z", "2026-01-01T00:00:00.000Z"),
            bookmark("b", "2026-01-03T00:00:00.000Z", "2026-01-03T00:00:00.000Z"),
            bookmark("c", "2026-01-02T00:00:00.000Z", "2026-01-02T00:00:00.000Z"),
        ];
        sort_bookmarks(&mut bookmarks);
        assert_eq!(bookmarks[0].id(), "b");
        assert_eq!(bookmarks[1].id(), "c");
        assert_eq!(bookmarks[2].id(), "a");
    }

    #[test]
    fn sort_bookmarks_by_id_desc_when_created_at_equal() {
        let mut bookmarks = vec![
            bookmark("a", "2026-01-01T00:00:00.000Z", "2026-01-01T00:00:00.000Z"),
            bookmark("c", "2026-01-01T00:00:00.000Z", "2026-01-01T00:00:00.000Z"),
            bookmark("b", "2026-01-01T00:00:00.000Z", "2026-01-01T00:00:00.000Z"),
        ];
        sort_bookmarks(&mut bookmarks);
        assert_eq!(bookmarks[0].id(), "c");
        assert_eq!(bookmarks[1].id(), "b");
        assert_eq!(bookmarks[2].id(), "a");
    }
}
