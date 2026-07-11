/// export NDJSON の1レコード。server が返した生の JSON 行と、マージ (`id`)・
/// ソート (`created_at`)・`since` 導出 (`updated_at`) に必要なキーを保持する。
/// 行を生のまま保持するのは、server 側のスキーマ進化 (フィールド追加) で
/// データを欠落させず、出力を server の応答とバイト単位で一致させるため。
// export が消費するまで bin では未使用。
#[allow(dead_code)]
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

#[allow(dead_code)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_extracts_keys() -> ::anyhow::Result<()> {
        let line = r#"{"comment":"c","created_at":"2026-07-06T23:06:49.751Z","id":"019f39af-0000-7000-8000-000000000000","title":"t","updated_at":"2026-07-07T00:00:00.000Z","url":"https://example.com/"}"#;
        let bookmark = CachedBookmark::parse(line)?;
        assert_eq!(bookmark.created_at(), "2026-07-06T23:06:49.751Z");
        assert_eq!(bookmark.id(), "019f39af-0000-7000-8000-000000000000");
        assert_eq!(bookmark.updated_at(), "2026-07-07T00:00:00.000Z");
        assert_eq!(bookmark.line(), line);
        Ok(())
    }

    #[test]
    fn parse_keeps_line_verbatim_including_unknown_fields() -> ::anyhow::Result<()> {
        let line = r#"{"created_at":"2026-01-01T00:00:00.000Z","future_field":"x","id":"i1","updated_at":"2026-01-01T00:00:00.000Z"}"#;
        let bookmark = CachedBookmark::parse(line)?;
        assert_eq!(bookmark.line(), line);
        Ok(())
    }

    #[test]
    fn parse_errors_on_invalid_json() {
        assert!(CachedBookmark::parse("not json").is_err());
    }

    #[test]
    fn parse_errors_when_id_is_missing() {
        let line =
            r#"{"created_at":"2026-01-01T00:00:00.000Z","updated_at":"2026-01-01T00:00:00.000Z"}"#;
        assert!(CachedBookmark::parse(line).is_err());
    }

    #[test]
    fn for_test_round_trips_through_parse() -> ::anyhow::Result<()> {
        let bookmark = CachedBookmark::for_test();
        assert_eq!(CachedBookmark::parse(bookmark.line())?, bookmark);
        Ok(())
    }
}
