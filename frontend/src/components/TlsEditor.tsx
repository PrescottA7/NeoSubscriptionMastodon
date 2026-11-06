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
  FormControlLabel,
  MenuItem,
  Stack,
  Switch,
  TextField,
  Typography,
} from "@mui/material";
import { ExpandMore } from "@mui/icons-material";
import { useTranslation } from "react-i18next";

import { FieldRow } from "./Fields";
import type { Quic, Tls } from "../types";
import type { FieldErrors } from "../validation";

const FINGERPRINTS = [
  "chrome",
  "firefox",
  "safari",
  "ios",
  "android",
  "edge",
  "360",
  "qq",
  "random",
  "randomized",
];

const TLS_VERSIONS = ["1.0", "1.1", "1.2", "1.3"];

export function TlsSection({
  tls,
  onChange,
  requireTls = false,
  allowReality = true,
  disabledReality,
  errors = {},
}: {
  tls: Tls;
  onChange: (next: Tls) => void;
  requireTls?: boolean;
  allowReality?: boolean;
  disabledReality?: boolean;
  errors?: FieldErrors;
}) {
  const { t } = useTranslation();
  const reality = tls.reality ?? {
    enabled: false,
    public_key: "",
    short_id: "",
    spider_x: "",
    mldsa65_verify: "",
  };

  function patch(next: Partial<Tls>) {
    onChange({ ...tls, ...next });
  }

  const field = (name: string) => ({
    error: Boolean(errors[name]),
    helperText: errors[name],
  });

  return (
    <Accordion disableGutters>
      <AccordionSummary expandIcon={<ExpandMore />}>
        <Stack direction="row" spacing={2} sx={{ alignItems: "center", width: "100%" }}>
          <Typography sx={{ flexGrow: 1 }}>{t("tls.title")}</Typography>
          <Typography variant="caption" color="text.secondary">
            {tls.reality?.enabled ? "REALITY" : tls.enabled ? "TLS" : t("tls.disabled")}
          </Typography>
        </Stack>
      </AccordionSummary>
      <AccordionDetails>
        <Stack spacing={2}>
          <Stack direction="row" spacing={3} sx={{ flexWrap: "wrap" }}>
            <FormControlLabel
              control={
                <Switch
                  checked={tls.enabled}
                  disabled={requireTls}
                  onChange={(event) => patch({ enabled: event.target.checked })}
                />
              }
              label={t("tls.enableTls")}
            />
            {allowReality && (
              <FormControlLabel
                control={
                  <Switch
                    checked={Boolean(reality.enabled)}
                    disabled={disabledReality}
                    onChange={(event) =>
                      patch({
                        reality: { ...reality, enabled: event.target.checked },
                        enabled: event.target.checked ? false : tls.enabled,
                      })
                    }
                  />
                }
                label={t("tls.useReality")}
              />
            )}
            <FormControlLabel
              control={
                <Switch
                  checked={tls.insecure}
                  onChange={(event) => patch({ insecure: event.target.checked })}
                />
              }
              label={t("tls.skipVerification")}
            />
          </Stack>

          <FieldRow>
            <TextField
              size="small"
              label={t("tls.serverName")}
              value={tls.server_name ?? ""}
              onChange={(event) => patch({ server_name: event.target.value })}
              helperText={t("tls.serverNameHelper")}
            />
            <TextField
              size="small"
              select
              label={t("tls.fingerprint")}
              value={tls.fingerprint ?? "chrome"}
              onChange={(event) => patch({ fingerprint: event.target.value })}
            >
              {FINGERPRINTS.map((value) => (
                <MenuItem key={value} value={value}>
                  {value}
                </MenuItem>
              ))}
            </TextField>
            <TextField
              size="small"
              label={t("tls.alpn")}
              value={tls.alpn.join(",")}
              onChange={(event) =>
                patch({
                  alpn: event.target.value
                    .split(",")
                    .map((item) => item.trim())
                    .filter((item) => item.length > 0),
                })
              }
              helperText={t("tls.alpnHelper")}
            />
            <TextField
              size="small"
              label={t("tls.certificateSha256")}
              value={tls.certificate_sha256 ?? ""}
              onChange={(event) => patch({ certificate_sha256: event.target.value })}
              helperText={t("tls.certificateSha256Helper")}
            />
            <TextField
              size="small"
              label={t("tls.verifyPeerCertByName")}
              value={tls.verify_peer_cert_by_name ?? ""}
              onChange={(event) => patch({ verify_peer_cert_by_name: event.target.value })}
              helperText={t("tls.verifyPeerCertByNameHelper")}
            />
            <Stack direction="row" spacing={2}>
              <TextField
                size="small"
                select
                label={t("tls.minVersion")}
                value={tls.min_version ?? ""}
                onChange={(event) => patch({ min_version: event.target.value })}
                sx={{ flex: 1 }}
              >
                <MenuItem value="">{t("tls.default")}</MenuItem>
                {TLS_VERSIONS.map((value) => (
                  <MenuItem key={value} value={value}>
                    {value}
                  </MenuItem>
                ))}
              </TextField>
              <TextField
                size="small"
                select
                label={t("tls.maxVersion")}
                value={tls.max_version ?? ""}
                onChange={(event) => patch({ max_version: event.target.value })}
                sx={{ flex: 1 }}
              >
                <MenuItem value="">{t("tls.default")}</MenuItem>
                {TLS_VERSIONS.map((value) => (
                  <MenuItem key={value} value={value}>
                    {value}
                  </MenuItem>
                ))}
              </TextField>
            </Stack>
          </FieldRow>

          {allowReality && reality.enabled && (
            <Stack spacing={2}>
              <Typography variant="subtitle2" color="text.secondary">
                {t("tls.realityParams")}
              </Typography>
              <FieldRow>
                <TextField
                  size="small"
                  label={t("tls.publicKey")}
                  value={reality.public_key}
                  onChange={(event) => patch({ reality: { ...reality, public_key: event.target.value } })}
                  required
                  {...field("params.tls.reality.public_key")}
                  helperText={
                    field("params.tls.reality.public_key").helperText ??
                    t("tls.publicKeyHelper")
                  }
                />
                <TextField
                  size="small"
                  label={t("tls.shortId")}
                  value={reality.short_id ?? ""}
                  onChange={(event) => patch({ reality: { ...reality, short_id: event.target.value } })}
                  helperText={t("tls.shortIdHelper")}
                />
                <TextField
                  size="small"
                  label={t("tls.spiderX")}
                  value={reality.spider_x ?? ""}
                  onChange={(event) => patch({ reality: { ...reality, spider_x: event.target.value } })}
                  helperText={t("tls.spiderXHelper")}
                />
                <TextField
                  size="small"
                  label={t("tls.mldsa65Verify")}
                  value={reality.mldsa65_verify ?? ""}
                  onChange={(event) =>
                    patch({ reality: { ...reality, mldsa65_verify: event.target.value } })
                  }
                  helperText={t("tls.mldsa65VerifyHelper")}
                />
              </FieldRow>
            </Stack>
          )}
        </Stack>
      </AccordionDetails>
    </Accordion>
  );
}

export function QuicSection({
  quic,
  onChange,
}: {
  quic: Quic;
  onChange: (next: Quic) => void;
}) {
  const { t } = useTranslation();

  return (
    <Accordion disableGutters>
      <AccordionSummary expandIcon={<ExpandMore />}>
        <Typography sx={{ flexGrow: 1 }}>{t("quic.title")}</Typography>
      </AccordionSummary>
      <AccordionDetails>
        <FieldRow>
          <TextField
            size="small"
            type="number"
            label={t("quic.initialStreamWindow")}
            value={quic.initial_stream_receive_window ?? ""}
            onChange={(event) =>
              onChange({
                ...quic,
                initial_stream_receive_window:
                  event.target.value === "" ? null : Number(event.target.value),
              })
            }
          />
          <TextField
            size="small"
            type="number"
            label={t("quic.maxStreamWindow")}
            value={quic.max_stream_receive_window ?? ""}
            onChange={(event) =>
              onChange({
                ...quic,
                max_stream_receive_window:
                  event.target.value === "" ? null : Number(event.target.value),
              })
            }
          />
          <TextField
            size="small"
            type="number"
            label={t("quic.initialConnectionWindow")}
            value={quic.initial_connection_receive_window ?? ""}
            onChange={(event) =>
              onChange({
                ...quic,
                initial_connection_receive_window:
                  event.target.value === "" ? null : Number(event.target.value),
              })
            }
          />
          <TextField
            size="small"
            type="number"
            label={t("quic.maxConnectionWindow")}
            value={quic.max_connection_receive_window ?? ""}
            onChange={(event) =>
              onChange({
                ...quic,
                max_connection_receive_window:
                  event.target.value === "" ? null : Number(event.target.value),
              })
            }
          />
          <TextField
            size="small"
            label={t("quic.keepAlive")}
            placeholder="10s"
            value={quic.keep_alive_period ?? ""}
            onChange={(event) => onChange({ ...quic, keep_alive_period: event.target.value })}
          />
          <FormControlLabel
            control={
              <Switch
                checked={quic.disable_path_mtu_discovery}
                onChange={(event) =>
                  onChange({ ...quic, disable_path_mtu_discovery: event.target.checked })
                }
              />
            }
            label={t("quic.disableMtuDiscovery")}
          />
        </FieldRow>
      </AccordionDetails>
    </Accordion>
  );
}
