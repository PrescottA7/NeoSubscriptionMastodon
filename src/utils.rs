use base64::Engine;
use rand::RngCore;
use sha2::{Digest, Sha256};

/// Length of generated secrets: subscription tokens, session tokens, proxies' credentials.
pub const SECRET_BYTES: usize = 24;

/// Generates a URL-safe random secret, trimmed of padding.
pub fn random_secret() -> String {
    let mut bytes = [0u8; SECRET_BYTES];
    rand::thread_rng().fill_bytes(&mut bytes);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

/// Hashes a secret for at-rest storage. Tokens are compared by hash only.
pub fn hash_secret(secret: &str) -> String {
    let digest = Sha256::digest(secret.as_bytes());
    hex_lower(&digest)
}

pub fn hex_lower(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

/// Constant-time comparison to avoid leaking token prefixes through timing.
pub fn secure_eq(left: &str, right: &str) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.bytes()
        .zip(right.bytes())
        .fold(0u8, |acc, (a, b)| acc | (a ^ b))
        == 0
}

/// Percent-encodes a value the way subscriptions expect (`encodeURIComponent`).
pub fn url_encode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'!' | b'~'
            | b'*' | b'\'' | b'(' | b')' => out.push(*byte as char),
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

/// Normalises a user-supplied path fragment into `/segment/segment` form.
pub fn normalize_path(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let trimmed = trimmed.trim_matches('/');
    format!("/{trimmed}")
}

/// Bundles several path fragments, collapsing duplicate separators.
pub fn join_paths(parts: &[&str]) -> String {
    let mut out = String::new();
    for part in parts {
        let part = part.trim_matches('/');
        if part.is_empty() {
            continue;
        }
        out.push('/');
        out.push_str(part);
    }
    if out.is_empty() {
        "/".to_owned()
    } else {
        out
    }
}

/// Renders a node/user host the way share links expect (IPv6 wrapped in brackets).
pub fn format_host(host: &str) -> String {
    let host = host.trim();
    let bare = host.trim_start_matches('[').trim_end_matches(']');
    if bare.contains(':') {
        format!("[{bare}]")
    } else {
        host.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secrets_are_unique_and_url_safe() {
        let first = random_secret();
        let second = random_secret();
        assert_ne!(first, second);
        assert!(first.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'));
    }

    #[test]
    fn url_encode_escapes_reserved_characters() {
        assert_eq!(url_encode("a b/c?d=e&f"), "a%20b%2Fc%3Fd%3De%26f");
        assert_eq!(url_encode("safe-._~"), "safe-._~");
    }

    #[test]
    fn normalize_path_adds_leading_slash() {
        assert_eq!(normalize_path("s"), "/s");
        assert_eq!(normalize_path("/s/"), "/s");
        assert_eq!(normalize_path("api/v2"), "/api/v2");
        assert_eq!(normalize_path(""), "");
    }

    #[test]
    fn format_host_wraps_ipv6() {
        assert_eq!(format_host("example.com"), "example.com");
        assert_eq!(format_host("2001:db8::1"), "[2001:db8::1]");
        assert_eq!(format_host("[2001:db8::1]"), "[2001:db8::1]");
    }

    #[test]
    fn hash_secret_is_stable() {
        assert_eq!(hash_secret("abc"), hash_secret("abc"));
        assert!(secure_eq(&hash_secret("abc"), &hash_secret("abc")));
        assert!(!secure_eq(&hash_secret("abc"), &hash_secret("abd")));
    }
}
