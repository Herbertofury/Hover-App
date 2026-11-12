use std::io::{self, Read, Write};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeMessage {
    Hover { url: String },
    Exit,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UrlValidationError {
    Empty,
    UnsupportedScheme,
    ContainsCredentials,
}

pub fn validate_remote_url(url: &str) -> Result<(), UrlValidationError> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return Err(UrlValidationError::Empty);
    }
    let lower = trimmed.to_ascii_lowercase();
    if !(lower.starts_with("https://") || lower.starts_with("http://")) {
        return Err(UrlValidationError::UnsupportedScheme);
    }
    let after_scheme = lower
        .split_once("://")
        .map(|(_, rest)| rest)
        .unwrap_or_default();
    let host_part = after_scheme.split('/').next().unwrap_or_default();
    if host_part.contains('@') {
        return Err(UrlValidationError::ContainsCredentials);
    }
    Ok(())
}

pub fn parse_message(payload: &str) -> NativeMessage {
    if payload.contains("\"type\":\"exit\"") || payload.contains("\"type\": \"exit\"") {
        return NativeMessage::Exit;
    }
    if !(payload.contains("\"type\":\"hover\"") || payload.contains("\"type\": \"hover\"")) {
        return NativeMessage::Unknown;
    }
    if let Some(url) = extract_json_string(payload, "url") {
        NativeMessage::Hover { url }
    } else {
        NativeMessage::Unknown
    }
}

pub fn read_native_message<R: Read>(mut reader: R) -> io::Result<Option<NativeMessage>> {
    let mut len_bytes = [0u8; 4];
    match reader.read_exact(&mut len_bytes) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(error) => return Err(error),
    }
    let len = u32::from_le_bytes(len_bytes) as usize;
    if len == 0 || len > 1024 * 1024 {
        return Ok(Some(NativeMessage::Unknown));
    }
    let mut payload = vec![0u8; len];
    reader.read_exact(&mut payload)?;
    let text = String::from_utf8_lossy(&payload);
    Ok(Some(parse_message(&text)))
}

pub fn write_native_message<W: Write>(mut writer: W, payload: &[u8]) -> io::Result<()> {
    let len = payload.len() as u32;
    writer.write_all(&len.to_le_bytes())?;
    writer.write_all(payload)
}

fn extract_json_string(payload: &str, key: &str) -> Option<String> {
    let needle = format!("\"{}\"", key);
    let start = payload.find(&needle)? + needle.len();
    let after_key = &payload[start..];
    let colon = after_key.find(':')?;
    let after_colon = after_key[colon + 1..].trim_start();
    let rest = after_colon.strip_prefix('"')?;
    let mut value = String::new();
    let mut escaped = false;
    for ch in rest.chars() {
        if escaped {
            value.push(ch);
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if ch == '"' {
            return Some(value);
        } else {
            value.push(ch);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_only_http_and_https_without_credentials() {
        assert!(validate_remote_url("https://example.com/a.png").is_ok());
        assert!(validate_remote_url("http://example.com/a.png").is_ok());
        assert_eq!(
            validate_remote_url("file:///c:/a.png"),
            Err(UrlValidationError::UnsupportedScheme)
        );
        assert_eq!(
            validate_remote_url("https://user:pass@example.com/a.png"),
            Err(UrlValidationError::ContainsCredentials)
        );
    }

    #[test]
    fn parses_hover_message() {
        let message = parse_message(r#"{"type":"hover","url":"https://example.com/a.png"}"#);
        assert_eq!(
            message,
            NativeMessage::Hover {
                url: "https://example.com/a.png".to_string()
            }
        );
    }
}
