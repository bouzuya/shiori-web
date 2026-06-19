/// bookmark を共有するための URL テンプレート。
///
/// `{{ ... }}` ブロックで bookmark の値を差し込む (前後の空白は許容):
/// - 変数: `url` / `title` / `comment`
/// - filter: `{{url}}` / `{{url|urlencode}}` … パーセントエンコード、`{{url|raw}}` … 生のまま
/// - 文字列リテラル: `{{ "{{" }}` のように `"..."` を書くと中身をそのまま出力する
///   (リテラルの波括弧を出したいときに使う)
///
/// `{{ ... }}` ブロック以外の裸の波括弧、未知の変数・filter、未対応の構文は不正とする。
/// また、ダミー値で展開した結果が有効な (絶対) URL であることを検証する。
///
/// テンプレートは最大 [`ShareUrl::MAX_LEN`] バイト。Firestore ドキュメントへ保存するため
/// 上限を明示する。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShareUrl {
    segments: Vec<Segment>,
    template: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Filter {
    Raw,
    UrlEncode,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Segment {
    Literal(String),
    Variable { filter: Filter, name: Variable },
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Variable {
    Comment,
    Title,
    Url,
}

impl ShareUrl {
    /// テンプレートの最大長 (バイト)。
    pub const MAX_LEN: usize = 2048;

    pub fn new(template: String) -> ::anyhow::Result<Self> {
        if template.len() > Self::MAX_LEN {
            ::anyhow::bail!(
                "ShareUrl too long: {} bytes (max {})",
                template.len(),
                Self::MAX_LEN
            );
        }
        let segments = Self::parse(&template)?;
        let share_url = Self { segments, template };
        // ダミー値で展開した結果が有効な (絶対) URL であることを検証する。
        let expanded = share_url.build("comment", "title", "https://example.com/");
        ::url::Url::parse(&expanded)?;
        Ok(share_url)
    }

    /// bookmark の各値でテンプレートを展開した URL を返す。
    pub fn build(&self, comment: &str, title: &str, url: &str) -> String {
        let mut out = String::new();
        for segment in &self.segments {
            match segment {
                Segment::Literal(s) => out.push_str(s),
                Segment::Variable { filter, name } => {
                    let value = match name {
                        Variable::Comment => comment,
                        Variable::Title => title,
                        Variable::Url => url,
                    };
                    match filter {
                        Filter::Raw => out.push_str(value),
                        Filter::UrlEncode => {
                            out.extend(::url::form_urlencoded::byte_serialize(value.as_bytes()))
                        }
                    }
                }
            }
        }
        out
    }

    fn parse(template: &str) -> ::anyhow::Result<Vec<Segment>> {
        let mut chars = template.chars().peekable();
        let mut segments = Vec::new();
        let mut literal = String::new();
        while let Some(c) = chars.next() {
            match c {
                '{' if chars.peek() == Some(&'{') => {
                    chars.next(); // 2つ目の `{` を消費する
                    if !literal.is_empty() {
                        segments.push(Segment::Literal(::std::mem::take(&mut literal)));
                    }
                    segments.push(Self::parse_block(&mut chars)?);
                }
                '{' | '}' => ::anyhow::bail!("invalid ShareUrl: unexpected brace"),
                _ => literal.push(c),
            }
        }
        if !literal.is_empty() {
            segments.push(Segment::Literal(literal));
        }
        Ok(segments)
    }

    /// `{{` の直後から1ブロックをパースする (閉じ `}}` まで消費する)。
    fn parse_block(
        chars: &mut ::std::iter::Peekable<::std::str::Chars<'_>>,
    ) -> ::anyhow::Result<Segment> {
        Self::skip_spaces(chars);
        if chars.peek() == Some(&'"') {
            // 文字列リテラル: 次の `"` までをそのまま出力する。
            chars.next(); // 開き `"` を消費する
            let mut literal = String::new();
            loop {
                match chars.next() {
                    Some('"') => break,
                    Some(c) => literal.push(c),
                    None => ::anyhow::bail!("invalid ShareUrl: unterminated string literal"),
                }
            }
            Self::skip_spaces(chars);
            Self::expect_close(chars)?;
            return Ok(Segment::Literal(literal));
        }

        let name = Self::take_ident(chars);
        Self::skip_spaces(chars);
        let filter = if chars.peek() == Some(&'|') {
            chars.next(); // `|` を消費する
            Self::skip_spaces(chars);
            let filter_name = Self::take_ident(chars);
            Self::skip_spaces(chars);
            match filter_name.as_str() {
                "raw" => Filter::Raw,
                "urlencode" => Filter::UrlEncode,
                _ => ::anyhow::bail!("invalid ShareUrl: unknown filter `{filter_name}`"),
            }
        } else {
            Filter::UrlEncode
        };
        Self::expect_close(chars)?;
        let name = match name.as_str() {
            "comment" => Variable::Comment,
            "title" => Variable::Title,
            "url" => Variable::Url,
            _ => ::anyhow::bail!("invalid ShareUrl: unknown variable `{name}`"),
        };
        Ok(Segment::Variable { filter, name })
    }

    /// 半角空白 (`' '`) のみを読み飛ばす (タブや全角空白などは許容しない)。
    fn skip_spaces(chars: &mut ::std::iter::Peekable<::std::str::Chars<'_>>) {
        while chars.peek() == Some(&' ') {
            chars.next();
        }
    }

    fn take_ident(chars: &mut ::std::iter::Peekable<::std::str::Chars<'_>>) -> String {
        let mut ident = String::new();
        while let Some(&c) = chars.peek() {
            if c.is_ascii_alphanumeric() || c == '_' {
                ident.push(c);
                chars.next();
            } else {
                break;
            }
        }
        ident
    }

    /// 閉じ `}}` を消費する。無ければエラー。
    fn expect_close(
        chars: &mut ::std::iter::Peekable<::std::str::Chars<'_>>,
    ) -> ::anyhow::Result<()> {
        if chars.next() == Some('}') && chars.next() == Some('}') {
            Ok(())
        } else {
            ::anyhow::bail!("invalid ShareUrl: expected `}}}}`");
        }
    }
}

impl ::std::fmt::Display for ShareUrl {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        write!(f, "{}", self.template)
    }
}

impl ::std::str::FromStr for ShareUrl {
    type Err = ::anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s.to_string())
    }
}

#[cfg(test)]
impl ShareUrl {
    pub fn for_test() -> Self {
        "https://example.com/share?url={{url}}&text={{title}}"
            .parse()
            .expect("valid ShareUrl")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_accepts_template_with_placeholders() -> ::anyhow::Result<()> {
        let s = "https://example.com/intent/tweet?url={{url}}&text={{title}}";
        assert_eq!(s.parse::<ShareUrl>()?.to_string(), s);
        Ok(())
    }

    #[test]
    fn test_new_rejects_relative_url() {
        assert!(ShareUrl::new("/relative?url={{url}}".to_string()).is_err());
    }

    #[test]
    fn test_new_rejects_invalid_url() {
        assert!(ShareUrl::new("not a url".to_string()).is_err());
        assert!(ShareUrl::new("".to_string()).is_err());
    }

    #[test]
    fn test_new_rejects_encoded_variable_in_scheme_position() {
        // 展開後は `{{url}}` がパーセントエンコードされ scheme を構成できないため弾く。
        assert!(ShareUrl::new("{{url}}?title={{title}}".to_string()).is_err());
    }

    #[test]
    fn test_new_rejects_invalid_syntax() {
        // 裸の波括弧・未知の変数・未知の filter・未閉鎖は弾く。
        assert!(ShareUrl::new("https://example.com/?u={{url}".to_string()).is_err());
        assert!(ShareUrl::new("https://example.com/?u={{{".to_string()).is_err());
        assert!(ShareUrl::new("https://example.com/?u={".to_string()).is_err());
        assert!(ShareUrl::new("https://example.com/?u=}".to_string()).is_err());
        assert!(ShareUrl::new("https://example.com/?u={{foo}}".to_string()).is_err());
        assert!(ShareUrl::new("https://example.com/?u={{foo}}}".to_string()).is_err());
        assert!(ShareUrl::new("https://example.com/?u={{url|bogus}}".to_string()).is_err());
    }

    #[test]
    fn test_build_encodes_by_default_and_with_urlencode_filter() -> ::anyhow::Result<()> {
        let share_url = "https://example.com/s?u={{url}}&t={{ title | urlencode }}&c={{comment}}"
            .parse::<ShareUrl>()?;
        let built = share_url.build("a & b", "Hello World", "https://example.com/?x=1");
        assert_eq!(
            built,
            "https://example.com/s?u=https%3A%2F%2Fexample.com%2F%3Fx%3D1&t=Hello+World&c=a+%26+b"
        );
        Ok(())
    }

    #[test]
    fn test_build_raw_filter_is_not_encoded() -> ::anyhow::Result<()> {
        // 生 URL をパスへ置くパターン (アーカイブ系サービス等) を表現できる。
        let share_url = "https://example.com/newest/{{url|raw}}".parse::<ShareUrl>()?;
        let built = share_url.build("c", "t", "https://example.com/?x=1");
        assert_eq!(built, "https://example.com/newest/https://example.com/?x=1");
        Ok(())
    }

    #[test]
    fn test_build_outputs_literal_braces() -> ::anyhow::Result<()> {
        let share_url = r#"https://example.com/?a={{ "{" }}&b={{"}"}}&c={{ "{{" }}&d={{ "}}" }}"#
            .parse::<ShareUrl>()?;
        let built = share_url.build("c", "t", "u");
        assert_eq!(built, "https://example.com/?a={&b=}&c={{&d=}}");
        Ok(())
    }

    #[test]
    fn test_new_rejects_non_space_whitespace_in_block() {
        // 半角空白以外の空白 (タブ・改行・全角空白) は許容しない。
        assert!(ShareUrl::new("https://example.com/?u={{\turl}}".to_string()).is_err());
        assert!(ShareUrl::new("https://example.com/?u={{url\n}}".to_string()).is_err());
        assert!(ShareUrl::new("https://example.com/?u={{\u{3000}url}}".to_string()).is_err());
    }

    #[test]
    fn test_build_tolerates_whitespace_in_block() -> ::anyhow::Result<()> {
        let spaced = "https://example.com/?u={{ url }}".parse::<ShareUrl>()?;
        let tight = "https://example.com/?u={{url}}".parse::<ShareUrl>()?;
        let args = ("c", "t", "https://example.com/");
        assert_eq!(
            spaced.build(args.0, args.1, args.2),
            tight.build(args.0, args.1, args.2)
        );
        Ok(())
    }

    #[test]
    fn test_new_accepts_template_at_max_len() -> ::anyhow::Result<()> {
        let prefix = "https://example.com/?q=";
        let template = format!("{prefix}{}", "a".repeat(ShareUrl::MAX_LEN - prefix.len()));
        assert_eq!(template.len(), ShareUrl::MAX_LEN);
        assert!(ShareUrl::new(template).is_ok());
        Ok(())
    }

    #[test]
    fn test_new_rejects_template_over_max_len() {
        let template = format!("https://example.com/?q={}", "a".repeat(ShareUrl::MAX_LEN));
        assert!(template.len() > ShareUrl::MAX_LEN);
        assert!(ShareUrl::new(template).is_err());
    }

    #[test]
    fn test_display_then_from_str_roundtrip() -> ::anyhow::Result<()> {
        let share_url = ShareUrl::for_test();
        assert_eq!(share_url.to_string().parse::<ShareUrl>()?, share_url);
        Ok(())
    }
}
