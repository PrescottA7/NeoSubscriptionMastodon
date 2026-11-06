/*
 * Copyright (C) 2026 Prescott Anderson
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * This file is part of NSM (NeoSubscriptionMastodon), licensed under the
 * GNU Affero General Public License v3.0 or later. See the LICENSE file
 * in the project root for the full text.
 */

import type { Transport } from "../types";

export const TRANSPORTS = [
  { value: "tcp", label: "TCP (RAW)" },
  { value: "ws", label: "WebSocket" },
  { value: "grpc", label: "gRPC" },
  { value: "http", label: "HTTP/2" },
  { value: "httpupgrade", label: "HTTPUpgrade" },
  { value: "xhttp", label: "XHTTP" },
] as const;

/** Creates the default option set for a transport, keeping the form valid on switch. */
export function defaultTransport(kind: Transport["kind"]): Transport {
  switch (kind) {
    case "tcp":
      return { kind: "tcp", header_type: "none" };
    case "ws":
      return {
        kind: "ws",
        path: "/",
        host: "",
        headers: {},
        max_early_data: null,
        early_data_header_name: "",
      };
    case "grpc":
      return { kind: "grpc", service_name: "", mode: "", authority: "" };
    case "http":
      return { kind: "http", host: [], path: "/", method: "" };
    case "httpupgrade":
      return { kind: "httpupgrade", host: "", path: "/" };
    case "xhttp":
      return { kind: "xhttp", host: "", path: "/", mode: "auto", extra: null, headers: {} };
  }
}
