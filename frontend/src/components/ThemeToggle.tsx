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

import { useColorMode } from "./colorMode";

export default function ThemeToggle() {
  const { mode, toggle } = useColorMode();
  const next = mode === "dark" ? "浅色" : "深色";

  return (
    <Tooltip title={`切换到${next}主题`}>
      <IconButton size="small" onClick={toggle} aria-label={`切换到${next}主题`}>
        {mode === "dark" ? (
          <LightModeOutlined sx={{ fontSize: 18 }} />
        ) : (
          <DarkModeOutlined sx={{ fontSize: 18 }} />
        )}
      </IconButton>
    </Tooltip>
  );
}
