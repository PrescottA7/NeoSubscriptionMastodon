import type { ServerConfig } from "../types";

/** A reasonable starting point: VLESS over TCP with sniffing enabled. */
export function emptyServerConfig(): ServerConfig {
  return {
    core: "xray",
    inbound: {
      protocol: "vless",
      listen: "0.0.0.0",
      port: 443,
      tag: "inbound",
      sniffing: true,
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
    fallbacks: [],
    sockopt: {
      tcp_fast_open: false,
      accept_proxy_protocol: false,
      mark: null,
      tcp_keep_alive_interval: null,
    },
  };
}
