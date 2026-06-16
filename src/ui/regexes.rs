use crate::strings as s;

/// Compiled regex patterns for URL and email matching.
pub struct MatchRegexes {
    pub http_regex: vte4::Regex,
    pub email_regex: vte4::Regex,
}

impl MatchRegexes {
    pub fn compile() -> Self {
        let http_regex = vte4::Regex::for_match(
            r#"(ftp|http)s?://[^ \t\n\b()<>{}«»\[\]'"]+[^.]"#,
            0x00000008, // PCRE2_CASELESS
        )
        .expect(s::ERROR_COMPILE_HTTP_REGEX);

        let email_regex = vte4::Regex::for_match(
            r"[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,24}",
            0x00000008, // PCRE2_CASELESS
        )
        .expect(s::ERROR_COMPILE_EMAIL_REGEX);

        Self {
            http_regex,
            email_regex,
        }
    }
}
