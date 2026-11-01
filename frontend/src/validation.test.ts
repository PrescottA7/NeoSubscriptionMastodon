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
    checkText(errors, "name", "节点名称", "   ", 1, 64);
    expect(errors.name).toBe("节点名称不能为空");
  });

  it("enforces both bounds", () => {
    const tooShort: FieldErrors = {};
    checkText(tooShort, "username", "用户名", "ab", 3, 10);
    expect(tooShort.username).toBe("用户名至少 3 个字符");

    const tooLong: FieldErrors = {};
    checkText(tooLong, "username", "用户名", "a".repeat(11), 3, 10);
    expect(tooLong.username).toBe("用户名最多 10 个字符");
  });

  it("accepts a value inside the bounds", () => {
    const errors: FieldErrors = {};
    checkText(errors, "name", "节点名称", "香港 01", 1, 64);
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
    checkInteger(blank, "sort_order", "排序", " ");
    expect(blank.sort_order).toBe("排序不能为空");

    const fractional: FieldErrors = {};
    checkInteger(fractional, "sort_order", "排序", "1.2");
    expect(fractional.sort_order).toBe("排序必须是整数");
  });
});

describe("checkDuration", () => {
  it("requires a unit", () => {
    const errors: FieldErrors = {};
    checkDuration(errors, "hop_interval", "跳跃间隔", "30");
    expect(errors.hop_interval).toBeTruthy();
  });

  it("allows empty values", () => {
    const errors: FieldErrors = {};
    checkDuration(errors, "hop_interval", "跳跃间隔", "");
    checkDuration(errors, "hop_interval", "跳跃间隔", null);
    expect(isEmpty(errors)).toBe(true);
  });

  it("accepts the supported units", () => {
    for (const value of ["30s", "1m", "1500ms", "2h"]) {
      const errors: FieldErrors = {};
      checkDuration(errors, "hop_interval", "跳跃间隔", value);
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
      { name: "本地消息" },
      { name: "服务端消息", port: "端口不合法" },
    );
    expect(merged.name).toBe("本地消息");
    expect(merged.port).toBe("端口不合法");
  });
});

describe("validateUserForm", () => {
  it("accepts a normal remark", () => {
    expect(isEmpty(validateUserForm({ remark: "Alice" }))).toBe(true);
  });

  it("limits the remark length", () => {
    expect(validateUserForm({ remark: "x".repeat(65) }).remark).toBe("备注最多 64 个字符");
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
    expect(errors["credentials.1.user_id"]).toBe("同一用户在该节点上只能有一组凭据");
  });

  it("rejects a malformed uuid but allows it to be omitted", () => {
    const bad: FieldErrors = {};
    validateCredentials(bad, [{ user_id: 1, uuid: "not-a-uuid" }]);
    expect(bad["credentials.0.uuid"]).toContain("UUID 格式不正确");

    const blank: FieldErrors = {};
    validateCredentials(blank, [{ user_id: 1, uuid: "   " }]);
    expect(isEmpty(blank)).toBe(true);
  });

  it("rejects a row without a user", () => {
    const errors: FieldErrors = {};
    validateCredentials(errors, [{ user_id: 0 }]);
    expect(errors["credentials.0.user_id"]).toBe("请选择凭据所属的用户");
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
      "订阅路径前缀必须以 / 开头",
    );
  });

  it("rejects reserved characters in the prefix", () => {
    expect(
      validateSettingsForm({ ...base, subscription_prefix: "/s?x" }).subscription_prefix,
    ).toBeTruthy();
  });

  it("requires a non-empty site name", () => {
    expect(validateSettingsForm({ ...base, site_name: " " }).site_name).toBe("站点名称不能为空");
  });

  it("bounds the failure delay", () => {
    expect(
      validateSettingsForm({ ...base, failure_delay_ms: "10001" }).failure_delay_ms,
    ).toBeTruthy();
    expect(isEmpty(validateSettingsForm({ ...base, failure_delay_ms: "5000" }))).toBe(true);
  });
});
