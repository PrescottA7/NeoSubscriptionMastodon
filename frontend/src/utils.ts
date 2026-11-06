/*
 * Copyright (C) 2026 Prescott Anderson
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * This file is part of NSM (NeoSubscriptionMastodon), licensed under the
 * GNU Affero General Public License v3.0 or later. See the LICENSE file
 * in the project root for the full text.
 */

/**
 * Small helpers with no React, store, or network dependency, mirroring the role
 * of `utils.rs` on the server side.
 */

/** Parses a positive integer from a route or query value, rejecting junk. */
export function parsePositiveId(value: string | null): number | null {
  if (value === null || value.trim() === "") {
    return null;
  }
  const parsed = Number(value);
  return Number.isInteger(parsed) && parsed > 0 ? parsed : null;
}

/**
 * Suggests a name for a duplicate of `name`: `Tokyo` becomes `Tokyo (copy)`, and
 * a second copy becomes `Tokyo (copy 2)`.
 *
 * Node names are not unique in the database, so this only produces a readable
 * default; the admin stays free to type something else.
 */
export function duplicateName(name: string, taken: readonly string[] = []): string {
  const trimmed = name.trim() || "node";
  const existing = new Set(taken.map((item) => item.trim()));
  let candidate = `${trimmed} (copy)`;
  for (let index = 2; existing.has(candidate); index += 1) {
    candidate = `${trimmed} (copy ${index})`;
  }
  return candidate;
}
