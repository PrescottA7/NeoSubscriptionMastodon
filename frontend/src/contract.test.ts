import { describe, expect, it } from "vitest";
import {
  MuxSchema,
  RealmSchema,
  ServerConfigSchema,
  XhttpExtraSchema,
} from "./types";

describe("contract parity with the Rust model", () => {
  it("keeps mux brutal settings through a parse", () => {
    const parsed = MuxSchema.parse({
      enabled: true, protocol: "h2mux", max_streams: 8, padding: true,
      brutal: { enabled: true, up_mbps: 100, down_mbps: 200 },
    });
    expect(parsed.brutal?.up_mbps).toBe(100);
    expect(parsed.brutal?.down_mbps).toBe(200);
  });

  it("keeps realm port mapping through a parse", () => {
    const parsed = RealmSchema.parse({
      server_url: "https://realm.example.com", realm_id: "id",
      port_mapping: { enabled: true, timeout: "5m", lifetime: "1h" },
    });
    expect(parsed.port_mapping?.enabled).toBe(true);
    expect(parsed.port_mapping?.lifetime).toBe("1h");
  });

  it("does not silently drop unknown-by-mistake nested xhttp settings", () => {
    const parsed = XhttpExtraSchema.parse({
      x_padding_bytes: "100-1000",
      download_settings: {
        address: "dl.example.com", port: 443, network: "xhttp",
        security: "reality",
        tls_settings: { server_name: "cdn.example.com", public_key: "pbk", short_id: "ab" },
        xhttp_settings: { host: "h", path: "/dl", mode: "auto" },
      },
    });
    expect(parsed.download_settings?.security).toBe("reality");
    expect(parsed.download_settings?.tls_settings?.public_key).toBe("pbk");
    expect(parsed.download_settings?.xhttp_settings?.path).toBe("/dl");
  });

  it("round-trips a mux config through JSON without loss", () => {
    const input = {
      enabled: true, protocol: "smux" as const, max_streams: 4, padding: false,
      brutal: { enabled: true, up_mbps: 50, down_mbps: 60 },
    };
    const once = MuxSchema.parse(input);
    const twice = MuxSchema.parse(JSON.parse(JSON.stringify(once)));
    expect(twice).toEqual(once);
  });
});

describe("server configuration", () => {
  it("keeps REALITY private key settings through a parse", () => {
    const parsed = ServerConfigSchema.parse({
      core: "xray",
      inbound: { protocol: "vless", port: 8443, transport: "ws", path: "/ray" },
      reality: {
        enabled: true,
        private_key: "PRIVATE",
        server_names: ["www.example.com"],
        short_ids: ["ab"],
        target: "www.example.com:443",
      },
    });

    expect(parsed.inbound.port).toBe(8443);
    expect(parsed.inbound.transport).toBe("ws");
    expect(parsed.reality.private_key).toBe("PRIVATE");
    expect(parsed.reality.short_ids).toEqual(["ab"]);
  });

  it("fills in defaults so a partial config stays usable", () => {
    const parsed = ServerConfigSchema.parse({});
    expect(parsed.core).toBe("xray");
    expect(parsed.inbound.port).toBe(443);
    expect(parsed.inbound.sniffing).toBe(true);
    expect(parsed.reality.enabled).toBe(false);
    expect(parsed.fallbacks).toEqual([]);
  });

  it("rejects an unknown core", () => {
    expect(() => ServerConfigSchema.parse({ core: "v2ray" })).toThrow();
  });

  it("accepts the hysteria2 core", () => {
    expect(ServerConfigSchema.parse({ core: "hysteria2" }).core).toBe("hysteria2");
  });
});

