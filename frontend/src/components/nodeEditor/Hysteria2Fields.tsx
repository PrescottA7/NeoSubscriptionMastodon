/*
 * Copyright (C) 2026 Prescott Anderson
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * This file is part of NSM (NeoSubscriptionMastodon), licensed under the
 * GNU Affero General Public License v3.0 or later. See the LICENSE file
 * in the project root for the full text.
 */

import {
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
import type { Hysteria2Params } from "../../types";
import type { FieldErrors } from "../../validation";

interface Hysteria2FieldsProps {
  params: Hysteria2Params;
  onChange: (next: Hysteria2Params) => void;
  errors?: FieldErrors;
}

export function Hysteria2Fields({ params, onChange, errors = {} }: Hysteria2FieldsProps) {
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
          label={t("hysteria2.serverPorts")}
          value={params.server_ports.join(",")}
          onChange={(event) =>
            onChange({
              ...params,
              server_ports: event.target.value
                .split(",")
                .map((item) => item.trim())
                .filter((item) => item.length > 0),
            })
          }
          {...field("params.server_ports")}
          helperText={field("params.server_ports").helperText ?? t("hysteria2.serverPortsHelper")}
        />
        <TextField
          size="small"
          label={t("hysteria2.hopInterval")}
          placeholder="30s"
          value={params.hop_interval ?? ""}
          onChange={(event) => onChange({ ...params, hop_interval: event.target.value })}
          {...field("params.hop_interval")}
          helperText={field("params.hop_interval").helperText ?? t("hysteria2.hopIntervalHelper")}
        />
        <TextField
          size="small"
          label={t("hysteria2.maxHopInterval")}
          placeholder="45s"
          value={params.hop_interval_max ?? ""}
          onChange={(event) => onChange({ ...params, hop_interval_max: event.target.value })}
          {...field("params.hop_interval_max")}
          helperText={
            field("params.hop_interval_max").helperText ?? t("hysteria2.maxHopIntervalHelper")
          }
        />
        <TextField
          size="small"
          label={t("hysteria2.bbrProfile")}
          placeholder="standard"
          value={params.bbr_profile ?? ""}
          onChange={(event) => onChange({ ...params, bbr_profile: event.target.value })}
        />
        <TextField
          size="small"
          type="number"
          label={`${t("hysteria2.up")} Mbps`}
          value={params.up_mbps ?? ""}
          onChange={(event) =>
            onChange({
              ...params,
              up_mbps: event.target.value === "" ? null : Number(event.target.value),
            })
          }
        />
        <TextField
          size="small"
          type="number"
          label={`${t("hysteria2.down")} Mbps`}
          value={params.down_mbps ?? ""}
          onChange={(event) =>
            onChange({
              ...params,
              down_mbps: event.target.value === "" ? null : Number(event.target.value),
            })
          }
        />
      </Box>

      <Stack direction="row" spacing={3} sx={{ flexWrap: "wrap" }}>
        <FormControlLabel
          control={
            <Switch
              checked={Boolean(params.obfs)}
              onChange={(event) =>
                onChange({
                  ...params,
                  obfs: event.target.checked
                    ? {
                        obfs_type: "salamander",
                        password: "",
                        min_packet_size: null,
                        max_packet_size: null,
                      }
                    : null,
                })
              }
            />
          }
          label={t("hysteria2.enableObfs")}
        />
        <FormControlLabel
          control={
            <Switch
              checked={params.brutal_debug}
              onChange={(event) => onChange({ ...params, brutal_debug: event.target.checked })}
            />
          }
          label={t("hysteria2.brutalDebug")}
        />
        <FormControlLabel
          control={
            <Switch
              checked={params.disable_chrome_parrot}
              onChange={(event) =>
                onChange({ ...params, disable_chrome_parrot: event.target.checked })
              }
            />
          }
          label={t("hysteria2.disableChromeParrot")}
        />
      </Stack>

      {params.obfs && (
        <Box sx={{ display: "grid", gridTemplateColumns: { xs: "1fr", sm: "1fr 1fr" }, gap: 2 }}>
          <FormControl size="small">
            <InputLabel>{t("hysteria2.obfsType")}</InputLabel>
            <Select
              label={t("hysteria2.obfsType")}
              value={params.obfs.obfs_type}
              onChange={(event) =>
                onChange({
                  ...params,
                  obfs: {
                    ...params.obfs!,
                    obfs_type: event.target.value as "salamander" | "gecko",
                  },
                })
              }
            >
              <MenuItem value="salamander">salamander</MenuItem>
              <MenuItem value="gecko">gecko</MenuItem>
            </Select>
          </FormControl>
          <TextField
            size="small"
            label={t("hysteria2.obfsPassword")}
            value={params.obfs.password}
            onChange={(event) =>
              onChange({ ...params, obfs: { ...params.obfs!, password: event.target.value } })
            }
            {...field("params.obfs.password")}
          />
          <TextField
            size="small"
            type="number"
            label={t("hysteria2.minPacketSize")}
            value={params.obfs.min_packet_size ?? ""}
            onChange={(event) =>
              onChange({
                ...params,
                obfs: {
                  ...params.obfs!,
                  min_packet_size: event.target.value === "" ? null : Number(event.target.value),
                },
              })
            }
            helperText={t("hysteria2.geckoOnly")}
          />
          <TextField
            size="small"
            type="number"
            label={t("hysteria2.maxPacketSize")}
            value={params.obfs.max_packet_size ?? ""}
            onChange={(event) =>
              onChange({
                ...params,
                obfs: {
                  ...params.obfs!,
                  max_packet_size: event.target.value === "" ? null : Number(event.target.value),
                },
              })
            }
            {...field("params.obfs.max_packet_size")}
            helperText={
              field("params.obfs.max_packet_size").helperText ?? t("hysteria2.geckoOnly")
            }
          />
        </Box>
      )}

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
