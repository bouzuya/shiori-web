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
