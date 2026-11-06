/*
 * Copyright (C) 2026 Prescott Anderson
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * This file is part of NSM (NeoSubscriptionMastodon), licensed under the
 * GNU Affero General Public License v3.0 or later. See the LICENSE file
 * in the project root for the full text.
 */

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
import { useTranslation } from "react-i18next";

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
  const { t } = useTranslation();
  
  return (
    <Accordion disableGutters>
      <AccordionSummary expandIcon={<ExpandMore />}>
        <Typography sx={{ flexGrow: 1 }}>{t("transport.title")}</Typography>
        <Typography variant="caption" color="text.secondary">
          {TRANSPORTS.find((item) => item.value === transport.kind)
            ? t(TRANSPORTS.find((item) => item.value === transport.kind)!.labelKey)
            : transport.kind}
        </Typography>
      </AccordionSummary>
      <AccordionDetails>
        <Stack spacing={2}>
          <FormControl size="small" fullWidth>
            <InputLabel>{t("transport.transportMethod")}</InputLabel>
            <Select
              label={t("transport.transportMethod")}
              value={transport.kind}
              onChange={(event) =>
                onChange(defaultTransport(event.target.value as Transport["kind"]))
              }
            >
              {TRANSPORTS.map((item) => (
                <MenuItem key={item.value} value={item.value}>
                  {t(item.labelKey)}
                </MenuItem>
              ))}
            </Select>
          </FormControl>

          {transport.kind === "tcp" && (
            <FormControl size="small" fullWidth>
              <InputLabel>{t("transport.header")}</InputLabel>
              <Select
                label={t("transport.header")}
                value={transport.header_type}
                onChange={(event) =>
                  onChange({ ...transport, header_type: event.target.value as "none" | "http" })
                }
              >
                <MenuItem value="none">{t("transport.none")}</MenuItem>
                <MenuItem value="http">HTTP</MenuItem>
              </Select>
            </FormControl>
          )}

          {transport.kind === "ws" && (
            <Stack spacing={2}>
              <TextField
                size="small"
                label={t("transport.path")}
                value={transport.path ?? ""}
                onChange={(event) => onChange({ ...transport, path: event.target.value })}
              />
              <TextField
                size="small"
                label={t("transport.host")}
                value={transport.host ?? ""}
                onChange={(event) => onChange({ ...transport, host: event.target.value })}
              />
              <TextField
                size="small"
                type="number"
                label={t("transport.earlyData")}
                value={transport.max_early_data ?? ""}
                onChange={(event) =>
                  onChange({
                    ...transport,
                    max_early_data: event.target.value === "" ? null : Number(event.target.value),
                  })
                }
                helperText={t("transport.earlyDataHelper")}
              />
              <TextField
                size="small"
                label={t("transport.earlyDataHeaderName")}
                value={transport.early_data_header_name ?? ""}
                onChange={(event) =>
                  onChange({ ...transport, early_data_header_name: event.target.value })
                }
                helperText={t("transport.earlyDataHeaderHelper")}
              />
            </Stack>
          )}

          {transport.kind === "grpc" && (
            <Stack spacing={2}>
              <TextField
                size="small"
                label={t("transport.serviceName")}
                value={transport.service_name ?? ""}
                onChange={(event) => onChange({ ...transport, service_name: event.target.value })}
              />
              <TextField
                size="small"
                label={t("transport.mode")}
                value={transport.mode ?? ""}
                onChange={(event) => onChange({ ...transport, mode: event.target.value })}
                helperText={t("transport.modeHelper")}
              />
              <TextField
                size="small"
                label={t("transport.authority")}
                value={transport.authority ?? ""}
                onChange={(event) => onChange({ ...transport, authority: event.target.value })}
              />
            </Stack>
          )}

          {transport.kind === "http" && (
            <Stack spacing={2}>
              <TextField
                size="small"
                label={t("transport.hostList")}
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
                helperText={t("transport.hostListHelper")}
              />
              <TextField
                size="small"
                label={t("transport.path")}
                value={transport.path ?? ""}
                onChange={(event) => onChange({ ...transport, path: event.target.value })}
              />
              <TextField
                size="small"
                label={t("transport.method")}
                value={transport.method ?? ""}
                onChange={(event) => onChange({ ...transport, method: event.target.value })}
                helperText={t("transport.methodHelper")}
              />
            </Stack>
          )}

          {transport.kind === "httpupgrade" && (
            <Stack spacing={2}>
              <TextField
                size="small"
                label={t("transport.path")}
                value={transport.path ?? ""}
                onChange={(event) => onChange({ ...transport, path: event.target.value })}
              />
              <TextField
                size="small"
                label={t("transport.host")}
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
              label={t("transport.enableMux")}
            />
            {mux.enabled && (
              <>
                <FormControl size="small" sx={{ minWidth: 140 }}>
                  <InputLabel>{t("transport.protocol")}</InputLabel>
                  <Select
                    label={t("transport.protocol")}
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
                  label={t("transport.maxStreams")}
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
                  label={t("transport.padding")}
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
  const { t } = useTranslation();
  
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
        label={t("transport.path")}
        value={transport.path ?? ""}
        onChange={(event) => onChange({ ...transport, path: event.target.value })}
      />
      <TextField
        size="small"
        label={t("transport.host")}
        value={transport.host ?? ""}
        onChange={(event) => onChange({ ...transport, host: event.target.value })}
      />
      <FormControl size="small" fullWidth>
        <InputLabel>{t("transport.xhttpMode")}</InputLabel>
        <Select
          label={t("transport.xhttpMode")}
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
        {t("transport.xhttpExtra")}
      </Typography>
      <Typography variant="caption" color="text.secondary">
        {t("transport.xhttpExtraHelper")}
      </Typography>
      <Stack direction="row" spacing={2} sx={{ flexWrap: "wrap" }}>
        <TextField
          size="small"
          label="xPaddingBytes"
          value={extra.x_padding_bytes ?? ""}
          sx={{ minWidth: 200 }}
          onChange={(event) => patchExtra({ x_padding_bytes: event.target.value })}
          helperText={t("transport.paddingBytesHelper")}
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
          <MenuItem value="">{t("transport.defaultOption")}</MenuItem>
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
          <MenuItem value="">{t("transport.defaultOption")}</MenuItem>
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
          <MenuItem value="">{t("transport.defaultOption")}</MenuItem>
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
        {t("transport.xmux")}
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
  const { t } = useTranslation();
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
        {t("transport.extraHeaders")}
      </Typography>
      {entries.map(([key, headerValue]) => (
        <Box key={key} sx={{ display: "flex", gap: 1, alignItems: "center" }}>
          <TextField size="small" label={t("transport.headerName")} value={key} disabled sx={{ flex: 1 }} />
          <TextField size="small" label={t("transport.headerValue")} value={headerValue} disabled sx={{ flex: 2 }} />
          <IconButton
            size="small"
            color="error"
            onClick={() => remove(key)}
            aria-label={t("transport.removeHeader")}
          >
            ×
          </IconButton>
        </Box>
      ))}
      <Stack direction="row" spacing={1}>
        <TextField
          size="small"
          label={t("transport.headerName")}
          value={name}
          onChange={(event) => setName(event.target.value)}
          sx={{ flex: 1 }}
        />
        <TextField
          size="small"
          label={t("transport.headerValue")}
          value={value}
          onChange={(event) => setValue(event.target.value)}
          sx={{ flex: 2 }}
        />
        <Button size="small" variant="outlined" startIcon={<Add />} onClick={add}>
          {t("common.add")}
        </Button>
      </Stack>
    </Stack>
  );
}

