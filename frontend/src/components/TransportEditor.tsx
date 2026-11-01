import { useState } from "react";
import {
  Accordion,
  AccordionDetails,
  AccordionSummary,
  Box,
  Button,
  FormControl,
  FormControlLabel,
  IconButton,
  InputLabel,
  MenuItem,
  Select,
  Stack,
  Switch,
  TextField,
  Typography,
} from "@mui/material";
import { Add, ExpandMore } from "@mui/icons-material";

import type { Mux, Transport, XhttpMode } from "../types";
import { defaultTransport, TRANSPORTS } from "./transportDefaults";

export function TransportSection({
  transport,
  onChange,
  mux,
  onMuxChange,
}: {
  transport: Transport;
  onChange: (next: Transport) => void;
  mux: Mux;
  onMuxChange: (next: Mux) => void;
}) {
  return (
    <Accordion disableGutters>
      <AccordionSummary expandIcon={<ExpandMore />}>
        <Typography sx={{ flexGrow: 1 }}>传输层</Typography>
        <Typography variant="caption" color="text.secondary">
          {TRANSPORTS.find((item) => item.value === transport.kind)?.label}
        </Typography>
      </AccordionSummary>
      <AccordionDetails>
        <Stack spacing={2}>
          <FormControl size="small" fullWidth>
            <InputLabel>传输方式</InputLabel>
            <Select
              label="传输方式"
              value={transport.kind}
              onChange={(event) =>
                onChange(defaultTransport(event.target.value as Transport["kind"]))
              }
            >
              {TRANSPORTS.map((item) => (
                <MenuItem key={item.value} value={item.value}>
                  {item.label}
                </MenuItem>
              ))}
            </Select>
          </FormControl>

          {transport.kind === "tcp" && (
            <FormControl size="small" fullWidth>
              <InputLabel>伪装头</InputLabel>
              <Select
                label="伪装头"
                value={transport.header_type}
                onChange={(event) =>
                  onChange({ ...transport, header_type: event.target.value as "none" | "http" })
                }
              >
                <MenuItem value="none">无</MenuItem>
                <MenuItem value="http">HTTP</MenuItem>
              </Select>
            </FormControl>
          )}

          {transport.kind === "ws" && (
            <Stack spacing={2}>
              <TextField
                size="small"
                label="路径"
                value={transport.path ?? ""}
                onChange={(event) => onChange({ ...transport, path: event.target.value })}
              />
              <TextField
                size="small"
                label="Host"
                value={transport.host ?? ""}
                onChange={(event) => onChange({ ...transport, host: event.target.value })}
              />
              <TextField
                size="small"
                type="number"
                label="Early Data 字节"
                value={transport.max_early_data ?? ""}
                onChange={(event) =>
                  onChange({
                    ...transport,
                    max_early_data: event.target.value === "" ? null : Number(event.target.value),
                  })
                }
                helperText="常见值 2048，留空关闭"
              />
              <TextField
                size="small"
                label="Early Data 头名"
                value={transport.early_data_header_name ?? ""}
                onChange={(event) =>
                  onChange({ ...transport, early_data_header_name: event.target.value })
                }
                helperText="Xray 默认 Sec-WebSocket-Protocol"
              />
            </Stack>
          )}

          {transport.kind === "grpc" && (
            <Stack spacing={2}>
              <TextField
                size="small"
                label="Service Name"
                value={transport.service_name ?? ""}
                onChange={(event) => onChange({ ...transport, service_name: event.target.value })}
              />
              <TextField
                size="small"
                label="模式"
                value={transport.mode ?? ""}
                onChange={(event) => onChange({ ...transport, mode: event.target.value })}
                helperText="gun / multi / guna，留空为默认"
              />
              <TextField
                size="small"
                label="Authority"
                value={transport.authority ?? ""}
                onChange={(event) => onChange({ ...transport, authority: event.target.value })}
              />
            </Stack>
          )}

          {transport.kind === "http" && (
            <Stack spacing={2}>
              <TextField
                size="small"
                label="Host 列表"
                value={transport.host.join(",")}
                onChange={(event) =>
                  onChange({
                    ...transport,
                    host: event.target.value
                      .split(",")
                      .map((item) => item.trim())
                      .filter((item) => item.length > 0),
                  })
                }
                helperText="逗号分隔"
              />
              <TextField
                size="small"
                label="路径"
                value={transport.path ?? ""}
                onChange={(event) => onChange({ ...transport, path: event.target.value })}
              />
              <TextField
                size="small"
                label="方法"
                value={transport.method ?? ""}
                onChange={(event) => onChange({ ...transport, method: event.target.value })}
                helperText="留空为默认 GET"
              />
            </Stack>
          )}

          {transport.kind === "httpupgrade" && (
            <Stack spacing={2}>
              <TextField
                size="small"
                label="路径"
                value={transport.path ?? ""}
                onChange={(event) => onChange({ ...transport, path: event.target.value })}
              />
              <TextField
                size="small"
                label="Host"
                value={transport.host ?? ""}
                onChange={(event) => onChange({ ...transport, host: event.target.value })}
              />
            </Stack>
          )}

          {transport.kind === "xhttp" && <XhttpEditor transport={transport} onChange={onChange} />}

          <Stack direction="row" spacing={3} sx={{ alignItems: "center", flexWrap: "wrap" }}>
            <FormControlLabel
              control={
                <Switch
                  checked={mux.enabled}
                  onChange={(event) => onMuxChange({ ...mux, enabled: event.target.checked })}
                />
              }
              label="启用多路复用"
            />
            {mux.enabled && (
              <>
                <FormControl size="small" sx={{ minWidth: 140 }}>
                  <InputLabel>协议</InputLabel>
                  <Select
                    label="协议"
                    value={mux.protocol}
                    onChange={(event) =>
                      onMuxChange({
                        ...mux,
                        protocol: event.target.value as Mux["protocol"],
                      })
                    }
                  >
                    <MenuItem value="smux">smux</MenuItem>
                    <MenuItem value="yamux">yamux</MenuItem>
                    <MenuItem value="h2mux">h2mux</MenuItem>
                  </Select>
                </FormControl>
                <TextField
                  size="small"
                  type="number"
                  label="最大并发流"
                  value={mux.max_streams ?? ""}
                  sx={{ width: 140 }}
                  onChange={(event) =>
                    onMuxChange({
                      ...mux,
                      max_streams: event.target.value === "" ? null : Number(event.target.value),
                    })
                  }
                />
                <FormControlLabel
                  control={
                    <Switch
                      checked={mux.padding}
                      onChange={(event) => onMuxChange({ ...mux, padding: event.target.checked })}
                    />
                  }
                  label="填充"
                />
              </>
            )}
          </Stack>
        </Stack>
      </AccordionDetails>
    </Accordion>
  );
}

function XhttpEditor({
  transport,
  onChange,
}: {
  transport: Extract<Transport, { kind: "xhttp" }>;
  onChange: (next: Transport) => void;
}) {
  const extra = transport.extra ?? {};

  function patchExtra(next: Partial<NonNullable<typeof transport.extra>>) {
    onChange({ ...transport, extra: { ...extra, ...next } });
  }

  const xmux = extra.xmux ?? {};

  function patchXmux(next: Partial<NonNullable<typeof extra.xmux>>) {
    patchExtra({ xmux: { ...xmux, ...next } });
  }

  return (
    <Stack spacing={2}>
      <TextField
        size="small"
        label="路径"
        value={transport.path ?? ""}
        onChange={(event) => onChange({ ...transport, path: event.target.value })}
      />
      <TextField
        size="small"
        label="Host"
        value={transport.host ?? ""}
        onChange={(event) => onChange({ ...transport, host: event.target.value })}
      />
      <FormControl size="small" fullWidth>
        <InputLabel>模式</InputLabel>
        <Select
          label="模式"
          value={transport.mode}
          onChange={(event) => onChange({ ...transport, mode: event.target.value as XhttpMode })}
        >
          <MenuItem value="auto">auto</MenuItem>
          <MenuItem value="stream-one">stream-one</MenuItem>
          <MenuItem value="stream-up">stream-up</MenuItem>
          <MenuItem value="packet-up">packet-up</MenuItem>
        </Select>
      </FormControl>

      <Typography variant="subtitle2" color="text.secondary">
        extra 参数
      </Typography>
      <Typography variant="caption" color="text.secondary">
        字段名与 Xray 的 SplitHTTPConfig 一一对应，导出时自动转成 camelCase。
      </Typography>
      <Stack direction="row" spacing={2} sx={{ flexWrap: "wrap" }}>
        <TextField
          size="small"
          label="xPaddingBytes"
          value={extra.x_padding_bytes ?? ""}
          sx={{ minWidth: 200 }}
          onChange={(event) => patchExtra({ x_padding_bytes: event.target.value })}
          helperText="例如 100-1000"
        />
        <TextField
          size="small"
          label="xPaddingKey"
          value={extra.x_padding_key ?? ""}
          sx={{ minWidth: 160 }}
          onChange={(event) => patchExtra({ x_padding_key: event.target.value })}
        />
        <TextField
          size="small"
          label="xPaddingHeader"
          value={extra.x_padding_header ?? ""}
          sx={{ minWidth: 160 }}
          onChange={(event) => patchExtra({ x_padding_header: event.target.value })}
        />
        <TextField
          size="small"
          select
          label="xPaddingPlacement"
          value={extra.x_padding_placement ?? ""}
          sx={{ minWidth: 180 }}
          onChange={(event) => patchExtra({ x_padding_placement: event.target.value })}
        >
          <MenuItem value="">默认</MenuItem>
          <MenuItem value="cookie">cookie</MenuItem>
          <MenuItem value="header">header</MenuItem>
          <MenuItem value="query">query</MenuItem>
          <MenuItem value="queryInHeader">queryInHeader</MenuItem>
        </TextField>
        <TextField
          size="small"
          select
          label="xPaddingMethod"
          value={extra.x_padding_method ?? ""}
          sx={{ minWidth: 160 }}
          onChange={(event) => patchExtra({ x_padding_method: event.target.value })}
        >
          <MenuItem value="">默认</MenuItem>
          <MenuItem value="repeat-x">repeat-x</MenuItem>
          <MenuItem value="tokenish">tokenish</MenuItem>
        </TextField>
        <TextField
          size="small"
          select
          label="uplinkHTTPMethod"
          value={extra.uplink_http_method ?? ""}
          sx={{ minWidth: 160 }}
          onChange={(event) => patchExtra({ uplink_http_method: event.target.value })}
        >
          <MenuItem value="">默认</MenuItem>
          <MenuItem value="POST">POST</MenuItem>
          <MenuItem value="PUT">PUT</MenuItem>
          <MenuItem value="GET">GET</MenuItem>
        </TextField>
        <TextField
          size="small"
          label="sessionIDTable"
          value={extra.session_id_table ?? ""}
          sx={{ minWidth: 160 }}
          onChange={(event) => patchExtra({ session_id_table: event.target.value })}
        />
        <TextField
          size="small"
          label="sessionIDLength"
          value={extra.session_id_length ?? ""}
          sx={{ minWidth: 160 }}
          onChange={(event) => patchExtra({ session_id_length: event.target.value })}
        />
        <TextField
          size="small"
          label="scMaxEachPostBytes"
          value={extra.sc_max_each_post_bytes ?? ""}
          sx={{ minWidth: 200 }}
          onChange={(event) => patchExtra({ sc_max_each_post_bytes: event.target.value })}
        />
        <TextField
          size="small"
          label="scMinPostsIntervalMs"
          value={extra.sc_min_posts_interval_ms ?? ""}
          sx={{ minWidth: 200 }}
          onChange={(event) => patchExtra({ sc_min_posts_interval_ms: event.target.value })}
        />
        <TextField
          size="small"
          label="scMaxBufferedPosts"
          type="number"
          value={extra.sc_max_buffered_posts ?? ""}
          sx={{ minWidth: 180 }}
          onChange={(event) =>
            patchExtra({
              sc_max_buffered_posts: event.target.value === "" ? null : Number(event.target.value),
            })
          }
        />
        <TextField
          size="small"
          label="scStreamUpServerSecs"
          value={extra.sc_stream_up_server_secs ?? ""}
          sx={{ minWidth: 200 }}
          onChange={(event) => patchExtra({ sc_stream_up_server_secs: event.target.value })}
        />
        <TextField
          size="small"
          label="serverMaxHeaderBytes"
          type="number"
          value={extra.server_max_header_bytes ?? ""}
          sx={{ minWidth: 200 }}
          onChange={(event) =>
            patchExtra({
              server_max_header_bytes: event.target.value === "" ? null : Number(event.target.value),
            })
          }
        />
        <FormControlLabel
          control={
            <Switch
              checked={Boolean(extra.no_sse_header)}
              onChange={(event) => patchExtra({ no_sse_header: event.target.checked })}
            />
          }
          label="noSSEHeader"
        />
        <FormControlLabel
          control={
            <Switch
              checked={Boolean(extra.no_grpc_header)}
              onChange={(event) => patchExtra({ no_grpc_header: event.target.checked })}
            />
          }
          label="noGRPCHeader"
        />
        <FormControlLabel
          control={
            <Switch
              checked={Boolean(extra.x_padding_obfs_mode)}
              onChange={(event) => patchExtra({ x_padding_obfs_mode: event.target.checked })}
            />
          }
          label="xPaddingObfsMode"
        />
      </Stack>

      <Typography variant="subtitle2" color="text.secondary">
        XMUX
      </Typography>
      <Stack direction="row" spacing={2} sx={{ flexWrap: "wrap" }}>
        <TextField
          size="small"
          label="maxConcurrency"
          value={xmux.max_concurrency ?? ""}
          sx={{ minWidth: 160 }}
          onChange={(event) => patchXmux({ max_concurrency: event.target.value })}
        />
        <TextField
          size="small"
          label="maxConnections"
          value={xmux.max_connections ?? ""}
          sx={{ minWidth: 160 }}
          onChange={(event) => patchXmux({ max_connections: event.target.value })}
        />
        <TextField
          size="small"
          label="cMaxReuseTimes"
          value={xmux.c_max_reuse_times ?? ""}
          sx={{ minWidth: 160 }}
          onChange={(event) => patchXmux({ c_max_reuse_times: event.target.value })}
        />
        <TextField
          size="small"
          label="cMaxLifetimeMs"
          value={xmux.c_max_lifetime_ms ?? ""}
          sx={{ minWidth: 160 }}
          onChange={(event) => patchXmux({ c_max_lifetime_ms: event.target.value })}
        />
        <TextField
          size="small"
          label="hMaxRequestTimes"
          value={xmux.h_max_request_times ?? ""}
          sx={{ minWidth: 160 }}
          onChange={(event) => patchXmux({ h_max_request_times: event.target.value })}
        />
        <TextField
          size="small"
          label="hMaxReusableSecs"
          value={xmux.h_max_reusable_secs ?? ""}
          sx={{ minWidth: 160 }}
          onChange={(event) => patchXmux({ h_max_reusable_secs: event.target.value })}
        />
        <TextField
          size="small"
          label="hKeepAlivePeriod"
          type="number"
          value={xmux.h_keep_alive_period ?? ""}
          sx={{ minWidth: 160 }}
          onChange={(event) =>
            patchXmux({
              h_keep_alive_period: event.target.value === "" ? null : Number(event.target.value),
            })
          }
        />
      </Stack>

      <HeaderEditor
        headers={transport.headers ?? {}}
        onChange={(headers) => onChange({ ...transport, headers })}
      />
    </Stack>
  );
}

function HeaderEditor({
  headers,
  onChange,
}: {
  headers: Record<string, string>;
  onChange: (next: Record<string, string>) => void;
}) {
  const [name, setName] = useState("");
  const [value, setValue] = useState("");
  const entries = Object.entries(headers);

  function add() {
    const key = name.trim();
    if (key.length === 0) {
      return;
    }
    onChange({ ...headers, [key]: value });
    setName("");
    setValue("");
  }

  function remove(key: string) {
    const next = { ...headers };
    delete next[key];
    onChange(next);
  }

  return (
    <Stack spacing={1}>
      <Typography variant="subtitle2" color="text.secondary">
        额外请求头
      </Typography>
      {entries.map(([key, headerValue]) => (
        <Box key={key} sx={{ display: "flex", gap: 1, alignItems: "center" }}>
          <TextField size="small" label="名称" value={key} disabled sx={{ flex: 1 }} />
          <TextField size="small" label="值" value={headerValue} disabled sx={{ flex: 2 }} />
          <IconButton size="small" color="error" onClick={() => remove(key)} aria-label="删除请求头">
            ×
          </IconButton>
        </Box>
      ))}
      <Stack direction="row" spacing={1}>
        <TextField
          size="small"
          label="名称"
          value={name}
          onChange={(event) => setName(event.target.value)}
          sx={{ flex: 1 }}
        />
        <TextField
          size="small"
          label="值"
          value={value}
          onChange={(event) => setValue(event.target.value)}
          sx={{ flex: 2 }}
        />
        <Button size="small" variant="outlined" startIcon={<Add />} onClick={add}>
          添加
        </Button>
      </Stack>
    </Stack>
  );
}

