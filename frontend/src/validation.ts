/*
 * Copyright (C) 2026 Prescott Anderson
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * This file is part of NSM (NeoSubscriptionMastodon), licensed under the
 * GNU Affero General Public License v3.0 or later. See the LICENSE file
 * in the project root for the full text.
 */

/**
 * Client-side mirror of the server-side validation rules in `src/validate.rs`.
 *
 * Running the same checks before the request means an obvious mistake is
 * reported instantly against the right input, and the server-side rules remain
 * the source of truth for anything the client cannot know.
 */

export type FieldErrors = Record<string, string>;

const UUID_PATTERN =
  /^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$/;

export function isEmpty(errors: FieldErrors): boolean {
  return Object.keys(errors).length === 0;
}

/** Required text with a length bound. */
export function checkText(
  errors: FieldErrors,
  field: string,
  label: string,
  value: string,
  min: number,
  max: number,
): void {
  const trimmed = value.trim();
  if (trimmed.length === 0) {
    errors[field] = `${label}不能为空`;
  } else if (trimmed.length < min) {
    errors[field] = `${label}至少 ${min} 个字符`;
  } else if (trimmed.length > max) {
    errors[field] = `${label}最多 ${max} 个字符`;
  }
}

/** Host or address: no scheme, no path, no brackets around IPv6. */
export function checkHost(errors: FieldErrors, field: string, value: string): void {
  const trimmed = value.trim();
  if (trimmed.length === 0) {
    errors[field] = "服务器地址不能为空";
  } else if (/\s/.test(trimmed)) {
    errors[field] = "服务器地址不能包含空格";
  } else if (trimmed.includes("://") || trimmed.includes("/")) {
    errors[field] = "服务器地址只填主机名或 IP，不要带协议或路径";
  } else if (trimmed.startsWith("[") || trimmed.endsWith("]")) {
    errors[field] = "IPv6 地址不需要方括号，例如 2001:db8::1";
  }
}

/** Port as typed by the user: must be a whole number in range. */
export function checkPort(errors: FieldErrors, field: string, raw: string): void {
  const trimmed = raw.trim();
  if (trimmed.length === 0) {
    errors[field] = "端口不能为空";
    return;
  }
  const value = Number(trimmed);
  if (!Number.isInteger(value) || value < 1 || value > 65535) {
    errors[field] = "端口必须是 1-65535 之间的整数";
  }
}

export function checkInteger(errors: FieldErrors, field: string, label: string, raw: string): void {
  const trimmed = raw.trim();
  if (trimmed.length === 0) {
    errors[field] = `${label}不能为空`;
    return;
  }
  if (!Number.isInteger(Number(trimmed))) {
    errors[field] = `${label}必须是整数`;
  }
}

/** Durations such as `30s`, `1m`, `1500ms`; empty is allowed. */
export function checkDuration(
  errors: FieldErrors,
  field: string,
  label: string,
  value: string | null | undefined,
): void {
  if (!value) {
    return;
  }
  const trimmed = value.trim();
  if (trimmed.length === 0) {
    return;
  }
  const match = /^(\d+(?:\.\d+)?)(ms|s|m|h)$/.exec(trimmed);
  if (!match || Number(match[1]) <= 0) {
    errors[field] = `${label}格式应为数字加单位，例如 30s`;
  }
}

/** Port ranges for Hysteria2, e.g. `2080:3000` or `443`. */
export function checkPortRanges(
  errors: FieldErrors,
  field: string,
  values: string[],
): void {
  for (const value of values) {
    const match = /^(\d+)(?::(\d+))?$/.exec(value.trim());
    if (!match) {
      errors[field] = `端口范围 “${value}” 不合法，应形如 2080:3000 或 443`;
      return;
    }
    const start = Number(match[1]);
    const end = match[2] ? Number(match[2]) : start;
    if (start === 0 || end === 0 || start > end || end > 65535) {
      errors[field] = `端口范围 “${value}” 不合法，起始端口不能大于结束端口`;
      return;
    }
  }
}

/** Applies server-provided field errors, without clobbering local ones. */
export function mergeServerErrors(errors: FieldErrors, server: FieldErrors): FieldErrors {
  const merged = { ...errors };
  for (const [field, message] of Object.entries(server)) {
    if (!merged[field]) {
      merged[field] = message;
    }
  }
  return merged;
}

/** Mirrors `validate_new_user` / `validate_user_update` on the server. */
export function validateUserForm(form: { remark: string; token?: string }): FieldErrors {
  const errors: FieldErrors = {};
  if (form.remark.trim().length > 64) {
    errors.remark = "备注最多 64 个字符";
  }

  const token = (form.token ?? "").trim();
  if (token.length > 0) {
    if (token.length > 64) {
      errors.token = "订阅路径最多 64 个字符";
    } else if (!/^[A-Za-z0-9_-]+$/.test(token)) {
      errors.token = "订阅路径只能包含字母、数字、- 和 _";
    }
  }

  return errors;
}

/**
 * Mirrors `validate` in `src/routes/nodes.rs` for the credential section.
 *
 * The server never accepts two rows for the same user on one node, and a
 * hand-typed uuid has to be a real UUID.
 */
export function validateCredentials(
  errors: FieldErrors,
  credentials: { user_id: number; uuid?: string | null }[],
): void {
  const seen = new Set<number>();
  credentials.forEach((credential, index) => {
    if (!credential.user_id) {
      errors[`credentials.${index}.user_id`] = "请选择凭据所属的用户";
      return;
    }
    if (seen.has(credential.user_id)) {
      errors[`credentials.${index}.user_id`] = "同一用户在该节点上只能有一组凭据";
    }
    seen.add(credential.user_id);

    const uuid = credential.uuid?.trim() ?? "";
    if (uuid.length > 0 && !UUID_PATTERN.test(uuid)) {
      errors[`credentials.${index}.uuid`] =
        "UUID 格式不正确，例如 5783a3e7-e373-51cd-8642-c83782b807c5";
    }
  });
}

/** Mirrors `validate_settings` on the server. */
export function validateSettingsForm(form: {
  subscription_prefix: string;
  default_target: string;
  site_name: string;
  failure_delay_ms: string;
  disguise_passthrough?: boolean;
  disguise_template_url?: string;
  disguise_template_ip?: string;
  disguise_template_port?: string;
}): FieldErrors {
  const errors: FieldErrors = {};

  const prefix = form.subscription_prefix.trim();
  if (prefix.length === 0) {
    errors.subscription_prefix = "订阅路径前缀不能为空";
  } else if (!prefix.startsWith("/")) {
    errors.subscription_prefix = "订阅路径前缀必须以 / 开头";
  } else if (prefix.length > 64) {
    errors.subscription_prefix = "订阅路径前缀最多 64 个字符";
  } else if (/[\s?#\\]/.test(prefix)) {
    errors.subscription_prefix = "订阅路径前缀不能包含空格、? 或 #";
  }

  checkText(errors, "site_name", "站点名称", form.site_name, 1, 32);

  const delay = form.failure_delay_ms.trim();
  if (delay.length > 0) {
    const value = Number(delay);
    if (!Number.isInteger(value) || value < 0 || value > 10_000) {
      errors.failure_delay_ms = "失败延迟必须是 0-10000 之间的整数";
    }
  }

  const templateUrl = (form.disguise_template_url ?? "").trim();
  if (templateUrl.length > 0) {
    let parsed: URL | null = null;
    try {
      parsed = new URL(templateUrl);
    } catch {
      parsed = null;
    }
    if (!parsed) {
      errors.disguise_template_url = "样板页地址不是合法的 URL";
    } else if (parsed.protocol !== "https:" || parsed.hostname.length === 0) {
      errors.disguise_template_url = "样板页地址必须是 https:// 开头并包含域名";
    }
  } else if (form.disguise_passthrough) {
    errors.disguise_template_url = "开启 404 透传前请先填写样板页地址";
  }

  const ip = (form.disguise_template_ip ?? "").trim();
  if (ip.length > 0) {
    checkHost(errors, "disguise_template_ip", ip);
  }

  const port = (form.disguise_template_port ?? "").trim();
  if (port.length > 0) {
    const value = Number(port);
    if (!Number.isInteger(value) || value < 0 || value > 65_535) {
      errors.disguise_template_port = "服务器端口必须在 0-65535 之间";
    }
  }

  return errors;
}
