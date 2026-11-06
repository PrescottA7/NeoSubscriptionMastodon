/*
 * Copyright (C) 2026 Prescott Anderson
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * This file is part of NSM (NeoSubscriptionMastodon), licensed under the
 * GNU Affero General Public License v3.0 or later. See the LICENSE file
 * in the project root for the full text.
 */

import { FormControlLabel, Stack, Switch } from "@mui/material";
import { useTranslation } from "react-i18next";

import { TlsSection } from "../TlsEditor";
import { TransportSection } from "../TransportEditor";
import type { TrojanParams } from "../../types";
import type { FieldErrors } from "../../validation";

interface TrojanFieldsProps {
  params: TrojanParams;
  onChange: (next: TrojanParams) => void;
  errors?: FieldErrors;
}

export function TrojanFields({ params, onChange, errors = {} }: TrojanFieldsProps) {
  const { t } = useTranslation();

  return (
    <Stack spacing={2}>
      <FormControlLabel
        control={
          <Switch
            checked={params.udp}
            onChange={(event) => onChange({ ...params, udp: event.target.checked })}
          />
        }
        label={t("protocol.udpEnabled")}
      />
      <TlsSection
        tls={params.tls}
        onChange={(tls) => onChange({ ...params, tls })}
        requireTls
        allowReality={false}
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
