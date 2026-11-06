/*
 * Copyright (C) 2026 Prescott Anderson
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * This file is part of NSM (NeoSubscriptionMastodon), licensed under the
 * GNU Affero General Public License v3.0 or later. See the LICENSE file
 * in the project root for the full text.
 */

import type { NodeParams, ServerCore } from "../types";

/** Proxy servers NSM can emit an inbound config for. */
export const SERVER_CORES: { value: ServerCore; label: string }[] = [
  { value: "xray", label: "Xray" },
  { value: "hysteria2", label: "Hysteria2" },
];

/**
 * The core that can actually run a listener for `protocol`.
 *
 * Xray has no Hysteria2 inbound and the Hysteria2 exporter only understands
 * Hysteria2, so each protocol has exactly one sensible server core.
 */
export function serverCore(protocol: NodeParams["protocol"]): ServerCore {
  return protocol === "hysteria2" ? "hysteria2" : "xray";
}
