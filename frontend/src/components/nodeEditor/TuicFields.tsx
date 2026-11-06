/*
 * Copyright (C) 2026 Prescott Anderson
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * This file is part of NSM (NeoSubscriptionMastodon), licensed under the
 * GNU Affero General Public License v3.0 or later. See the LICENSE file
 * in the project root for the full text.
 */

import {
  Alert,
  Box,
  FormControl,
  FormControlLabel,
  InputLabel,
  MenuItem,
  Select,
  Stack,
  Switch,
  TextField,
} from "@mui/material";
import { useTranslation } from "react-i18next";

import { TlsSection, QuicSection } from "../TlsEditor";
import type { TuicParams } from "../../types";
import type { FieldErrors } from "../../validation";

interface TuicFieldsProps {
  params: TuicParams;
  onChange: (next: TuicParams) => void;
  errors?: FieldErrors;
}

export function TuicFields({ params, onChange, errors = {} }: TuicFieldsProps) {
  const { t } = useTranslation();
  const field = (name: string) => ({
    error: Boolean(errors[name]),
    helperText: errors[name],
  });

  const sectionErrors = [errors["params.tls.enabled"], errors["params.udp_relay_mode"]].filter(
    (message): message is string => Boolean(message),
  );

  return (
    <Stack spacing={2}>
      {sectionErrors.map((message) => (
        <Alert key={message} severity="error">
          {message}
        </Alert>
      ))}
      <Box sx={{ display: "grid", gridTemplateColumns: { xs: "1fr", sm: "1fr 1fr" }, gap: 2 }}>
        <FormControl size="small">
          <InputLabel>{t("tuic.congestionControl")}</InputLabel>
          <Select
            label={t("tuic.congestionControl")}
            value={params.congestion_control}
            onChange={(event) =>
              onChange({
                ...params,
                congestion_control: event.target.value as typeof params.congestion_control,
              })
            }
          >
            <MenuItem value="cubic">cubic</MenuItem>
            <MenuItem value="new_reno">new_reno</MenuItem>
            <MenuItem value="bbr">bbr</MenuItem>
          </Select>
        </FormControl>
        <FormControl size="small">
          <InputLabel>{t("tuic.udpRelayMode")}</InputLabel>
          <Select
            label={t("tuic.udpRelayMode")}
            value={params.udp_relay_mode}
            onChange={(event) =>
              onChange({
                ...params,
                udp_relay_mode: event.target.value as typeof params.udp_relay_mode,
              })
            }
          >
            <MenuItem value="native">native</MenuItem>
            <MenuItem value="quic">quic</MenuItem>
          </Select>
        </FormControl>
        <TextField
          size="small"
          label={t("tuic.heartbeat")}
          placeholder="10s"
          value={params.heartbeat ?? ""}
          onChange={(event) => onChange({ ...params, heartbeat: event.target.value })}
          {...field("params.heartbeat")}
          helperText={field("params.heartbeat").helperText ?? t("tuic.heartbeatHelper")}
        />
      </Box>
      <Stack direction="row" spacing={3} sx={{ flexWrap: "wrap" }}>
        <FormControlLabel
          control={
            <Switch
              checked={params.udp_over_stream}
              onChange={(event) =>
                onChange({ ...params, udp_over_stream: event.target.checked })
              }
            />
          }
          label={t("tuic.udpOverStream")}
        />
        <FormControlLabel
          control={
            <Switch
              checked={params.zero_rtt_handshake}
              onChange={(event) =>
                onChange({ ...params, zero_rtt_handshake: event.target.checked })
              }
            />
          }
          label={t("tuic.zeroRttHandshake")}
        />
      </Stack>
      <TlsSection
        tls={params.tls}
        onChange={(tls) => onChange({ ...params, tls })}
        requireTls
        allowReality={false}
        errors={errors}
      />
      <QuicSection quic={params.quic} onChange={(quic) => onChange({ ...params, quic })} />
    </Stack>
  );
}
