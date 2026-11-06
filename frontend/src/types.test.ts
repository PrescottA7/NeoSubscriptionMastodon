/*
 * Copyright (C) 2026 Prescott Anderson
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * This file is part of NSM (NeoSubscriptionMastodon), licensed under the
 * GNU Affero General Public License v3.0 or later. See the LICENSE file
 * in the project root for the full text.
 */

import { describe, expect, it } from "vitest";

import { NodeSchema } from "./types";

const baseNode = {
  id: 1,
  name: "Tokyo",
  protocol: "vless",
  server: "example.com",
  port: 443,
  enabled: true,
  sort_order: 0,
  created_at: "2026-01-01T00:00:00Z",
  updated_at: "2026-01-01T00:00:00Z",
};

const vlessParams = {
  protocol: "vless",
  flow: "xtls-rprx-vision",
  packet_encoding: "xudp",
  encryption: "",
  tls: {
    enabled: false,
    server_name: "www.microsoft.com",
    insecure: false,
    alpn: ["h2"],
    fingerprint: "chrome",
    reality: {
      enabled: true,
      public_key: "pbk",
      short_id: "0123abcd",
      spider_x: "/",
    },
  },
  transport: { kind: "tcp", header_type: "none" },
  mux: { enabled: false, protocol: "smux", padding: false },
};

describe("NodeSchema", () => {
  it("parses a vless node with reality params", () => {
    const parsed = NodeSchema.parse({ ...baseNode, params: vlessParams });

    expect(parsed.params.protocol).toBe("vless");
    if (parsed.params.protocol === "vless") {
      expect(parsed.params.tls.reality?.public_key).toBe("pbk");
      expect(parsed.params.transport.kind).toBe("tcp");
    }
  });

  it("parses an xhttp transport with headers and extra", () => {
    const parsed = NodeSchema.parse({
      ...baseNode,
      params: {
        ...vlessParams,
        transport: {
          kind: "xhttp",
          host: "cdn.example.com",
          path: "/x",
          mode: "packet-up",
          headers: { "X-Test": "1" },
          extra: {
            x_padding_bytes: "100-1000",
            xmux: { max_concurrency: "4" },
          },
        },
      },
    });

    if (parsed.params.protocol === "vless" && parsed.params.transport.kind === "xhttp") {
      expect(parsed.params.transport.mode).toBe("packet-up");
      expect(parsed.params.transport.headers).toEqual({ "X-Test": "1" });
      expect(parsed.params.transport.extra?.x_padding_bytes).toBe("100-1000");
    } else {
      throw new Error("expected xhttp transport");
    }
  });

  it("accepts the full Xray XHTTP field set in snake_case", () => {
    // Mirrors Rust's `XhttpExtra`; every key is read from Xray's
    // SplitHTTPConfig, so the form must never reject a legitimate field.
    const parsed = NodeSchema.parse({
      ...baseNode,
      params: {
        ...vlessParams,
        transport: {
          kind: "xhttp",
          path: "/x",
          mode: "packet-up",
          extra: {
            x_padding_bytes: "100-1000",
            x_padding_obfs_mode: true,
            x_padding_key: "x_padding",
            x_padding_header: "X-Padding",
            x_padding_placement: "queryInHeader",
            x_padding_method: "repeat-x",
            uplink_http_method: "POST",
            session_id_placement: "header",
            session_id_key: "X-Session",
            session_id_table: "base62",
            session_id_length: "10-20",
            seq_placement: "query",
            seq_key: "seq",
            uplink_data_placement: "body",
            uplink_data_key: "data",
            uplink_chunk_size: "1024",
            no_grpc_header: true,
            no_sse_header: true,
            sc_max_each_post_bytes: "1000000",
            sc_min_posts_interval_ms: "30",
            sc_max_buffered_posts: 100,
            sc_stream_up_server_secs: "30-60",
            server_max_header_bytes: 8192,
            xmux: { max_concurrency: "16-32", h_keep_alive_period: 60 },
            download_settings: {
              address: "dl.example.com",
              port: 443,
              network: "xhttp",
              security: "tls",
              tls_settings: { server_name: "cdn.example.com" },
              xhttp_settings: { path: "/dl", mode: "auto" },
            },
          },
        },
      },
    });

    if (parsed.params.protocol === "vless" && parsed.params.transport.kind === "xhttp") {
      const extra = parsed.params.transport.extra;
      expect(extra?.x_padding_obfs_mode).toBe(true);
      expect(extra?.session_id_table).toBe("base62");
      expect(extra?.sc_max_buffered_posts).toBe(100);
      expect(extra?.server_max_header_bytes).toBe(8192);
      expect(extra?.xmux?.h_keep_alive_period).toBe(60);
      expect(extra?.download_settings?.tls_settings?.server_name).toBe("cdn.example.com");
    } else {
      throw new Error("expected xhttp transport");
    }
  });

  it("rejects unknown protocols", () => {
    expect(() =>
      NodeSchema.parse({ ...baseNode, params: { protocol: "ss", tls: {} } }),
    ).toThrow();
  });

  it("applies defaults for omitted optional fields", () => {
    const parsed = NodeSchema.parse({
      ...baseNode,
      protocol: "hysteria2",
      params: { protocol: "hysteria2", tls: { enabled: true } },
    });

    if (parsed.params.protocol === "hysteria2") {
      expect(parsed.params.server_ports).toEqual([]);
      expect(parsed.params.quic.disable_path_mtu_discovery).toBe(false);
      expect(parsed.params.brutal_debug).toBe(false);
    }
  });
});
