/*
 * Copyright (C) 2026 Prescott Anderson
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * This file is part of NSM (NeoSubscriptionMastodon), licensed under the
 * GNU Affero General Public License v3.0 or later. See the LICENSE file
 * in the project root for the full text.
 */

import { Alert, Button, Stack, TextField } from "@mui/material";
import { Refresh } from "@mui/icons-material";
import { useTranslation } from "react-i18next";

import { generatePassword, generateUuid, type CredentialValues } from "./fields";

/**
 * Credentials as an Xray inbound reads them.
 *
 * VLESS has a UUID and no password; Trojan has a password and no UUID. Showing
 * both for either protocol is what previously let operators type a value the
 * server would silently ignore.
 */
export function XrayCredentials({
  protocol,
  values,
  onChange,
}: {
  protocol: "vless" | "trojan";
  values: CredentialValues;
  onChange: (next: CredentialValues) => void;
}) {
  const { t } = useTranslation();

  if (protocol === "vless") {
    return (
      <Stack spacing={1}>
        <TextField
          size="small"
          label={t("protocol.credentials.uuid")}
          value={values.uuid}
          onChange={(event) => onChange({ ...values, uuid: event.target.value })}
          helperText={t("protocol.credentials.uuidHelper")}
          fullWidth
          slotProps={{
            htmlInput: { style: { fontFamily: "monospace" } },
            input: {
              endAdornment: (
                <Button
                  size="small"
                  startIcon={<Refresh sx={{ fontSize: 14 }} />}
                  onClick={() => onChange({ ...values, uuid: generateUuid() })}
                >
                  {t("protocol.credentials.regenerate")}
                </Button>
              ),
            },
          }}
        />
        <Alert severity="info" sx={{ py: 0 }}>
          {t("protocol.credentials.uuidRequired")}
        </Alert>
      </Stack>
    );
  }

  return (
    <TextField
      size="small"
      label={t("protocol.credentials.password")}
      value={values.password}
      onChange={(event) => onChange({ ...values, password: event.target.value })}
      helperText={t("protocol.credentials.passwordHelper")}
      fullWidth
      type="password"
      slotProps={{
        htmlInput: { style: { fontFamily: "monospace" } },
        input: {
          endAdornment: (
            <Button
              size="small"
              startIcon={<Refresh sx={{ fontSize: 14 }} />}
              onClick={() => onChange({ ...values, password: generatePassword() })}
            >
              {t("protocol.credentials.regenerate")}
            </Button>
          ),
        },
      }}
    />
  );
}
