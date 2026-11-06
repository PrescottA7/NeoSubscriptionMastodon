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

import i18n from "./i18n";

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
    errors[field] = i18n.t("validation.required", { label });
  } else if (trimmed.length < min) {
    errors[field] = i18n.t("validation.minLength", { label, min });
  } else if (trimmed.length > max) {
    errors[field] = i18n.t("validation.maxLength", { label, max });
  }
}

/** Host or address: no scheme, no path, no brackets around IPv6. */
export function checkHost(errors: FieldErrors, field: string, value: string): void {
  const trimmed = value.trim();
  if (trimmed.length === 0) {
    errors[field] = i18n.t("validation.hostRequired");
  } else if (/\s/.test(trimmed)) {
    errors[field] = i18n.t("validation.hostNoSpaces");
  } else if (trimmed.includes("://") || trimmed.includes("/")) {
    errors[field] = i18n.t("validation.hostNoScheme");
  } else if (trimmed.startsWith("[") || trimmed.endsWith("]")) {
    errors[field] = i18n.t("validation.hostNoIpv6Brackets");
  }
}

/** Port as typed by the user: must be a whole number in range. */
export function checkPort(errors: FieldErrors, field: string, raw: string): void {
  const trimmed = raw.trim();
  if (trimmed.length === 0) {
    errors[field] = i18n.t("validation.portRequired");
    return;
  }
  const value = Number(trimmed);
  if (!Number.isInteger(value) || value < 1 || value > 65535) {
    errors[field] = i18n.t("validation.portRange");
  }
}

export function checkInteger(errors: FieldErrors, field: string, label: string, raw: string): void {
  const trimmed = raw.trim();
  if (trimmed.length === 0) {
    errors[field] = i18n.t("validation.required", { label });
    return;
  }
  if (!Number.isInteger(Number(trimmed))) {
    errors[field] = i18n.t("validation.integerRequired", { label });
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
    errors[field] = i18n.t("validation.durationFormat", { label });
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
      errors[field] = i18n.t("validation.portRangeInvalid", { value });
      return;
    }
    const start = Number(match[1]);
    const end = match[2] ? Number(match[2]) : start;
    if (start === 0 || end === 0 || start > end || end > 65535) {
      errors[field] = i18n.t("validation.portRangeOrder", { value });
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
    errors.remark = i18n.t("validation.remarkMaxLength");
  }

  const token = (form.token ?? "").trim();
  if (token.length > 0) {
    if (token.length > 64) {
      errors.token = i18n.t("validation.tokenMaxLength");
    } else if (!/^[A-Za-z0-9_-]+$/.test(token)) {
      errors.token = i18n.t("validation.tokenFormat");
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
      errors[`credentials.${index}.user_id`] = i18n.t("validation.credentialUserRequired");
      return;
    }
    if (seen.has(credential.user_id)) {
      errors[`credentials.${index}.user_id`] = i18n.t("validation.credentialDuplicate");
    }
    seen.add(credential.user_id);

    const uuid = credential.uuid?.trim() ?? "";
    if (uuid.length > 0 && !UUID_PATTERN.test(uuid)) {
      errors[`credentials.${index}.uuid`] = i18n.t("validation.uuidInvalid");
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
    errors.subscription_prefix = i18n.t("validation.subscriptionPrefixRequired");
  } else if (!prefix.startsWith("/")) {
    errors.subscription_prefix = i18n.t("validation.subscriptionPrefixSlash");
  } else if (prefix.length > 64) {
    errors.subscription_prefix = i18n.t("validation.subscriptionPrefixLength");
  } else if (/[\s?#\\]/.test(prefix)) {
    errors.subscription_prefix = i18n.t("validation.subscriptionPrefixChars");
  }

  checkText(errors, "site_name", i18n.t("validation.siteName"), form.site_name, 1, 32);

  const delay = form.failure_delay_ms.trim();
  if (delay.length > 0) {
    const value = Number(delay);
    if (!Number.isInteger(value) || value < 0 || value > 10_000) {
      errors.failure_delay_ms = i18n.t("validation.failureDelayRange");
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
      errors.disguise_template_url = i18n.t("validation.templateUrlInvalid");
    } else if (parsed.protocol !== "https:" || parsed.hostname.length === 0) {
      errors.disguise_template_url = i18n.t("validation.templateUrlScheme");
    }
  } else if (form.disguise_passthrough) {
    errors.disguise_template_url = i18n.t("validation.templateUrlRequired");
  }

  const ip = (form.disguise_template_ip ?? "").trim();
  if (ip.length > 0) {
    checkHost(errors, "disguise_template_ip", ip);
  }

  const port = (form.disguise_template_port ?? "").trim();
  if (port.length > 0) {
    const value = Number(port);
    if (!Number.isInteger(value) || value < 0 || value > 65_535) {
      errors.disguise_template_port = i18n.t("validation.templatePortRange");
    }
  }

  return errors;
}
