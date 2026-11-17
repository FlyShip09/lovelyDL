//! Removes secrets and personal paths from anything that is logged or copied out.

const REDACTED: &str = "[redacted]";

/// Query parameter names that carry credentials or signed tokens.
const SECRET_PARAMS: [&str; 10] =
    ["token", "sig", "signature", "key", "auth", "pass", "session", "secret", "cookie", "sid"];

/// Headers whose value is a credential.
const SECRET_HEADERS: [&str; 4] = ["authorization:", "cookie:", "set-cookie:", "x-goog-visitor-id:"];

/// yt-dlp options whose next argument is a secret.
const SECRET_FLAGS: [&str; 12] = [
    "--username",
    "-u",
    "--password",
    "-p",
    "--video-password",
    "--ap-username",
    "--ap-password",
    "--twofactor",
    "-2",
    "--add-headers",
    "--add-header",
    "--client-certificate-password",
];

/// Redacts one line of text. `home` is the user's home directory, replaced with `~`.
pub fn redact(text: &str, home: Option<&str>) -> String {
    let mut out = text.to_string();
    if let Some(home) = home.filter(|h| h.len() > 3) {
        // Paths show up plain and JSON-escaped (doubled backslashes), in any letter case.
        let escaped = home.replace('\\', "\\\\");
        out = replace_ignore_case(&replace_ignore_case(&out, &escaped, "~"), home, "~");
    }
    out = redact_headers(&out);
    out = redact_url_credentials(&out);
    redact_query_params(&out)
}

/// Redacts a command line: secret option values, then each argument as text.
pub fn redact_args(args: &[String], home: Option<&str>) -> Vec<String> {
    let mut out = Vec::with_capacity(args.len());
    let mut hide_next = false;
    for arg in args {
        if hide_next {
            out.push(REDACTED.to_string());
            hide_next = false;
            continue;
        }
        hide_next = SECRET_FLAGS.contains(&arg.as_str());
        // The value can also be glued on: --password=secret, -psecret.
        if let Some((flag, _)) = arg.split_once('=')
            && flag.starts_with("--")
            && SECRET_FLAGS.contains(&flag)
        {
            out.push(format!("{flag}={REDACTED}"));
        } else if let Some(flag) = ["-u", "-p", "-2"].into_iter().find(|f| arg.len() > 2 && arg.starts_with(f)) {
            out.push(format!("{flag}{REDACTED}"));
        } else {
            out.push(redact(arg, home));
        }
    }
    out
}

/// Windows paths match in any letter case. ASCII lower-casing keeps byte offsets intact.
fn replace_ignore_case(text: &str, from: &str, to: &str) -> String {
    let (hay, needle) = (text.to_ascii_lowercase(), from.to_ascii_lowercase());
    let mut out = String::with_capacity(text.len());
    let mut last = 0;
    for (at, _) in hay.match_indices(&needle) {
        out.push_str(&text[last..at]);
        out.push_str(to);
        last = at + needle.len();
    }
    out.push_str(&text[last..]);
    out
}

fn redact_headers(text: &str) -> String {
    let lower = text.to_ascii_lowercase();
    for header in SECRET_HEADERS {
        if let Some(at) = lower.find(header) {
            // Everything after the header name on this line is the secret.
            let value_start = at + header.len();
            let line_end = text[value_start..].find('\n').map_or(text.len(), |n| value_start + n);
            let mut out = String::with_capacity(text.len());
            out.push_str(&text[..value_start]);
            out.push(' ');
            out.push_str(REDACTED);
            out.push_str(&redact_headers(&text[line_end..]));
            return out;
        }
    }
    text.to_string()
}

/// `scheme://user:pass@host` becomes `scheme://[redacted]@host`.
fn redact_url_credentials(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find("://") {
        let (head, tail) = rest.split_at(at + 3);
        out.push_str(head);
        let authority_end = tail.find(['/', '?', '#', ' ', '"', '\'']).unwrap_or(tail.len());
        match tail[..authority_end].rfind('@') {
            Some(user_end) => {
                out.push_str(REDACTED);
                rest = &tail[user_end..];
            }
            None => rest = tail,
        }
    }
    out.push_str(rest);
    out
}

/// `?token=abc&x=1` becomes `?token=[redacted]&x=1`.
fn redact_query_params(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find(['?', '&']) {
        let (head, tail) = rest.split_at(at + 1);
        out.push_str(head);
        let name_end = tail.find(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '-'));
        let Some(name_end) = name_end.filter(|&n| tail[n..].starts_with('=')) else {
            rest = tail;
            continue;
        };
        let name = tail[..name_end].to_ascii_lowercase();
        let value_start = name_end + 1;
        let value_end =
            tail[value_start..].find(['&', ' ', '"', '\'', '#', ')', '\n']).map_or(tail.len(), |n| value_start + n);
        out.push_str(&tail[..value_start]);
        if SECRET_PARAMS.iter().any(|s| name.contains(s)) && value_end > value_start {
            out.push_str(REDACTED);
        } else {
            out.push_str(&tail[value_start..value_end]);
        }
        rest = &tail[value_end..];
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn headers_and_cookies() {
        let line = "[debug] Sending Cookie: SID=abc123; HSID=zzz";
        assert_eq!(redact(line, None), "[debug] Sending Cookie: [redacted]");
        let two = "Authorization: Bearer xyz\nnext line";
        assert_eq!(redact(two, None), "Authorization: [redacted]\nnext line");
    }

    #[test]
    fn url_credentials_and_tokens() {
        assert_eq!(
            redact("--proxy socks5://bob:hunter2@10.0.0.1:1080/", None),
            "--proxy socks5://[redacted]@10.0.0.1:1080/"
        );
        assert_eq!(
            redact("GET https://a.test/v.mp4?expire=1&sig=AbC-_9&itag=18 done", None),
            "GET https://a.test/v.mp4?expire=1&sig=[redacted]&itag=18 done"
        );
        assert_eq!(redact("https://a.test/?access_token=q", None), "https://a.test/?access_token=[redacted]");
    }

    #[test]
    fn home_directory_both_spellings() {
        let home = r"C:\Users\someone";
        assert_eq!(redact(r"C:\Users\someone\Downloads\a.mp4", Some(home)), r"~\Downloads\a.mp4");
        assert_eq!(redact(r#""C:\\Users\\someone\\a.mp4""#, Some(home)), r#""~\\a.mp4""#);
        assert_eq!(redact(r"c:\users\SOMEONE\a.mp4", Some(home)), r"~\a.mp4");
    }

    #[test]
    fn plain_text_is_untouched() {
        for s in ["ERROR: [youtube] abc: This video is unavailable", "What? No & yes", "a=b&c"] {
            assert_eq!(redact(s, None), s);
        }
    }

    #[test]
    fn secret_flag_values() {
        let args: Vec<String> = ["--password", "hunter2", "--add-headers", "X: y", "--", "https://a.test/?key=1"]
            .map(String::from)
            .to_vec();
        assert_eq!(
            redact_args(&args, None),
            ["--password", "[redacted]", "--add-headers", "[redacted]", "--", "https://a.test/?key=[redacted]"]
        );
        let glued: Vec<String> = ["--password=hunter2", "-phunter2", "--video-password=x", "-o", "%(title)s.%(ext)s"]
            .map(String::from)
            .to_vec();
        assert_eq!(
            redact_args(&glued, None),
            ["--password=[redacted]", "-p[redacted]", "--video-password=[redacted]", "-o", "%(title)s.%(ext)s"]
        );
    }
}
