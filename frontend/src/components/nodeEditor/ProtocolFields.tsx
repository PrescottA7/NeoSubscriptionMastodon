/*
 * Copyright (C) 2026 Prescott Anderson
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * This file is part of NSM (NeoSubscriptionMastodon), licensed under the
 * GNU Affero General Public License v3.0 or later. See the LICENSE file
 * in the project root for the full text.
 */

import { VlessFields } from "./VlessFields";
import { TrojanFields } from "./TrojanFields";
import { Hysteria2Fields } from "./Hysteria2Fields";
import { TuicFields } from "./TuicFields";
import { AnytlsFields } from "./AnytlsFields";
import type { NodeParams } from "../../types";
import type { FieldErrors } from "../../validation";

interface ProtocolFieldsProps {
  params: NodeParams;
  onChange: (next: NodeParams) => void;
  errors?: FieldErrors;
}

export function ProtocolFields({ params, onChange, errors = {} }: ProtocolFieldsProps) {
  switch (params.protocol) {
    case "vless":
      return <VlessFields params={params} onChange={onChange} errors={errors} />;
    case "trojan":
      return <TrojanFields params={params} onChange={onChange} errors={errors} />;
    case "hysteria2":
      return <Hysteria2Fields params={params} onChange={onChange} errors={errors} />;
    case "tuic":
      return <TuicFields params={params} onChange={onChange} errors={errors} />;
    case "anytls":
      return <AnytlsFields params={params} onChange={onChange} errors={errors} />;
    default:
      return null;
  }
}
