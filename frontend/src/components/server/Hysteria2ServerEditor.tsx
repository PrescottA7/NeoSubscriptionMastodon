/*
 * Copyright (C) 2026 Prescott Anderson
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * This file is part of NSM (NeoSubscriptionMastodon), licensed under the
 * GNU Affero General Public License v3.0 or later. See the LICENSE file
 * in the project root for the full text.
 */

import { Alert, Box, Stack, TextField, Typography } from "@mui/material";
import { useTranslation } from "react-i18next";

import type { ServerConfig } from "../../types";
import type { FieldErrors } from "../../validation";

/**
 * Hysteria2 inbound settings.
 *
 * The Hysteria2 server config is much smaller than Xray's: a `listen` address, a
 * mandatory TLS certificate, and the account table. It has no transports, no
 * sniffing, no fallbacks, and — importantly — no REALITY, so none of those are
 * offered here.
 */
export function Hysteria2ServerEditor({
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

  function patchInbound(next: Partial<ServerConfig["inbound"]>) {
    onChange({ ...config, inbound: { ...config.inbound, ...next } });
  }

  function patchTls(next: Partial<ServerConfig["tls"]>) {
    onChange({ ...config, tls: { ...config.tls, ...next } });
  }

  return (
    <Stack spacing={2}>
      <Typography variant="subtitle2" color="text.secondary">
        {t("server.listen")}
      </Typography>
      <Box sx={{ display: "grid", gridTemplateColumns: { xs: "1fr", sm: "1fr 1fr" }, gap: 2 }}>
        <TextField
          size="small"
          label={t("server.listenAddress")}
          value={config.inbound.listen}
          onChange={(event) => patchInbound({ listen: event.target.value })}
          {...field("server_params.inbound.listen")}
          helperText={
            field("server_params.inbound.listen").helperText ?? t("server.listenAddressHelper")
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
      </Box>

      <Typography variant="subtitle2" color="text.secondary">
        {t("server.certificate")}
      </Typography>
      <Alert severity="info">{t("server.certificateRequired")}</Alert>
      <Stack spacing={2}>
        <TextField
          size="small"
          label={t("server.certificateFile")}
          value={config.tls.certificate_file}
          onChange={(event) => patchTls({ certificate_file: event.target.value })}
          {...field("server_params.tls.certificate_file")}
          helperText={
            field("server_params.tls.certificate_file").helperText ??
            t("server.certificateFileHelper")
          }
        />
        <TextField
          size="small"
          label={t("server.keyFile")}
          value={config.tls.key_file}
          onChange={(event) => patchTls({ key_file: event.target.value })}
          helperText={t("server.keyFileHelper")}
        />
      </Stack>
    </Stack>
  );
}
