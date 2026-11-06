/*
 * Copyright (C) 2026 Prescott Anderson
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * This file is part of NSM (NeoSubscriptionMastodon), licensed under the
 * GNU Affero General Public License v3.0 or later. See the LICENSE file
 * in the project root for the full text.
 */

import { Button, Stack, TextField } from "@mui/material";
import { Refresh } from "@mui/icons-material";
import { useTranslation } from "react-i18next";

import { generatePassword, type CredentialValues } from "./fields";

/**
 * Credentials as the Hysteria2 server reads them.
 *
 * Hysteria2 has no UUID concept: the server builds an `auth.userpass` map, so an
 * account is a username plus a password. The username lives in the shared
 * `email` column because that is what every client displays as the label.
 */
export function Hysteria2Credentials({
  values,
  onChange,
}: {
  values: CredentialValues;
  onChange: (next: CredentialValues) => void;
}) {
  const { t } = useTranslation();

  return (
    <Stack spacing={1.5}>
      <TextField
        size="small"
        label={t("protocol.credentials.username")}
        value={values.username}
        onChange={(event) => onChange({ ...values, username: event.target.value })}
        helperText={t("protocol.credentials.usernameHelper")}
        fullWidth
        slotProps={{ htmlInput: { style: { fontFamily: "monospace" } } }}
      />
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
    </Stack>
  );
}
