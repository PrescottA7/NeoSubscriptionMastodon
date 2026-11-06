/*
 * Copyright (C) 2026 Prescott Anderson
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * This file is part of NSM (NeoSubscriptionMastodon), licensed under the
 * GNU Affero General Public License v3.0 or later. See the LICENSE file
 * in the project root for the full text.
 */

import { Alert, Box, Button, MenuItem, Stack, TextField, Typography } from "@mui/material";
import { Add } from "@mui/icons-material";
import { useTranslation } from "react-i18next";

import type { ServerConfig, ServerCore } from "../../types";
import type { FieldErrors } from "../../validation";
import { emptyServerConfig, retargetCore } from "./defaults";
import { Hysteria2ServerEditor } from "./Hysteria2ServerEditor";
import { XrayServerEditor } from "./XrayServerEditor";

/**
 * Server-side (inbound) settings for a node.
 *
 * This is how the machine itself runs, not how a client reaches it. Each core
 * gets its own editor because their inbound models barely overlap: Xray has
 * transports, sniffing, REALITY, and fallbacks; Hysteria2 has a listener, a
 * certificate, and an account table. A shared form with conditionals used to let
 * operators configure REALITY on Hysteria2, which the exporter then dropped.
 */
export function ServerSection({
  config,
  onChange,
  errors = {},
  protocol,
}: {
  config: ServerConfig | null;
  onChange: (next: ServerConfig | null) => void;
  errors?: FieldErrors;
  /** Protocol of the node, used to pick a sensible default listener. */
  protocol?: string;
}) {
  const { t } = useTranslation();

  if (!config) {
    return (
      <Box>
        <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
          {t("server.noConfig")}
        </Typography>
        <Button
          variant="outlined"
          onClick={() => onChange(emptyServerConfig("xray", protocol))}
          startIcon={<Add />}
        >
          {t("server.addConfig")}
        </Button>
      </Box>
    );
  }

  const core = config.core;

  function changeCore(next: ServerCore) {
    if (next === core) {
      return;
    }
    onChange(retargetCore(config!, next));
  }

  return (
    <Stack spacing={2}>
      <TextField
        size="small"
        select
        fullWidth
        label={t("server.targetCore")}
        value={core}
        onChange={(event) => changeCore(event.target.value as ServerCore)}
        helperText={t("server.coreHint", { core: t(`core.${core}`) })}
      >
        <MenuItem value="xray">{t("core.xray")}</MenuItem>
        <MenuItem value="hysteria2">{t("core.hysteria2")}</MenuItem>
      </TextField>

      {core === "hysteria2" && config.inbound.protocol !== "hysteria2" && (
        <Alert severity="warning">
          {t("core.notSupported", {
            protocol: t(`protocol.${config.inbound.protocol}`),
            core: t("core.hysteria2"),
          })}
        </Alert>
      )}

      {core === "xray" ? (
        <XrayServerEditor config={config} onChange={onChange} errors={errors} />
      ) : (
        <Hysteria2ServerEditor config={config} onChange={onChange} errors={errors} />
      )}

      <Box>
        <Button color="error" size="small" onClick={() => onChange(null)}>
          {t("server.removeConfig")}
        </Button>
      </Box>
    </Stack>
  );
}
