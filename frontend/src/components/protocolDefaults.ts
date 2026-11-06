/*
 * Copyright (C) 2026 Prescott Anderson
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * This file is part of NSM (NeoSubscriptionMastodon), licensed under the
 * GNU Affero General Public License v3.0 or later. See the LICENSE file
 * in the project root for the full text.
 */

import type { NodeParams } from "../types";

/** Default protocol parameters, matching the Rust-side defaults. */
export function defaultParams(protocol: NodeParams["protocol"]): NodeParams {
  switch (protocol) {
    case "vless":
      return {
        protocol: "vless",
        flow: "",
        packet_encoding: "xudp",
        encryption: "",
        tls: defaultTls(),
        transport: { kind: "tcp", header_type: "none" },
        mux: { enabled: false, protocol: "smux", max_streams: null, padding: false },
      };
    case "trojan":
      return {
        protocol: "trojan",
        udp: false,
        // Trojan always runs over TLS; defaulting to off would fail validation.
        tls: { ...defaultTls(), enabled: true },
        transport: { kind: "tcp", header_type: "none" },
        mux: { enabled: false, protocol: "smux", max_streams: null, padding: false },
      };
    case "hysteria2":
      return {
        protocol: "hysteria2",
        server_ports: [],
        hop_interval: "",
        hop_interval_max: "",
        up_mbps: null,
        down_mbps: null,
        obfs: null,
        bbr_profile: "",
        brutal_debug: false,
        disable_chrome_parrot: false,
        realm: null,
        tls: { ...defaultTls(), enabled: true },
        quic: defaultQuic(),
      };
    case "tuic":
      return {
        protocol: "tuic",
        congestion_control: "cubic",
        udp_relay_mode: "native",
        udp_over_stream: false,
        zero_rtt_handshake: false,
        heartbeat: "",
        tls: { ...defaultTls(), enabled: true },
        quic: defaultQuic(),
      };
    case "anytls":
      return {
        protocol: "anytls",
        idle_session_check_interval: "",
        idle_session_timeout: "",
        min_idle_session: null,
        client_metadata: "",
        tls: { ...defaultTls(), enabled: true },
      };
  }
}

export function defaultTls() {
  return {
    enabled: false,
    server_name: "",
    insecure: false,
    alpn: [] as string[],
    fingerprint: "chrome",
    certificate_sha256: "",
    verify_peer_cert_by_name: "",
    min_version: "",
    max_version: "",
    reality: {
      enabled: false,
      public_key: "",
      short_id: "",
      spider_x: "",
      mldsa65_verify: "",
    },
  };
}

export function defaultQuic() {
  return {
    initial_stream_receive_window: null,
    max_stream_receive_window: null,
    initial_connection_receive_window: null,
    max_connection_receive_window: null,
    keep_alive_period: "",
    disable_path_mtu_discovery: false,
  };
}

export const PROTOCOL_LABELS: Record<NodeParams["protocol"], string> = {
  vless: "VLESS",
  trojan: "Trojan",
  hysteria2: "Hysteria2",
  tuic: "TUIC",
  anytls: "AnyTLS",
};
