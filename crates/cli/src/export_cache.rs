const APP_DIR: &str = "shiori";
const CACHE_FILE: &str = "export.ndjson";

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

/// `$XDG_CACHE_HOME/shiori/export.ndjson` (未設定なら `$HOME/.cache/shiori/export.ndjson`)
/// に export 結果の local cache を読み書きする。
// export / login が消費するまで bin では未使用。
#[allow(dead_code)]
pub(crate) struct ExportCache {
    path: ::std::path::PathBuf,
}

#[allow(dead_code)]
impl ExportCache {
    pub(crate) fn new(cache_home: impl AsRef<::std::path::Path>) -> Self {
        Self {
            path: cache_home.as_ref().join(APP_DIR).join(CACHE_FILE),
        }
    }

    pub(crate) fn from_env() -> ::anyhow::Result<Self> {
        let cache_home = resolve_cache_home(
            ::std::env::var("XDG_CACHE_HOME").ok().as_deref(),
            ::std::env::var("HOME").ok().as_deref(),
        )?;
        Ok(Self::new(cache_home))
    }

    /// cache を読み込む。ファイル不在なら `None`。
    /// 壊れている (parse できない行がある) 場合は stderr に警告を出して `None`
    /// を返し、呼び出し側が全件取得で作り直せるようにする。
    pub(crate) fn load(&self) -> ::anyhow::Result<Option<Vec<CachedBookmark>>> {
        let contents = match ::std::fs::read_to_string(&self.path) {
            Ok(contents) => contents,
            Err(e) if e.kind() == ::std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(::anyhow::anyhow!(e)),
        };
        let bookmarks = contents
            .lines()
            .map(CachedBookmark::parse)
            .collect::<::anyhow::Result<Vec<CachedBookmark>>>();
        match bookmarks {
            Ok(bookmarks) => Ok(Some(bookmarks)),
            Err(e) => {
                ::std::eprintln!(
                    "warning: cache file {} is corrupted ({e}); rebuilding from a full export",
                    self.path.display()
                );
                Ok(None)
            }
        }
    }

    /// temp ファイルに書いてから rename し、 atomic に置き換える。
    pub(crate) fn save(&self, bookmarks: &[CachedBookmark]) -> ::anyhow::Result<()> {
        if let Some(parent) = self.path.parent() {
            ::std::fs::create_dir_all(parent)?;
        }
        let mut contents = String::new();
        for bookmark in bookmarks {
            contents.push_str(bookmark.line());
            contents.push('\n');
        }
        let temp_path = self.path.with_extension("ndjson.tmp");
        ::std::fs::write(&temp_path, contents)?;
        ::std::fs::rename(&temp_path, &self.path)?;
        Ok(())
    }

    pub(crate) fn remove(&self) -> ::anyhow::Result<()> {
        match ::std::fs::remove_file(&self.path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == ::std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(::anyhow::anyhow!(e)),
        }
    }
}

fn resolve_cache_home(
    xdg_cache_home: Option<&str>,
    home: Option<&str>,
) -> ::anyhow::Result<::std::path::PathBuf> {
    if let Some(dir) = xdg_cache_home.filter(|s| !s.is_empty()) {
        return Ok(::std::path::PathBuf::from(dir));
    }
    let home = home
        .filter(|s| !s.is_empty())
        .ok_or_else(|| ::anyhow::anyhow!("neither XDG_CACHE_HOME nor HOME is set"))?;
    Ok(::std::path::PathBuf::from(home).join(".cache"))
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

    #[test]
    fn save_then_load_round_trips() -> ::anyhow::Result<()> {
        let dir = ::tempfile::tempdir()?;
        let cache = ExportCache::new(dir.path());
        let bookmarks = vec![CachedBookmark::for_test(), CachedBookmark::for_test()];
        cache.save(&bookmarks)?;
        assert_eq!(cache.load()?, Some(bookmarks));
        Ok(())
    }

    #[test]
    fn load_returns_none_when_file_is_missing() -> ::anyhow::Result<()> {
        let dir = ::tempfile::tempdir()?;
        let cache = ExportCache::new(dir.path());
        assert_eq!(cache.load()?, None);
        Ok(())
    }

    #[test]
    fn load_returns_none_when_file_is_corrupted() -> ::anyhow::Result<()> {
        let dir = ::tempfile::tempdir()?;
        let cache = ExportCache::new(dir.path());
        let path = dir.path().join(APP_DIR).join(CACHE_FILE);
        ::std::fs::create_dir_all(dir.path().join(APP_DIR))?;
        ::std::fs::write(&path, "not json\n")?;
        assert_eq!(cache.load()?, None);
        Ok(())
    }

    #[test]
    fn saves_lines_under_shiori_export_ndjson() -> ::anyhow::Result<()> {
        let dir = ::tempfile::tempdir()?;
        let cache = ExportCache::new(dir.path());
        let bookmarks = vec![CachedBookmark::for_test(), CachedBookmark::for_test()];
        cache.save(&bookmarks)?;
        let path = dir.path().join(APP_DIR).join(CACHE_FILE);
        let contents = ::std::fs::read_to_string(&path)?;
        assert_eq!(
            contents,
            format!("{}\n{}\n", bookmarks[0].line(), bookmarks[1].line())
        );
        Ok(())
    }

    #[test]
    fn save_replaces_existing_content_and_leaves_no_temp_file() -> ::anyhow::Result<()> {
        let dir = ::tempfile::tempdir()?;
        let cache = ExportCache::new(dir.path());
        cache.save(&[CachedBookmark::for_test(), CachedBookmark::for_test()])?;
        let bookmarks = vec![CachedBookmark::for_test()];
        cache.save(&bookmarks)?;
        assert_eq!(cache.load()?, Some(bookmarks));
        let entries = ::std::fs::read_dir(dir.path().join(APP_DIR))?
            .map(|e| Ok(e?.file_name().to_string_lossy().into_owned()))
            .collect::<::anyhow::Result<Vec<String>>>()?;
        assert_eq!(entries, vec![CACHE_FILE.to_string()]);
        Ok(())
    }

    #[test]
    fn remove_deletes_cache_file() -> ::anyhow::Result<()> {
        let dir = ::tempfile::tempdir()?;
        let cache = ExportCache::new(dir.path());
        cache.save(&[CachedBookmark::for_test()])?;
        cache.remove()?;
        assert_eq!(cache.load()?, None);
        Ok(())
    }

    #[test]
    fn remove_succeeds_when_file_is_missing() -> ::anyhow::Result<()> {
        let dir = ::tempfile::tempdir()?;
        let cache = ExportCache::new(dir.path());
        cache.remove()?;
        Ok(())
    }

    #[test]
    fn resolve_cache_home_prefers_xdg_cache_home() -> ::anyhow::Result<()> {
        let dir = resolve_cache_home(Some("/xdg/cache"), Some("/home/u"))?;
        assert_eq!(dir, ::std::path::PathBuf::from("/xdg/cache"));
        Ok(())
    }

    #[test]
    fn resolve_cache_home_falls_back_to_home_dot_cache() -> ::anyhow::Result<()> {
        let dir = resolve_cache_home(None, Some("/home/u"))?;
        assert_eq!(dir, ::std::path::PathBuf::from("/home/u/.cache"));
        Ok(())
    }

    #[test]
    fn resolve_cache_home_treats_empty_xdg_as_unset() -> ::anyhow::Result<()> {
        let dir = resolve_cache_home(Some(""), Some("/home/u"))?;
        assert_eq!(dir, ::std::path::PathBuf::from("/home/u/.cache"));
        Ok(())
    }

    #[test]
    fn resolve_cache_home_errors_without_xdg_or_home() {
        assert!(resolve_cache_home(None, None).is_err());
    }
}
