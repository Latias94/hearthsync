use crate::core::error::{AppError, AppResult};

pub(in crate::core) fn is_http_url(value: &str) -> bool {
    value.starts_with("http://") || value.starts_with("https://")
}

pub(in crate::core) fn validate_http_url(value: &str, field: &str) -> AppResult<()> {
    if value.trim().is_empty() {
        return Err(AppError::Validation(format!("{field} must not be empty")));
    }
    if value.trim() != value {
        return Err(AppError::Validation(format!(
            "{field} must not have surrounding whitespace"
        )));
    }
    if value.chars().any(|char| char.is_control()) {
        return Err(AppError::Validation(format!(
            "{field} must not contain control characters"
        )));
    }
    if !is_http_url(value) {
        return Err(AppError::Validation(format!(
            "{field} must start with `http://` or `https://`"
        )));
    }
    let parsed = reqwest::Url::parse(value).map_err(|_| {
        AppError::Validation(format!("{field} must be a valid absolute HTTP(S) URL"))
    })?;
    if parsed.host_str().is_none() {
        return Err(AppError::Validation(format!("{field} must include a host")));
    }

    Ok(())
}

pub(in crate::core) fn is_rfc3339_timestamp_shape(value: &str) -> bool {
    if value.trim() != value || value.is_empty() {
        return false;
    }
    let Some((date, time)) = value.split_once('T') else {
        return false;
    };

    is_rfc3339_date(date) && is_rfc3339_time(time)
}

fn is_rfc3339_date(value: &str) -> bool {
    value.len() == 10
        && value.as_bytes().get(4) == Some(&b'-')
        && value.as_bytes().get(7) == Some(&b'-')
        && value
            .chars()
            .enumerate()
            .all(|(index, char)| matches!(index, 4 | 7) || char.is_ascii_digit())
}

fn is_rfc3339_time(value: &str) -> bool {
    let Some((time, zone)) = split_rfc3339_zone(value) else {
        return false;
    };
    let (time, fraction) = time
        .split_once('.')
        .map_or((time, None), |(time, fraction)| (time, Some(fraction)));

    time.len() == 8
        && time.as_bytes().get(2) == Some(&b':')
        && time.as_bytes().get(5) == Some(&b':')
        && time
            .chars()
            .enumerate()
            .all(|(index, char)| matches!(index, 2 | 5) || char.is_ascii_digit())
        && fraction.is_none_or(|fraction| {
            !fraction.is_empty() && fraction.chars().all(|char| char.is_ascii_digit())
        })
        && is_rfc3339_zone(zone)
}

pub(in crate::core) fn is_hex_digest(value: &str, expected_len: usize) -> bool {
    value.len() == expected_len && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

pub(in crate::core) fn is_sha256_hex(value: &str) -> bool {
    is_hex_digest(value, 64)
}

pub(in crate::core) fn validate_hex_digest(
    value: &str,
    field: &str,
    expected_len: usize,
    algorithm: &str,
) -> AppResult<()> {
    if !is_hex_digest(value, expected_len) {
        return Err(AppError::Validation(format!(
            "{field} must be a {expected_len}-character {algorithm} hex digest"
        )));
    }

    Ok(())
}

fn split_rfc3339_zone(value: &str) -> Option<(&str, &str)> {
    if let Some(time) = value.strip_suffix('Z') {
        return Some((time, "Z"));
    }
    value
        .rfind(['+', '-'])
        .map(|zone_start| value.split_at(zone_start))
}

fn is_rfc3339_zone(value: &str) -> bool {
    if value == "Z" {
        return true;
    }
    let Some(rest) = value.strip_prefix(['+', '-']) else {
        return false;
    };

    rest.len() == 5
        && rest.as_bytes().get(2) == Some(&b':')
        && rest
            .chars()
            .enumerate()
            .all(|(index, char)| index == 2 || char.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::{
        is_hex_digest, is_http_url, is_rfc3339_timestamp_shape, is_sha256_hex, validate_hex_digest,
        validate_http_url,
    };

    #[test]
    fn is_http_url_accepts_supported_schemes() {
        assert!(is_http_url("http://example.com/addon.zip"));
        assert!(is_http_url("https://example.com/addon.zip"));
        assert!(!is_http_url("ftp://example.com/addon.zip"));
        assert!(!is_http_url(" https://example.com/addon.zip"));
    }

    #[test]
    fn validate_http_url_reports_blank_and_wrong_scheme() {
        let blank = validate_http_url(" ", "download URL").expect_err("blank url");
        assert!(blank.to_string().contains("download URL must not be empty"));

        let wrong_scheme =
            validate_http_url("ftp://example.com/addon.zip", "download URL").expect_err("scheme");
        assert!(
            wrong_scheme
                .to_string()
                .contains("download URL must start with")
        );
    }

    #[test]
    fn validate_http_url_rejects_ambiguous_url_shapes() {
        for (value, expected_message) in [
            (
                "https://example.com/addon.zip ",
                "must not have surrounding whitespace",
            ),
            ("https://", "must be a valid absolute HTTP(S) URL"),
            (
                "https://example.com/addon\n.zip",
                "must not contain control characters",
            ),
        ] {
            let error = validate_http_url(value, "download URL").expect_err("invalid url");
            assert!(error.to_string().contains(expected_message), "{value}");
        }
    }

    #[test]
    fn is_rfc3339_timestamp_shape_accepts_basic_fractional_and_offset_times() {
        assert!(is_rfc3339_timestamp_shape("2026-04-29T10:00:00Z"));
        assert!(is_rfc3339_timestamp_shape("2026-04-29T10:00:00.123Z"));
        assert!(is_rfc3339_timestamp_shape("2026-04-29T10:00:00+08:00"));
        assert!(is_rfc3339_timestamp_shape("2026-04-29T10:00:00.123-07:00"));
    }

    #[test]
    fn is_rfc3339_timestamp_shape_rejects_ambiguous_or_padded_values() {
        for value in [
            "",
            " 2026-04-29T10:00:00Z",
            "2026-04-29T10:00:00Z ",
            "2026-04-29 10:00:00Z",
            "2026-04-29T10:00Z",
            "2026-04-29T10:00:00",
            "2026-04-29T10:00:00.",
            "2026-04-29T10:00:00+0800",
        ] {
            assert!(!is_rfc3339_timestamp_shape(value), "{value}");
        }
    }

    #[test]
    fn hex_digest_helpers_validate_exact_ascii_hex_shapes() {
        assert!(is_hex_digest("0123456789abcdefABCDEF", 22));
        assert!(is_sha256_hex(
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
        ));

        for value in [
            "",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaag",
            " aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        ] {
            assert!(!is_sha256_hex(value), "{value}");
        }

        let error =
            validate_hex_digest("abc", "asset digest", 64, "SHA-256").expect_err("short digest");
        assert!(
            error
                .to_string()
                .contains("asset digest must be a 64-character SHA-256 hex digest")
        );
    }
}
