/*
 * Copyright (C) 2026 Prescott Anderson
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * This file is part of NSM (NeoSubscriptionMastodon), licensed under the
 * GNU Affero General Public License v3.0 or later. See the LICENSE file
 * in the project root for the full text.
 */

import { createContext, useContext } from "react";
import type { PaletteMode } from "@mui/material";
import { DEFAULT_ACCENT } from "../theme";

export type ColorModeContextValue = {
  mode: PaletteMode;
  toggle: () => void;
  /** `#rrggbb` accent applied to primary actions, focus rings and selection. */
  accent: string;
  setAccent: (accent: string) => void;
};

export const ColorModeContext = createContext<ColorModeContextValue>({
  mode: "dark",
  toggle: () => undefined,
  accent: DEFAULT_ACCENT,
  setAccent: () => undefined,
});

export function useColorMode() {
  return useContext(ColorModeContext);
}

export const COLOR_MODE_STORAGE_KEY = "nsm.color-mode";
export const ACCENT_STORAGE_KEY = "nsm.accent";
