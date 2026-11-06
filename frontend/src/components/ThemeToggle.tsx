/*
 * Copyright (C) 2026 Prescott Anderson
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * This file is part of NSM (NeoSubscriptionMastodon), licensed under the
 * GNU Affero General Public License v3.0 or later. See the LICENSE file
 * in the project root for the full text.
 */

import { IconButton, Tooltip } from "@mui/material";
import { DarkModeOutlined, LightModeOutlined } from "@mui/icons-material";
import { useTranslation } from "react-i18next";

import { useColorMode } from "./colorMode";

export default function ThemeToggle() {
  const { t } = useTranslation();
  const { mode, toggle } = useColorMode();
  const label = mode === "dark" ? t("common.switchToLight") : t("common.switchToDark");

  return (
    <Tooltip title={label}>
      <IconButton size="small" onClick={toggle} aria-label={label}>
        {mode === "dark" ? (
          <LightModeOutlined sx={{ fontSize: 18 }} />
        ) : (
          <DarkModeOutlined sx={{ fontSize: 18 }} />
        )}
      </IconButton>
    </Tooltip>
  );
}
