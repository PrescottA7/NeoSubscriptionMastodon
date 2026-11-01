import { z } from "zod";

/* ------------------------------------------------------------------ */
/* Protocol parameter schemas (mirror of the Rust `ProxyKind` model)   */
/* ------------------------------------------------------------------ */

export const RealitySchema = z.object({
  enabled: z.boolean().default(false),
  public_key: z.string().default(""),
  short_id: z.string().optional().nullable(),
  spider_x: z.string().optional().nullable(),
  mldsa65_verify: z.string().optional().nullable(),
});

export const TlsSchema = z
  .object({
    enabled: z.boolean().default(false),
    server_name: z.string().optional().nullable(),
    insecure: z.boolean().default(false),
    alpn: z.array(z.string()).default([]),
    fingerprint: z.string().optional().nullable(),
    certificate_sha256: z.string().optional().nullable(),
    verify_peer_cert_by_name: z.string().optional().nullable(),
    min_version: z.string().optional().nullable(),
    max_version: z.string().optional().nullable(),
    reality: RealitySchema.optional().nullable(),
  })
  .default({ enabled: false, insecure: false, alpn: [] });

export const MuxSchema = z
  .object({
    enabled: z.boolean().default(false),
    protocol: z.enum(["smux", "yamux", "h2mux"]).default("smux"),
    max_streams: z.number().int().positive().optional().nullable(),
    padding: z.boolean().default(false),
    brutal: z
      .object({
        enabled: z.boolean().default(false),
        up_mbps: z.number().int().positive().optional().nullable(),
        down_mbps: z.number().int().positive().optional().nullable(),
      })
      .optional()
      .nullable(),
  })
  .default({ enabled: false, protocol: "smux", padding: false });

export const QuicSchema = z
  .object({
    initial_stream_receive_window: z.number().int().positive().optional().nullable(),
    max_stream_receive_window: z.number().int().positive().optional().nullable(),
    initial_connection_receive_window: z.number().int().positive().optional().nullable(),
    max_connection_receive_window: z.number().int().positive().optional().nullable(),
    keep_alive_period: z.string().optional().nullable(),
    disable_path_mtu_discovery: z.boolean().default(false),
  })
  .default({ disable_path_mtu_discovery: false });

export const XhttpModeSchema = z.enum(["auto", "stream-one", "stream-up", "packet-up"]);
export type XhttpMode = z.infer<typeof XhttpModeSchema>;

export const XmuxSchema = z.object({
  max_concurrency: z.string().optional().nullable(),
  max_connections: z.string().optional().nullable(),
  c_max_reuse_times: z.string().optional().nullable(),
  c_max_lifetime_ms: z.string().optional().nullable(),
  h_max_request_times: z.string().optional().nullable(),
  h_max_reusable_secs: z.string().optional().nullable(),
  h_keep_alive_period: z.number().int().optional().nullable(),
});

/** Mirror of Xray's `SplitHTTPConfig`, stored in snake_case like the rest of the API. */
export const XhttpExtraSchema = z.object({
  x_padding_bytes: z.string().optional().nullable(),
  x_padding_obfs_mode: z.boolean().optional().nullable(),
  x_padding_key: z.string().optional().nullable(),
  x_padding_header: z.string().optional().nullable(),
  x_padding_placement: z.string().optional().nullable(),
  x_padding_method: z.string().optional().nullable(),
  uplink_http_method: z.string().optional().nullable(),
  session_id_placement: z.string().optional().nullable(),
  session_id_key: z.string().optional().nullable(),
  session_id_table: z.string().optional().nullable(),
  session_id_length: z.string().optional().nullable(),
  seq_placement: z.string().optional().nullable(),
  seq_key: z.string().optional().nullable(),
  uplink_data_placement: z.string().optional().nullable(),
  uplink_data_key: z.string().optional().nullable(),
  uplink_chunk_size: z.string().optional().nullable(),
  no_grpc_header: z.boolean().optional().nullable(),
  no_sse_header: z.boolean().optional().nullable(),
  sc_max_each_post_bytes: z.string().optional().nullable(),
  sc_min_posts_interval_ms: z.string().optional().nullable(),
  sc_max_buffered_posts: z.number().int().optional().nullable(),
  sc_stream_up_server_secs: z.string().optional().nullable(),
  server_max_header_bytes: z.number().int().optional().nullable(),
  xmux: XmuxSchema.optional().nullable(),
  download_settings: z
    .object({
      address: z.string().optional().nullable(),
      port: z.number().int().optional().nullable(),
      network: z.string().optional().nullable(),
      security: z.string().optional().nullable(),
      tls_settings: z
        .object({
          server_name: z.string().optional().nullable(),
          fingerprint: z.string().optional().nullable(),
          public_key: z.string().optional().nullable(),
          short_id: z.string().optional().nullable(),
        })
        .optional()
        .nullable(),
      xhttp_settings: z
        .object({
          host: z.string().optional().nullable(),
          path: z.string().optional().nullable(),
          mode: z.string().optional().nullable(),
        })
        .optional()
        .nullable(),
    })
    .optional()
    .nullable(),
});

export const TransportSchema = z.discriminatedUnion("kind", [
  z.object({ kind: z.literal("tcp"), header_type: z.enum(["none", "http"]).default("none") }),
  z.object({
    kind: z.literal("ws"),
    path: z.string().optional().nullable(),
    host: z.string().optional().nullable(),
    headers: z.record(z.string(), z.string()).default({}),
    max_early_data: z.number().int().nonnegative().optional().nullable(),
    early_data_header_name: z.string().optional().nullable(),
  }),
  z.object({
    kind: z.literal("grpc"),
    service_name: z.string().optional().nullable(),
    mode: z.string().optional().nullable(),
    authority: z.string().optional().nullable(),
  }),
  z.object({
    kind: z.literal("http"),
    host: z.array(z.string()).default([]),
    path: z.string().optional().nullable(),
    method: z.string().optional().nullable(),
  }),
  z.object({
    kind: z.literal("httpupgrade"),
    host: z.string().optional().nullable(),
    path: z.string().optional().nullable(),
  }),
  z.object({
    kind: z.literal("xhttp"),
    host: z.string().optional().nullable(),
    path: z.string().optional().nullable(),
    mode: XhttpModeSchema.default("auto"),
    extra: XhttpExtraSchema.optional().nullable(),
    headers: z.record(z.string(), z.string()).default({}).optional(),
  }),
]);

export type Transport = z.infer<typeof TransportSchema>;
export type Tls = z.infer<typeof TlsSchema>;
export type Mux = z.infer<typeof MuxSchema>;
export type Quic = z.infer<typeof QuicSchema>;

/* ------------------------------------------------------------------ */
/* Node params                                                        */
/* ------------------------------------------------------------------ */

export const VlessParamsSchema = z.object({
  protocol: z.literal("vless"),
  flow: z.string().optional().nullable(),
  packet_encoding: z.string().optional().nullable(),
  encryption: z.string().optional().nullable(),
  tls: TlsSchema,
  transport: TransportSchema,
  mux: MuxSchema,
});

export const TrojanParamsSchema = z.object({
  protocol: z.literal("trojan"),
  udp: z.boolean().default(false),
  tls: TlsSchema,
  transport: TransportSchema,
  mux: MuxSchema,
});

export const ObfsSchema = z.object({
  obfs_type: z.enum(["salamander", "gecko"]).default("salamander"),
  password: z.string().default(""),
  min_packet_size: z.number().int().positive().optional().nullable(),
  max_packet_size: z.number().int().positive().optional().nullable(),
});

export const RealmSchema = z.object({
  server_url: z.string().default(""),
  token: z.string().optional().nullable(),
  realm_id: z.string().default(""),
  stun_servers: z.array(z.string()).default([]),
  ip_version: z.number().int().optional().nullable(),
  port_mapping: z
    .object({
      enabled: z.boolean().default(false),
      timeout: z.string().optional().nullable(),
      lifetime: z.string().optional().nullable(),
    })
    .optional()
    .nullable(),
});

export const Hysteria2ParamsSchema = z.object({
  protocol: z.literal("hysteria2"),
  server_ports: z.array(z.string()).default([]),
  hop_interval: z.string().optional().nullable(),
  hop_interval_max: z.string().optional().nullable(),
  up_mbps: z.number().int().positive().optional().nullable(),
  down_mbps: z.number().int().positive().optional().nullable(),
  obfs: ObfsSchema.optional().nullable(),
  bbr_profile: z.string().optional().nullable(),
  brutal_debug: z.boolean().default(false),
  disable_chrome_parrot: z.boolean().default(false),
  realm: RealmSchema.optional().nullable(),
  tls: TlsSchema,
  quic: QuicSchema,
});

export const TuicParamsSchema = z.object({
  protocol: z.literal("tuic"),
  congestion_control: z.enum(["cubic", "new_reno", "bbr"]).default("cubic"),
  udp_relay_mode: z.enum(["native", "quic"]).default("native"),
  udp_over_stream: z.boolean().default(false),
  zero_rtt_handshake: z.boolean().default(false),
  heartbeat: z.string().optional().nullable(),
  tls: TlsSchema,
  quic: QuicSchema,
});

export const AnytlsParamsSchema = z.object({
  protocol: z.literal("anytls"),
  idle_session_check_interval: z.string().optional().nullable(),
  idle_session_timeout: z.string().optional().nullable(),
  min_idle_session: z.number().int().nonnegative().optional().nullable(),
  client_metadata: z.string().optional().nullable(),
  tls: TlsSchema,
});

export const NodeParamsSchema = z.discriminatedUnion("protocol", [
  VlessParamsSchema,
  TrojanParamsSchema,
  Hysteria2ParamsSchema,
  TuicParamsSchema,
  AnytlsParamsSchema,
]);

export type NodeParams = z.infer<typeof NodeParamsSchema>;
export type ProtocolName = NodeParams["protocol"];
export type VlessParams = z.infer<typeof VlessParamsSchema>;
export type TrojanParams = z.infer<typeof TrojanParamsSchema>;
export type Hysteria2Params = z.infer<typeof Hysteria2ParamsSchema>;
export type TuicParams = z.infer<typeof TuicParamsSchema>;
export type AnytlsParams = z.infer<typeof AnytlsParamsSchema>;

/* ------------------------------------------------------------------ */
/* Server-side (inbound) configuration                                */
/* ------------------------------------------------------------------ */

export const InboundProtocolSchema = z.enum([
  "vless",
  "trojan",
  "hysteria2",
  "tuic",
  "anytls",
]);

export const ServerCoreSchema = z.enum(["xray", "hysteria2"]);

export type ServerCore = z.infer<typeof ServerCoreSchema>;

export const InboundSchema = z.object({
  protocol: InboundProtocolSchema.default("vless"),
  listen: z.string().default("0.0.0.0"),
  port: z.number().int().min(1).max(65535).default(443),
  tag: z.string().default("inbound"),
  sniffing: z.boolean().default(true),
  dest_override: z.array(z.string()).default(["http", "tls", "quic"]),
  transport: z.string().default("tcp"),
  path: z.string().default("/"),
  host: z.string().default(""),
  service_name: z.string().default(""),
});

export const ServerTlsSchema = z.object({
  certificate_file: z.string().default(""),
  key_file: z.string().default(""),
  security: z.string().default("tls"),
  alpn: z.array(z.string()).default([]),
});

/**
 * REALITY listener settings. The private key lives here and never reaches a
 * client document; the matching public key is derived server-side.
 */
export const ServerRealitySchema = z.object({
  enabled: z.boolean().default(false),
  private_key: z.string().default(""),
  server_names: z.array(z.string()).default([]),
  target: z.string().default(""),
  short_ids: z.array(z.string()).default([]),
  mldsa65_seed: z.string().default(""),
  max_time_diff: z.number().int().default(0),
  min_client_ver: z.string().default(""),
  max_client_ver: z.string().default(""),
});

export const FallbackSchema = z.object({
  dest: z.string().default(""),
  alpn: z.string().default(""),
  name: z.string().default(""),
  path: z.string().default(""),
});

export const SockoptSchema = z.object({
  tcp_fast_open: z.boolean().default(false),
  accept_proxy_protocol: z.boolean().default(false),
  mark: z.number().int().nullable().optional(),
  tcp_keep_alive_interval: z.number().int().nullable().optional(),
});

export const ServerConfigSchema = z.object({
  core: ServerCoreSchema.default("xray"),
  inbound: InboundSchema.default(() => InboundSchema.parse({})),
  tls: ServerTlsSchema.default(() => ServerTlsSchema.parse({})),
  reality: ServerRealitySchema.default(() => ServerRealitySchema.parse({})),
  fallbacks: z.array(FallbackSchema).default([]),
  sockopt: SockoptSchema.default(() => SockoptSchema.parse({})),
});

export type ServerConfig = z.infer<typeof ServerConfigSchema>;
export type ServerInbound = z.infer<typeof InboundSchema>;
export type ServerReality = z.infer<typeof ServerRealitySchema>;

/* ------------------------------------------------------------------ */
/* API payloads                                                       */
/* ------------------------------------------------------------------ */

export const AdminSchema = z.object({
  id: z.number(),
  username: z.string(),
});

export const SetupStatusSchema = z.object({
  needs_setup: z.boolean(),
  site_name: z.string(),
});

export const LoginResponseSchema = z.object({
  admin: AdminSchema,
});

export const UserSchema = z.object({
  id: z.number(),
  remark: z.string(),
  enabled: z.boolean(),
  token_hint: z.string(),
  access_count: z.number(),
  last_accessed_at: z.string().nullable(),
  created_at: z.string(),
  updated_at: z.string(),
});

/// Credentials a user holds on one node. Each proxy server keeps its own
/// account list, so these belong to the (node, user) pair.
export const CredentialSchema = z.object({
  node_id: z.number(),
  user_id: z.number(),
  uuid: z.string(),
  password: z.string(),
  email: z.string(),
  enabled: z.boolean(),
});

/// Credential row submitted from the node editor; empty values are generated.
export const CredentialInputSchema = z.object({
  user_id: z.number(),
  uuid: z.string().optional().nullable(),
  password: z.string().optional().nullable(),
  email: z.string().optional().nullable(),
  enabled: z.boolean().default(true),
});

export const ProvisionedUserSchema = z.object({
  user: UserSchema,
  token: z.string(),
  subscription_path: z.string(),
  subscription_url: z.string(),
});

export const NodeSchema = z.object({
  id: z.number(),
  name: z.string(),
  protocol: z.string(),
  server: z.string(),
  port: z.number(),
  enabled: z.boolean(),
  sort_order: z.number(),
  params: NodeParamsSchema,
  server_params: ServerConfigSchema.nullable().optional(),
  created_at: z.string(),
  updated_at: z.string(),
});

export const NodeDetailSchema = NodeSchema.extend({
  credentials: z.array(CredentialSchema),
});

export const ProtocolDescriptorSchema = z.object({
  name: z.string(),
  default_port: z.number(),
  requires_tls: z.boolean(),
  supports_transport: z.boolean(),
});

export const SettingsSchema = z.object({
  subscription_prefix: z.string(),
  default_target: z.string(),
  failure_delay_ms: z.number(),
  site_name: z.string(),
  has_custom_disguise_page: z.boolean(),
  disguise_page: z.string(),
  available_cores: z.array(z.string()),
});

export const RenderedDocumentSchema = z.object({
  core: z.string(),
  content_type: z.string(),
  filename: z.string(),
  content: z.string(),
});

export type Admin = z.infer<typeof AdminSchema>;
export type SetupStatus = z.infer<typeof SetupStatusSchema>;
export type User = z.infer<typeof UserSchema>;
export type Credential = z.infer<typeof CredentialSchema>;
export type CredentialInput = z.infer<typeof CredentialInputSchema>;
export type ProvisionedUser = z.infer<typeof ProvisionedUserSchema>;
export type Node = z.infer<typeof NodeSchema>;
export type NodeDetail = z.infer<typeof NodeDetailSchema>;
export type ProtocolDescriptor = z.infer<typeof ProtocolDescriptorSchema>;
export type Settings = z.infer<typeof SettingsSchema>;
export type RenderedDocument = z.infer<typeof RenderedDocumentSchema>;
