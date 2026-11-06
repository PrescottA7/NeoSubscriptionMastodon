/*
 * Copyright (C) 2026 Prescott Anderson
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * This file is part of NSM (NeoSubscriptionMastodon), licensed under the
 * GNU Affero General Public License v3.0 or later. See the LICENSE file
 * in the project root for the full text.
 */

import { describe, expect, it } from "vitest";

import { duplicateName, parsePositiveId } from "./utils";

describe("parsePositiveId", () => {
  it("accepts a positive integer", () => {
    expect(parsePositiveId("7")).toBe(7);
  });

  it("rejects blank, zero, negative, fractional and non-numeric values", () => {
    for (const value of ["", "   ", "0", "-3", "1.5", "abc", "3x"]) {
      expect(parsePositiveId(value)).toBeNull();
    }
  });

  it("rejects a missing parameter", () => {
    expect(parsePositiveId(null)).toBeNull();
  });
});

describe("duplicateName", () => {
  it("suffixes the original name", () => {
    expect(duplicateName("Tokyo")).toBe("Tokyo (copy)");
  });

  it("counts up while the suggestion is taken", () => {
    expect(duplicateName("Tokyo", ["Tokyo (copy)"])).toBe("Tokyo (copy 2)");
    expect(duplicateName("Tokyo", ["Tokyo (copy)", "Tokyo (copy 2)"])).toBe("Tokyo (copy 3)");
  });

  it("ignores unrelated names and surrounding whitespace", () => {
    expect(duplicateName("Tokyo", ["Osaka", " Tokyo (copy) "])).toBe("Tokyo (copy 2)");
  });

  it("falls back to a usable stem for a blank name", () => {
    expect(duplicateName("   ")).toBe("node (copy)");
  });
});
