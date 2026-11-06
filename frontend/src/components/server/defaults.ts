/*
 * Copyright (C) 2026 Prescott Anderson
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * This file is part of NSM (NeoSubscriptionMastodon), licensed under the
 * GNU Affero General Public License v3.0 or later. See the LICENSE file
 * in the project root for the full text.
 */

import type { ServerConfig, ServerCore } from "../../types";

/** Inbound protocols an Xray listener can actually serve. */
export const XRAY_INBOUNDS = ["vless", "trojan"] as const;

/** Inbound protocols the Hysteria2 listener can serve. */
export const HYSTERIA2_INBOUNDS = ["hysteria2"] as const;

/** Share-link transport vocabulary, matching the Xray inbound names. */
export const XRAY_TRANSPORTS = ["tcp", "ws", "grpc", "httpupgrade", "xhttp"] as const;

/**
 * The core that can run a listener for `protocol`.
 *
 * Xray has no Hysteria2 inbound and the Hysteria2 exporter only understands
 * Hysteria2, so each protocol maps to exactly one runnable core. TUIC and AnyTLS
 * have no inbound in either, and are reported as `null` rather than silently
 * producing an empty account list.
 */
export function coreForProtocol(protocol: string): ServerCore | null {
  switch (protocol) {
    case "vless":
    case "trojan":
      return "xray";
    case "hysteria2":
      return "hysteria2";
    default:
      return null;
  }
}

/** Inbound protocols available for a core. */
export function inboundsForCore(core: ServerCore): readonly string[] {
  return core === "xray" ? XRAY_INBOUNDS : HYSTERIA2_INBOUNDS;
}

/** A starting server config whose listener matches `core` and `protocol`. */
export function emptyServerConfig(
  core: ServerCore = "xray",
  protocol: string = core === "hysteria2" ? "hysteria2" : "vless",
): ServerConfig {
  return {
    core,
    inbound: {
      protocol: protocol as ServerConfig["inbound"]["protocol"],
      listen: "0.0.0.0",
      port: 443,
      tag: "inbound",
      // Sniffing is an Xray inbound feature; Hysteria2 has no equivalent.
      sniffing: core === "xray",
      dest_override: ["http", "tls", "quic"],
      transport: "tcp",
      path: "/",
      host: "",
      service_name: "",
    },
    tls: { certificate_file: "", key_file: "", security: "tls", alpn: [] },
    reality: {
      enabled: false,
      private_key: "",
      server_names: [],
      target: "",
      short_ids: [],
      mldsa65_seed: "",
      max_time_diff: 0,
      min_client_ver: "",
      max_client_ver: "",
    },
    // Fallbacks are Xray `fallbacks` entries; Hysteria2 has no counterpart.
    fallbacks: [],
    sockopt: {
      tcp_fast_open: false,
      accept_proxy_protocol: false,
      mark: null,
      tcp_keep_alive_interval: null,
    },
  };
}

/**
 * Rewrites a config for a new core, keeping what still applies.
 *
 * Switching cores must not leave a listener the new core cannot run, so the
 * inbound protocol is clamped and Xray-only settings are reset.
 */
export function retargetCore(config: ServerConfig, core: ServerCore): ServerConfig {
  const allowed = inboundsForCore(core);
  const protocol = allowed.includes(config.inbound.protocol)
    ? config.inbound.protocol
    : (allowed[0] as ServerConfig["inbound"]["protocol"]);

  if (core === "hysteria2") {
    return {
      ...config,
      core,
      inbound: { ...config.inbound, protocol, transport: "tcp", sniffing: false },
      // Hysteria2 has neither REALITY nor fallbacks.
      reality: { ...config.reality, enabled: false },
      fallbacks: [],
    };
  }

  return { ...config, core, inbound: { ...config.inbound, protocol } };
}
