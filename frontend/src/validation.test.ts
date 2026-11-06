/*
 * Copyright (C) 2026 Prescott Anderson
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * This file is part of NSM (NeoSubscriptionMastodon), licensed under the
 * GNU Affero General Public License v3.0 or later. See the LICENSE file
 * in the project root for the full text.
 */

import { describe, expect, it } from "vitest";

import {
  checkDuration,
  checkHost,
  checkInteger,
  checkPort,
  checkPortRanges,
  checkText,
  isEmpty,
  mergeServerErrors,
  validateCredentials,
  validateSettingsForm,
  validateUserForm,
  type FieldErrors,
} from "./validation";

describe("checkText", () => {
  it("rejects blank values and names the field", () => {
    const errors: FieldErrors = {};
    checkText(errors, "name", "Node name", "   ", 1, 64);
    expect(errors.name).toBe("Node name cannot be empty");
  });

  it("enforces both bounds", () => {
    const tooShort: FieldErrors = {};
    checkText(tooShort, "username", "Username", "ab", 3, 10);
    expect(tooShort.username).toBe("Username must be at least 3 characters");

    const tooLong: FieldErrors = {};
    checkText(tooLong, "username", "Username", "a".repeat(11), 3, 10);
    expect(tooLong.username).toBe("Username must be at most 10 characters");
  });

  it("accepts a value inside the bounds", () => {
    const errors: FieldErrors = {};
    checkText(errors, "name", "Node name", "Hong Kong 01", 1, 64);
    expect(isEmpty(errors)).toBe(true);
  });
});

describe("checkHost", () => {
  it("rejects schemes, paths, spaces and bracketed IPv6", () => {
    for (const value of ["https://a.com", "a.com/x", "a b.com", "[::1]"]) {
      const errors: FieldErrors = {};
      checkHost(errors, "server", value);
      expect(errors.server, value).toBeTruthy();
    }
  });

  it("accepts domains, IPv4 and bare IPv6", () => {
    for (const value of ["a.com", "1.2.3.4", "2001:db8::1"]) {
      const errors: FieldErrors = {};
      checkHost(errors, "server", value);
      expect(isEmpty(errors), value).toBe(true);
    }
  });
});

describe("checkPort", () => {
  it("rejects empty, fractional and out-of-range values", () => {
    for (const value of ["", "1.5", "0", "65536", "abc"]) {
      const errors: FieldErrors = {};
      checkPort(errors, "port", value);
      expect(errors.port, value).toBeTruthy();
    }
  });

  it("accepts a valid port", () => {
    const errors: FieldErrors = {};
    checkPort(errors, "port", "443");
    expect(isEmpty(errors)).toBe(true);
  });
});

describe("checkInteger", () => {
  it("rejects blank and fractional input", () => {
    const blank: FieldErrors = {};
    checkInteger(blank, "sort_order", "Sort order", " ");
    expect(blank.sort_order).toBe("Sort order cannot be empty");

    const fractional: FieldErrors = {};
    checkInteger(fractional, "sort_order", "Sort order", "1.2");
    expect(fractional.sort_order).toBe("Sort order must be an integer");
  });
});

describe("checkDuration", () => {
  it("requires a unit", () => {
    const errors: FieldErrors = {};
    checkDuration(errors, "hop_interval", "Hop interval", "30");
    expect(errors.hop_interval).toBeTruthy();
  });

  it("allows empty values", () => {
    const errors: FieldErrors = {};
    checkDuration(errors, "hop_interval", "Hop interval", "");
    checkDuration(errors, "hop_interval", "Hop interval", null);
    expect(isEmpty(errors)).toBe(true);
  });

  it("accepts the supported units", () => {
    for (const value of ["30s", "1m", "1500ms", "2h"]) {
      const errors: FieldErrors = {};
      checkDuration(errors, "hop_interval", "Hop interval", value);
      expect(isEmpty(errors), value).toBe(true);
    }
  });
});

describe("checkPortRanges", () => {
  it("rejects malformed and inverted ranges", () => {
    for (const value of ["oops", "3000:2080", "0", "70000"]) {
      const errors: FieldErrors = {};
      checkPortRanges(errors, "server_ports", [value]);
      expect(errors.server_ports, value).toBeTruthy();
    }
  });

  it("accepts single ports and ascending ranges", () => {
    const errors: FieldErrors = {};
    checkPortRanges(errors, "server_ports", ["443", "2080:3000"]);
    expect(isEmpty(errors)).toBe(true);
  });
});

describe("mergeServerErrors", () => {
  it("keeps local messages and adds missing ones", () => {
    const merged = mergeServerErrors(
      { name: "local message" },
      { name: "server message", port: "invalid port" },
    );
    expect(merged.name).toBe("local message");
    expect(merged.port).toBe("invalid port");
  });
});

describe("validateUserForm", () => {
  it("accepts a plain remark", () => {
    expect(validateUserForm({ remark: "alice" })).toEqual({});
  });

  it("accepts a url-safe subscription path", () => {
    expect(validateUserForm({ remark: "alice", token: "alice_link-01" })).toEqual({});
  });

  it("rejects characters that would need escaping in a path", () => {
    expect(validateUserForm({ remark: "a", token: "with space" }).token).toBe(
      "Subscription path can only contain letters, numbers, - and _",
    );
    expect(validateUserForm({ remark: "a", token: "with/slash" }).token).toBeTruthy();
    expect(validateUserForm({ remark: "a", token: "with?query" }).token).toBeTruthy();
  });

  it("bounds the subscription path length", () => {
    expect(validateUserForm({ remark: "a", token: "x".repeat(65) }).token).toBe(
      "Subscription path must be at most 64 characters",
    );
  });

  it("treats an empty path as generated", () => {
    expect(validateUserForm({ remark: "a", token: "   " })).toEqual({});
  });

  it("accepts a normal remark", () => {
    expect(isEmpty(validateUserForm({ remark: "Alice" }))).toBe(true);
  });

  it("limits the remark length", () => {
    expect(validateUserForm({ remark: "x".repeat(65) }).remark).toBe(
      "Remark must be at most 64 characters",
    );
    expect(isEmpty(validateUserForm({ remark: "x".repeat(64) }))).toBe(true);
  });
});

describe("validateCredentials", () => {
  const uuid = "5783a3e7-e373-51cd-8642-c83782b807c5";

  it("accepts one row per user", () => {
    const errors: FieldErrors = {};
    validateCredentials(errors, [
      { user_id: 1, uuid },
      { user_id: 2, uuid: "" },
      { user_id: 3, uuid: null },
    ]);
    expect(isEmpty(errors)).toBe(true);
  });

  it("rejects two rows for the same user", () => {
    const errors: FieldErrors = {};
    validateCredentials(errors, [{ user_id: 1 }, { user_id: 1 }]);
    expect(errors["credentials.1.user_id"]).toBe(
      "A user can only hold one credential per node",
    );
  });

  it("rejects a malformed uuid but allows it to be omitted", () => {
    const bad: FieldErrors = {};
    validateCredentials(bad, [{ user_id: 1, uuid: "not-a-uuid" }]);
    expect(bad["credentials.0.uuid"]).toContain("Invalid UUID format");

    const blank: FieldErrors = {};
    validateCredentials(blank, [{ user_id: 1, uuid: "   " }]);
    expect(isEmpty(blank)).toBe(true);
  });

  it("rejects a row without a user", () => {
    const errors: FieldErrors = {};
    validateCredentials(errors, [{ user_id: 0 }]);
    expect(errors["credentials.0.user_id"]).toBe(
      "Please choose the user this credential belongs to",
    );
  });
});

describe("validateSettingsForm", () => {
  const base = {
    subscription_prefix: "/s",
    default_target: "sing-box",
    site_name: "NSM",
    failure_delay_ms: "0",
  };

  it("accepts sensible settings", () => {
    expect(isEmpty(validateSettingsForm(base))).toBe(true);
  });

  it("requires a leading slash on the prefix", () => {
    expect(validateSettingsForm({ ...base, subscription_prefix: "s" }).subscription_prefix).toBe(
      "Subscription prefix must start with /",
    );
  });

  it("rejects reserved characters in the prefix", () => {
    expect(
      validateSettingsForm({ ...base, subscription_prefix: "/s?x" }).subscription_prefix,
    ).toBeTruthy();
  });

  it("requires a non-empty site name", () => {
    expect(validateSettingsForm({ ...base, site_name: " " }).site_name).toBe(
      "Site name cannot be empty",
    );
  });

  it("bounds the failure delay", () => {
    expect(
      validateSettingsForm({ ...base, failure_delay_ms: "10001" }).failure_delay_ms,
    ).toBeTruthy();
    expect(isEmpty(validateSettingsForm({ ...base, failure_delay_ms: "5000" }))).toBe(true);
  });

  it("requires a https template url when pass-through is on", () => {
    expect(
      validateSettingsForm({ ...base, disguise_passthrough: true }).disguise_template_url,
    ).toBe("Fill in the template URL before enabling 404 passthrough");
  });

  it("rejects a plain http template url", () => {
    expect(
      validateSettingsForm({
        ...base,
        disguise_passthrough: true,
        disguise_template_url: "http://sample.example.com/",
      }).disguise_template_url,
    ).toBe("Template URL must start with https:// and include a hostname");
  });

  it("accepts a valid template url and override", () => {
    expect(
      isEmpty(
        validateSettingsForm({
          ...base,
          disguise_passthrough: true,
          disguise_template_url: "https://sample.example.com/",
          disguise_template_ip: "203.0.113.7",
          disguise_template_port: "8443",
        }),
      ),
    ).toBe(true);
  });

  it("rejects a malformed override address and port", () => {
    const errors = validateSettingsForm({
      ...base,
      disguise_template_ip: "https://host.docker.internal",
      disguise_template_port: "70000",
    });
    expect(errors.disguise_template_ip).toBe(
      "Server address should be hostname or IP only, without protocol or path",
    );
    expect(errors.disguise_template_port).toBe("Server port must be between 0 and 65535");
  });

  it("accepts a hostname override", () => {
    expect(
      validateSettingsForm({ ...base, disguise_template_ip: "host.docker.internal" })
        .disguise_template_ip,
    ).toBeUndefined();
  });

  it("accepts an ipv6 override", () => {
    expect(
      validateSettingsForm({ ...base, disguise_template_ip: "2001:db8::1" }).disguise_template_ip,
    ).toBeUndefined();
  });
});
