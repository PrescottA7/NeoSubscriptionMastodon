/*
 * Copyright (C) 2026 Prescott Anderson
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * This file is part of NSM (NeoSubscriptionMastodon), licensed under the
 * GNU Affero General Public License v3.0 or later. See the LICENSE file
 * in the project root for the full text.
 */

import { Alert, Button, Stack, TextField } from "@mui/material";
import { useTranslation } from "react-i18next";

import type { ProtocolName, ServerCore } from "../../types";
import { Hysteria2Credentials } from "./Hysteria2Credentials";
import { XrayCredentials } from "./XrayCredentials";
import { generatePassword, generateUuid, type CredentialValues } from "./fields";

/**
 * Renders exactly the credential inputs the selected core will read.
 *
 * Each core has its own component rather than one generic form with conditional
 * fields, because the shapes genuinely differ: Xray wants a UUID for VLESS and a
 * password for Trojan, while Hysteria2 wants a username/password pair and has no
 * UUID at all.
 */
export function CredentialFields({
  core,
  protocol,
  values,
  onChange,
  errors = {},
  index,
}: {
  core: ServerCore;
  protocol: ProtocolName;
  values: CredentialValues;
  onChange: (next: CredentialValues) => void;
  errors?: Record<string, string>;
  /** Row index, used to look up `credentials.<index>.<field>` errors. */
  index?: number;
}) {
  const { t } = useTranslation();
  const errorFor = (field: string) =>
    index === undefined ? undefined : errors[`credentials.${index}.${field}`];

  if (core === "hysteria2") {
    // The Hysteria2 exporter only understands its own protocol; anything else
    // would produce a config with no accounts, so say so instead of guessing.
    if (protocol !== "hysteria2") {
      return (
        <Alert severity="warning" sx={{ py: 0 }}>
          {t("core.notSupported", {
            protocol: t(`protocol.${protocol}`),
            core: t("core.hysteria2"),
          })}
        </Alert>
      );
    }
    return <Hysteria2Credentials values={values} onChange={onChange} />;
  }

  if (protocol === "vless" || protocol === "trojan") {
    return (
      <XrayCredentials
        protocol={protocol}
        values={values}
        onChange={onChange}
      />
    );
  }

  // TUIC and AnyTLS have no inbound in either core, so only the client-facing
  // shape can be shown; it is labelled as such rather than pretending it is a
  // server credential.
  if (protocol === "tuic") {
    return (
      <Stack spacing={1.5}>
        <TextField
          size="small"
          label={t("protocol.credentials.uuid")}
          value={values.uuid}
          onChange={(event) => onChange({ ...values, uuid: event.target.value })}
          error={Boolean(errorFor("uuid"))}
          helperText={errorFor("uuid") ?? t("protocol.credentials.uuidHelper")}
          fullWidth
          slotProps={{
            htmlInput: { style: { fontFamily: "monospace" } },
            input: {
              endAdornment: (
                <Button onClick={() => onChange({ ...values, uuid: generateUuid() })}>
                  {t("protocol.credentials.regenerate")}
                </Button>
              ),
            },
          }}
        />
        <TextField
          size="small"
          label={t("protocol.credentials.password")}
          value={values.password}
          onChange={(event) => onChange({ ...values, password: event.target.value })}
          error={Boolean(errorFor("password"))}
          helperText={errorFor("password") ?? t("protocol.credentials.passwordHelper")}
          fullWidth
          type="password"
          slotProps={{
            htmlInput: { style: { fontFamily: "monospace" } },
            input: {
              endAdornment: (
                <Button onClick={() => onChange({ ...values, password: generatePassword() })}>
                  {t("protocol.credentials.regenerate")}
                </Button>
              ),
            },
          }}
        />
        <Alert severity="info" sx={{ py: 0 }}>
          {t("core.notSupported", {
            protocol: t("protocol.tuic"),
            core: t("core.xray"),
          })}
        </Alert>
      </Stack>
    );
  }

  return (
    <Stack spacing={1.5}>
      <TextField
        size="small"
        label={t("protocol.credentials.password")}
        value={values.password}
        onChange={(event) => onChange({ ...values, password: event.target.value })}
        error={Boolean(errorFor("password"))}
        helperText={errorFor("password") ?? t("protocol.credentials.passwordHelper")}
        fullWidth
        type="password"
        slotProps={{
          htmlInput: { style: { fontFamily: "monospace" } },
          input: {
            endAdornment: (
              <Button onClick={() => onChange({ ...values, password: generatePassword() })}>
                {t("protocol.credentials.regenerate")}
              </Button>
            ),
          },
        }}
      />
      <Alert severity="info" sx={{ py: 0 }}>
        {t("core.notSupported", {
          protocol: t("protocol.anytls"),
          core: t("core.xray"),
        })}
      </Alert>
    </Stack>
  );
}
