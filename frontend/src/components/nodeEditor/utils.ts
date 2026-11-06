/*
 * Copyright (C) 2026 Prescott Anderson
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * This file is part of NSM (NeoSubscriptionMastodon), licensed under the
 * GNU Affero General Public License v3.0 or later. See the LICENSE file
 * in the project root for the full text.
 */

import type { Credential, CredentialInput } from "../../types";
import type { CredentialValues } from "../credentials";

/** Editor state for one credential row. */
export interface CredentialRow extends CredentialValues {
  user_id: number;
  email: string;
  enabled: boolean;
}

export function toRow(credential: Credential): CredentialRow {
  return {
    user_id: credential.user_id,
    uuid: credential.uuid,
    password: credential.password,
    // The server stores the account label in `email`; that is the Hysteria2
    // username, so the editor exposes it under that name.
    username: credential.email,
    email: credential.email,
    enabled: credential.enabled,
  };
}

export function toInput(row: CredentialRow): CredentialInput {
  return {
    user_id: row.user_id,
    uuid: row.uuid.trim() || null,
    password: row.password.trim() || null,
    email: row.username.trim() || null,
    enabled: row.enabled,
  };
}
