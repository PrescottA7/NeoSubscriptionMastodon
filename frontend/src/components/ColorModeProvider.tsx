/*
 * Copyright (C) 2026 Prescott Anderson
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * This file is part of NSM (NeoSubscriptionMastodon), licensed under the
 * GNU Affero General Public License v3.0 or later. See the LICENSE file
 * in the project root for the full text.
 */

import { useCallback, useEffect, useMemo, useState } from "react";
import type { PropsWithChildren } from "react";
import { CssBaseline, ThemeProvider } from "@mui/material";
import type { PaletteMode } from "@mui/material";

import { buildTheme } from "../theme";
import { COLOR_MODE_STORAGE_KEY, ColorModeContext } from "./colorMode";

function initialMode(): PaletteMode {
  const stored = window.localStorage.getItem(COLOR_MODE_STORAGE_KEY);
  if (stored === "light" || stored === "dark") {
    return stored;
  }
  // Follow the OS unless the user has chosen explicitly.
  return window.matchMedia?.("(prefers-color-scheme: light)").matches ? "light" : "dark";
}

/** Provides the active palette and exposes a toggle to any descendant. */
export default function ColorModeProvider({ children }: PropsWithChildren) {
  const [mode, setMode] = useState<PaletteMode>(initialMode);

  useEffect(() => {
    window.localStorage.setItem(COLOR_MODE_STORAGE_KEY, mode);
    // Keep the browser chrome (form controls, scrollbars) in step with the app.
    document.documentElement.style.colorScheme = mode;
  }, [mode]);

  const toggle = useCallback(() => {
    setMode((current) => (current === "dark" ? "light" : "dark"));
  }, []);

  const theme = useMemo(() => buildTheme(mode), [mode]);
  const value = useMemo(() => ({ mode, toggle }), [mode, toggle]);

  return (
    <ColorModeContext.Provider value={value}>
      <ThemeProvider theme={theme}>
        <CssBaseline />
        {children}
      </ThemeProvider>
    </ColorModeContext.Provider>
  );
}
