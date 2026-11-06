/*
 * Copyright (C) 2026 Prescott Anderson
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * This file is part of NSM (NeoSubscriptionMastodon), licensed under the
 * GNU Affero General Public License v3.0 or later. See the LICENSE file
 * in the project root for the full text.
 */

import type { ProtocolName, ServerCore } from "../../types";

/**
 * A credential value a proxy core can actually consume.
 *
 * The three are not interchangeable: Xray authenticates VLESS by UUID and
 * Trojan by password, while Hysteria2 has no notion of a UUID at all and uses a
 * `username: password` table. Rendering all three everywhere is what made the
 * old form lie about what the server would read.
 */
export type CredentialField = "uuid" | "password" | "username";

/** Values held for one (node, user) pair, as edited in the node editor. */
export interface CredentialValues {
  uuid: string;
  password: string;
  /** Account name; the server config uses it as the Hysteria2 username. */
  username: string;
}

export const EMPTY_CREDENTIALS: CredentialValues = {
  uuid: "",
  password: "",
  username: "",
};

/** Fields the protocol's own clients expect, independent of any server core. */
export function protocolCredentialFields(protocol: ProtocolName): CredentialField[] {
  switch (protocol) {
    case "vless":
      return ["uuid"];
    case "trojan":
      return ["password"];
    case "hysteria2":
      return ["username", "password"];
    case "tuic":
      return ["uuid", "password"];
    case "anytls":
      return ["password"];
  }
}

/**
 * Fields the selected server core reads from this listener.
 *
 * Xray inbounds take a UUID (VLESS) or a password (Trojan); the Hysteria2
 * exporter writes a `userpass` map, so it needs a username and a password.
 */
export function coreCredentialFields(
  core: ServerCore,
  protocol: ProtocolName,
): CredentialField[] {
  switch (core) {
    case "xray":
      return protocol === "trojan" ? ["password"] : ["uuid"];
    case "hysteria2":
      return ["username", "password"];
  }
}

/** True when the core ignores `field`, so the form can omit it. */
export function fieldIsUnused(
  core: ServerCore,
  protocol: ProtocolName,
  field: CredentialField,
): boolean {
  return !coreCredentialFields(core, protocol).includes(field);
}

/**
 * RFC 4122 v4 identifier, matching `generate_uuid` on the server.
 *
 * Built from `getRandomValues` rather than `randomUUID`, which is missing on
 * non-secure origins and would leave the field unfillable there.
 */
export function generateUuid(): string {
  const bytes = new Uint8Array(16);
  crypto.getRandomValues(bytes);
  bytes[6] = (bytes[6] & 0x0f) | 0x40;
  bytes[8] = (bytes[8] & 0x3f) | 0x80;
  const hex = Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join("");
  return `${hex.slice(0, 8)}-${hex.slice(8, 12)}-${hex.slice(12, 16)}-${hex.slice(16, 20)}-${hex.slice(20)}`;
}

/** URL-safe secret, long enough for a proxy password. */
export function generatePassword(): string {
  const bytes = new Uint8Array(24);
  crypto.getRandomValues(bytes);
  const binary = Array.from(bytes, (byte) => String.fromCharCode(byte)).join("");
  return btoa(binary).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}
