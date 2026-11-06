/*
 * Copyright (C) 2026 Prescott Anderson
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * This file is part of NSM (NeoSubscriptionMastodon), licensed under the
 * GNU Affero General Public License v3.0 or later. See the LICENSE file
 * in the project root for the full text.
 */

import { Alert, Box, MenuItem, Stack, TextField } from "@mui/material";
import { useTranslation } from "react-i18next";

import { TlsSection } from "../TlsEditor";
import { TransportSection } from "../TransportEditor";
import type { VlessParams } from "../../types";
import type { FieldErrors } from "../../validation";

interface VlessFieldsProps {
  params: VlessParams;
  onChange: (next: VlessParams) => void;
  errors?: FieldErrors;
}

export function VlessFields({ params, onChange, errors = {} }: VlessFieldsProps) {
  const { t } = useTranslation();
  const field = (name: string) => ({
    error: Boolean(errors[name]),
    helperText: errors[name],
  });

  const tlsError = errors["params.tls.enabled"];

  return (
    <Stack spacing={2}>
      {tlsError && <Alert severity="error">{tlsError}</Alert>}
      <Box sx={{ display: "grid", gridTemplateColumns: { xs: "1fr", sm: "1fr 1fr" }, gap: 2 }}>
        <TextField
          select
          size="small"
          label={t("protocol.flow")}
          value={params.flow ?? ""}
          onChange={(event) => onChange({ ...params, flow: event.target.value })}
          {...field("params.flow")}
          helperText={field("params.flow").helperText ?? t("protocol.flowHelper")}
        >
          <MenuItem value="">{t("protocol.flowNone")}</MenuItem>
          <MenuItem value="xtls-rprx-vision">xtls-rprx-vision</MenuItem>
        </TextField>
        <TextField
          select
          size="small"
          label={t("protocol.packetEncoding")}
          value={params.packet_encoding ?? ""}
          onChange={(event) => onChange({ ...params, packet_encoding: event.target.value })}
        >
          <MenuItem value="">{t("protocol.default")}</MenuItem>
          <MenuItem value="xudp">xudp</MenuItem>
          <MenuItem value="packetaddr">packetaddr</MenuItem>
        </TextField>
        <TextField
          size="small"
          label={t("protocol.vlessEncryption")}
          value={params.encryption ?? ""}
          onChange={(event) => onChange({ ...params, encryption: event.target.value })}
          helperText={t("protocol.vlessEncryptionHelper")}
        />
      </Box>
      <TlsSection
        tls={params.tls}
        onChange={(tls) => onChange({ ...params, tls })}
        errors={errors}
      />
      <TransportSection
        transport={params.transport}
        onChange={(transport) => onChange({ ...params, transport })}
        mux={params.mux}
        onMuxChange={(mux) => onChange({ ...params, mux })}
      />
    </Stack>
  );
}
