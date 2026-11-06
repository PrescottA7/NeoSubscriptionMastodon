// Copyright (C) 2026 Prescott Anderson
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// This file is part of NSM (NeoSubscriptionMastodon), licensed under the
// GNU Affero General Public License v3.0 or later. See the LICENSE file
// in the project root for the full text.

//! Input validation shared by the API handlers.
//!
//! Rules live here (not in serde) so a failure can name the exact field and
//! return a message a user can act on, instead of a bare deserialisation error.

use crate::error::FieldError;

/// Collects field errors so one response can report every problem at once.
#[derive(Default)]
pub struct Validator {
    errors: Vec<FieldError>,
}

impl Validator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, field: impl Into<String>, message: impl Into<String>) {
        self.errors.push(FieldError::new(field, message));
    }

    /// Records an error when `condition` is false.
    pub fn require(&mut self, condition: bool, field: impl Into<String>, message: impl Into<String>) {
        if !condition {
            self.push(field, message);
        }
    }

    /// Required trimmed text field with a length bound.
    pub fn text(
        &mut self,
        field: &str,
        label: &str,
        value: &str,
        min: usize,
        max: usize,
    ) -> &mut Self {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            self.push(field, format!("{label}不能为空"));
        } else if trimmed.chars().count() < min {
            self.push(field, format!("{label}至少 {min} 个字符"));
        } else if trimmed.chars().count() > max {
            self.push(field, format!("{label}最多 {max} 个字符"));
        }
        self
    }

    /// Optional text field with an upper bound; empty is allowed.
    pub fn optional_text(
        &mut self,
        field: &str,
        label: &str,
        value: Option<&str>,
        max: usize,
    ) -> &mut Self {
        if let Some(value) = value
            && !value.is_empty()
            && value.chars().count() > max
        {
            self.push(field, format!("{label}最多 {max} 个字符"));
        }
        self
    }

    /// Host or address: non-empty, no whitespace, no scheme or slash.
    pub fn host(&mut self, field: &str, value: &str) -> &mut Self {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            self.push(field, "服务器地址不能为空");
        } else if trimmed.chars().any(char::is_whitespace) {
            self.push(field, "服务器地址不能包含空格");
        } else if trimmed.contains("://") || trimmed.contains('/') {
            self.push(field, "服务器地址只填主机名或 IP，不要带协议或路径");
        } else if trimmed.starts_with('[') || trimmed.ends_with(']') {
            self.push(field, "IPv6 地址不需要方括号，例如 2001:db8::1");
        }
        self
    }

    pub fn port(&mut self, field: &str, value: u16) -> &mut Self {
        if value == 0 {
            self.push(field, "端口必须在 1-65535 之间");
        }
        self
    }

    /// Duration strings such as `30s`, `1m`, `1500ms`.
    pub fn duration(&mut self, field: &str, label: &str, value: Option<&str>) -> &mut Self {
        let Some(value) = value.filter(|value| !value.is_empty()) else {
            return self;
        };
        let has_unit = ["ms", "s", "m", "h"]
            .iter()
            .any(|unit| value.ends_with(unit) && value.len() > unit.len());
        let numeric = value
            .trim_end_matches(|c: char| c.is_ascii_alphabetic())
            .parse::<f64>();
        if !has_unit || numeric.map(|n| n <= 0.0).unwrap_or(true) {
            self.push(field, format!("{label}格式应为数字加单位，例如 30s"));
        }
        self
    }

    /// Port ranges as accepted by Hysteria2, e.g. `2080:3000` or `443`.
    pub fn port_ranges(&mut self, field: &str, values: &[String]) -> &mut Self {
        for value in values {
            if crate::protocol::kinds::hysteria2::parse_port_range(value).is_none() {
                self.push(
                    field,
                    format!("端口范围 “{value}” 不合法，应形如 2080:3000 或 443"),
                );
            }
        }
        self
    }

    pub fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn finish(self) -> Result<(), crate::error::RequestError> {
        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(crate::error::RequestError::validation(self.errors))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_text_is_rejected_with_the_label() {
        let mut validator = Validator::new();
        validator.text("name", "节点名称", "   ", 1, 64);
        let errors = validator.errors;
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].field, "name");
        assert!(errors[0].message.contains("节点名称不能为空"));
    }

    #[test]
    fn text_length_bounds_are_enforced() {
        let mut validator = Validator::new();
        validator.text("username", "用户名", "ab", 3, 10);
        assert!(validator.errors[0].message.contains("至少 3"));
    }

    #[test]
    fn host_rejects_scheme_and_brackets() {
        for bad in ["https://a.com", "a.com/x", "a b.com", "[::1]"] {
            let mut validator = Validator::new();
            validator.host("server", bad);
            assert!(!validator.is_empty(), "{bad} should be rejected");
        }
    }

    #[test]
    fn host_accepts_domains_and_bare_ipv6() {
        for good in ["a.com", "1.2.3.4", "2001:db8::1"] {
            let mut validator = Validator::new();
            validator.host("server", good);
            assert!(validator.is_empty(), "{good} should pass");
        }
    }

    #[test]
    fn port_zero_is_rejected() {
        let mut validator = Validator::new();
        validator.port("port", 0);
        assert!(!validator.is_empty());
    }

    #[test]
    fn durations_need_a_unit() {
        let mut validator = Validator::new();
        validator.duration("hop", "跳跃间隔", Some("30"));
        assert!(!validator.is_empty());

        let mut validator = Validator::new();
        validator.duration("hop", "跳跃间隔", Some("30s"));
        assert!(validator.is_empty());
    }

    #[test]
    fn blank_optional_duration_passes() {
        let mut validator = Validator::new();
        validator.duration("hop", "跳跃间隔", Some(""));
        validator.duration("hop", "跳跃间隔", None);
        assert!(validator.is_empty());
    }

    #[test]
    fn invalid_port_ranges_are_reported() {
        let mut validator = Validator::new();
        validator.port_ranges("ports", &["2080:3000".to_owned(), "oops".to_owned()]);
        assert_eq!(validator.errors.len(), 1);
        assert!(validator.errors[0].message.contains("oops"));
    }

    #[test]
    fn finish_returns_422_with_all_errors() {
        let mut validator = Validator::new();
        validator.text("name", "节点名称", "", 1, 64);
        validator.port("port", 0);
        let error = validator.finish().expect_err("should fail");
        let response = error.into_response();
        assert_eq!(response.status(), axum::http::StatusCode::UNPROCESSABLE_ENTITY);
    }
}
