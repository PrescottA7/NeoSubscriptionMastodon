/*
 * Copyright (C) 2026 Prescott Anderson
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * This file is part of NSM (NeoSubscriptionMastodon), licensed under the
 * GNU Affero General Public License v3.0 or later. See the LICENSE file
 * in the project root for the full text.
 */

import { Box, Stack, TextField } from "@mui/material";
import { useTranslation } from "react-i18next";

import { TlsSection } from "../TlsEditor";
import type { AnytlsParams } from "../../types";
import type { FieldErrors } from "../../validation";

interface AnytlsFieldsProps {
  params: AnytlsParams;
  onChange: (next: AnytlsParams) => void;
  errors?: FieldErrors;
}

export function AnytlsFields({ params, onChange, errors = {} }: AnytlsFieldsProps) {
  const { t } = useTranslation();
  const field = (name: string) => ({
    error: Boolean(errors[name]),
    helperText: errors[name],
  });

  return (
    <Stack spacing={2}>
      <Box sx={{ display: "grid", gridTemplateColumns: { xs: "1fr", sm: "1fr 1fr" }, gap: 2 }}>
        <TextField
          size="small"
          label={t("anytls.idleCheckInterval")}
          placeholder="30s"
          value={params.idle_session_check_interval ?? ""}
          onChange={(event) =>
            onChange({ ...params, idle_session_check_interval: event.target.value })
          }
          {...field("params.idle_session_check_interval")}
          helperText={
            field("params.idle_session_check_interval").helperText ?? t("anytls.durationHelper")
          }
        />
        <TextField
          size="small"
          label={t("anytls.idleSessionTimeout")}
          placeholder="30s"
          value={params.idle_session_timeout ?? ""}
          onChange={(event) => onChange({ ...params, idle_session_timeout: event.target.value })}
          {...field("params.idle_session_timeout")}
          helperText={
            field("params.idle_session_timeout").helperText ?? t("anytls.durationHelper")
          }
        />
        <TextField
          size="small"
          type="number"
          label={t("anytls.minIdleSession")}
          value={params.min_idle_session ?? ""}
          onChange={(event) =>
            onChange({
              ...params,
              min_idle_session: event.target.value === "" ? null : Number(event.target.value),
            })
          }
        />
        <TextField
          size="small"
          label={t("anytls.clientMetadata")}
          value={params.client_metadata ?? ""}
          onChange={(event) => onChange({ ...params, client_metadata: event.target.value })}
        />
      </Box>
      <TlsSection
        tls={params.tls}
        onChange={(tls) => onChange({ ...params, tls })}
        requireTls
        allowReality={false}
        errors={errors}
      />
    </Stack>
  );
}
