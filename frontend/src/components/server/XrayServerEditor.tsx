/*
 * Copyright (C) 2026 Prescott Anderson
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * This file is part of NSM (NeoSubscriptionMastodon), licensed under the
 * GNU Affero General Public License v3.0 or later. See the LICENSE file
 * in the project root for the full text.
 */

import {
  Accordion,
  AccordionDetails,
  AccordionSummary,
  Alert,
  Box,
  Button,
  FormControlLabel,
  MenuItem,
  Stack,
  Switch,
  TextField,
  Typography,
} from "@mui/material";
import { Add, Delete, ExpandMore } from "@mui/icons-material";
import { useTranslation } from "react-i18next";

import type { ServerConfig } from "../../types";
import type { FieldErrors } from "../../validation";
import { XRAY_TRANSPORTS } from "./defaults";

/**
 * Xray inbound settings.
 *
 * Everything here exists in an Xray `inbounds[]` entry: the listener, the stream
 * security (plain TLS or REALITY), fallbacks for non-proxy traffic, and socket
 * options. Hysteria2 has no counterpart for fallbacks or REALITY, which is why
 * this lives in its own component instead of a shared form with `if`s.
 */
export function XrayServerEditor({
  config,
  onChange,
  errors = {},
}: {
  config: ServerConfig;
  onChange: (next: ServerConfig) => void;
  errors?: FieldErrors;
}) {
  const { t } = useTranslation();

  const field = (name: string) => ({
    error: Boolean(errors[name]),
    helperText: errors[name],
  });

  function patch(next: Partial<ServerConfig>) {
    onChange({ ...config, ...next });
  }

  function patchInbound(next: Partial<ServerConfig["inbound"]>) {
    patch({ inbound: { ...config.inbound, ...next } });
  }

  function patchReality(next: Partial<ServerConfig["reality"]>) {
    patch({ reality: { ...config.reality, ...next } });
  }

  function patchTls(next: Partial<ServerConfig["tls"]>) {
    patch({ tls: { ...config.tls, ...next } });
  }

  const reality = config.reality;

  return (
    <Stack spacing={2}>
      <Accordion defaultExpanded disableGutters>
        <AccordionSummary expandIcon={<ExpandMore />}>
          <Typography sx={{ flexGrow: 1 }}>{t("server.listen")}</Typography>
          <Typography variant="caption" color="text.secondary">
            {config.inbound.listen}:{config.inbound.port}
          </Typography>
        </AccordionSummary>
        <AccordionDetails>
          <Box sx={{ display: "grid", gridTemplateColumns: { xs: "1fr", sm: "1fr 1fr" }, gap: 2 }}>
            <TextField
              size="small"
              select
              label={t("server.inboundProtocol")}
              value={config.inbound.protocol}
              onChange={(event) =>
                patchInbound({ protocol: event.target.value as ServerConfig["inbound"]["protocol"] })
              }
            >
              <MenuItem value="vless">{t("protocol.vless")}</MenuItem>
              <MenuItem value="trojan">{t("protocol.trojan")}</MenuItem>
            </TextField>
            <TextField
              size="small"
              label={t("server.listenAddress")}
              value={config.inbound.listen}
              onChange={(event) => patchInbound({ listen: event.target.value })}
              {...field("server_params.inbound.listen")}
              helperText={
                field("server_params.inbound.listen").helperText ??
                t("server.listenAddressHelper")
              }
            />
            <TextField
              size="small"
              type="number"
              label={t("server.listenPort")}
              value={config.inbound.port}
              onChange={(event) => patchInbound({ port: Number(event.target.value) })}
              {...field("server_params.inbound.port")}
            />
            <TextField
              size="small"
              label={t("server.inboundTag")}
              value={config.inbound.tag}
              onChange={(event) => patchInbound({ tag: event.target.value })}
              {...field("server_params.inbound.tag")}
              helperText={
                field("server_params.inbound.tag").helperText ?? t("server.inboundTagHelper")
              }
            />
            <TextField
              size="small"
              select
              label={t("server.transportMethod")}
              value={config.inbound.transport}
              onChange={(event) => patchInbound({ transport: event.target.value })}
            >
              {XRAY_TRANSPORTS.map((value) => (
                <MenuItem key={value} value={value}>
                  {t(`transport.${value}`)}
                </MenuItem>
              ))}
            </TextField>
            {["ws", "httpupgrade", "xhttp"].includes(config.inbound.transport) && (
              <TextField
                size="small"
                label={t("server.path")}
                value={config.inbound.path}
                onChange={(event) => patchInbound({ path: event.target.value })}
              />
            )}
            {config.inbound.transport === "grpc" && (
              <TextField
                size="small"
                label={t("server.grpcServiceName")}
                value={config.inbound.service_name}
                onChange={(event) => patchInbound({ service_name: event.target.value })}
              />
            )}
            <FormControlLabel
              control={
                <Switch
                  checked={config.inbound.sniffing}
                  onChange={(event) => patchInbound({ sniffing: event.target.checked })}
                />
              }
              label={t("server.sniffing")}
            />
          </Box>
        </AccordionDetails>
      </Accordion>

      <Accordion disableGutters>
        <AccordionSummary expandIcon={<ExpandMore />}>
          <Typography sx={{ flexGrow: 1 }}>{t("server.certificate")}</Typography>
          <Typography variant="caption" color="text.secondary">
            {config.tls.certificate_file ? t("server.configured") : t("server.notConfigured")}
          </Typography>
        </AccordionSummary>
        <AccordionDetails>
          <Stack spacing={2}>
            <Alert severity="info">{t("server.certificateInfo")}</Alert>
            <TextField
              size="small"
              label={t("server.certificateFile")}
              value={config.tls.certificate_file}
              onChange={(event) => patchTls({ certificate_file: event.target.value })}
              helperText={t("server.certificateFileHelper")}
            />
            <TextField
              size="small"
              label={t("server.keyFile")}
              value={config.tls.key_file}
              onChange={(event) => patchTls({ key_file: event.target.value })}
              helperText={t("server.keyFileHelper")}
            />
            <TextField
              size="small"
              label={t("server.alpn")}
              value={config.tls.alpn.join(",")}
              onChange={(event) =>
                patchTls({
                  alpn: event.target.value
                    .split(",")
                    .map((item) => item.trim())
                    .filter((item) => item.length > 0),
                })
              }
              helperText={t("server.alpnHelper")}
            />
          </Stack>
        </AccordionDetails>
      </Accordion>

      <Accordion disableGutters>
        <AccordionSummary expandIcon={<ExpandMore />}>
          <Typography sx={{ flexGrow: 1 }}>{t("server.realityServer")}</Typography>
          <Typography variant="caption" color="text.secondary">
            {reality.enabled ? t("server.enabled") : t("server.disabled")}
          </Typography>
        </AccordionSummary>
        <AccordionDetails>
          <Stack spacing={2}>
            <Alert severity="warning">{t("server.realityWarning")}</Alert>
            <FormControlLabel
              control={
                <Switch
                  checked={reality.enabled}
                  onChange={(event) => patchReality({ enabled: event.target.checked })}
                />
              }
              label={t("server.enableReality")}
            />
            {reality.enabled && (
              <Box
                sx={{ display: "grid", gridTemplateColumns: { xs: "1fr", sm: "1fr 1fr" }, gap: 2 }}
              >
                <TextField
                  size="small"
                  label={t("server.privateKey")}
                  value={reality.private_key}
                  onChange={(event) => patchReality({ private_key: event.target.value })}
                  {...field("server_params.reality.private_key")}
                  helperText={
                    field("server_params.reality.private_key").helperText ??
                    t("server.privateKeyHelper")
                  }
                  slotProps={{ htmlInput: { style: { fontFamily: "monospace" } } }}
                />
                <TextField
                  size="small"
                  label={t("server.target")}
                  value={reality.target}
                  onChange={(event) => patchReality({ target: event.target.value })}
                  helperText={t("server.targetHelper")}
                />
                <TextField
                  size="small"
                  label={t("server.serverNames")}
                  value={reality.server_names.join(",")}
                  onChange={(event) =>
                    patchReality({
                      server_names: event.target.value
                        .split(",")
                        .map((item) => item.trim())
                        .filter((item) => item.length > 0),
                    })
                  }
                  helperText={t("server.serverNamesHelper")}
                />
                <TextField
                  size="small"
                  label={t("server.shortIds")}
                  value={reality.short_ids.join(",")}
                  onChange={(event) =>
                    patchReality({
                      short_ids: event.target.value
                        .split(",")
                        .map((item) => item.trim())
                        .filter((item) => item.length > 0),
                    })
                  }
                  helperText={t("server.shortIdsHelper")}
                />
                <TextField
                  size="small"
                  label={t("server.maxTimeDiff")}
                  type="number"
                  value={reality.max_time_diff}
                  onChange={(event) => patchReality({ max_time_diff: Number(event.target.value) })}
                  helperText={t("server.maxTimeDiffHelper")}
                />
                <TextField
                  size="small"
                  label={t("server.mldsa65Seed")}
                  value={reality.mldsa65_seed}
                  onChange={(event) => patchReality({ mldsa65_seed: event.target.value })}
                  helperText={t("server.mldsa65SeedHelper")}
                />
              </Box>
            )}
          </Stack>
        </AccordionDetails>
      </Accordion>

      <Accordion disableGutters>
        <AccordionSummary expandIcon={<ExpandMore />}>
          <Typography sx={{ flexGrow: 1 }}>{t("server.fallbacks")}</Typography>
          <Typography variant="caption" color="text.secondary">
            {t("server.count", { count: config.fallbacks.length })}
          </Typography>
        </AccordionSummary>
        <AccordionDetails>
          <Stack spacing={1.5}>
            {config.fallbacks.map((fallback, index) => (
              <Box
                key={index}
                sx={{
                  display: "grid",
                  gridTemplateColumns: { xs: "1fr", sm: "1fr 1fr auto" },
                  gap: 1.5,
                  alignItems: "start",
                }}
              >
                <TextField
                  size="small"
                  label={t("server.dest")}
                  value={fallback.dest}
                  onChange={(event) =>
                    patch({
                      fallbacks: config.fallbacks.map((item, position) =>
                        position === index ? { ...item, dest: event.target.value } : item,
                      ),
                    })
                  }
                  helperText={t("server.destHelper")}
                />
                <TextField
                  size="small"
                  label={t("server.alpnNamePath")}
                  value={fallback.alpn || fallback.name || fallback.path}
                  onChange={(event) =>
                    patch({
                      fallbacks: config.fallbacks.map((item, position) =>
                        position === index ? { ...item, alpn: event.target.value } : item,
                      ),
                    })
                  }
                  helperText={t("server.alpnNamePathHelper")}
                />
                <Button
                  size="small"
                  color="error"
                  aria-label={t("server.removeFallback")}
                  onClick={() =>
                    patch({
                      fallbacks: config.fallbacks.filter((_, position) => position !== index),
                    })
                  }
                >
                  <Delete fontSize="small" />
                </Button>
              </Box>
            ))}
            <Button
              size="small"
              startIcon={<Add />}
              onClick={() =>
                patch({
                  fallbacks: [...config.fallbacks, { dest: "", alpn: "", name: "", path: "" }],
                })
              }
            >
              {t("server.addFallback")}
            </Button>
          </Stack>
        </AccordionDetails>
      </Accordion>
    </Stack>
  );
}
