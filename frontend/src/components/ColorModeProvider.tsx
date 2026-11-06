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

import { DEFAULT_ACCENT, buildTheme, normalizeAccent } from "../theme";
import { ACCENT_STORAGE_KEY, COLOR_MODE_STORAGE_KEY, ColorModeContext } from "./colorMode";

function initialMode(): PaletteMode {
  const stored = window.localStorage.getItem(COLOR_MODE_STORAGE_KEY);
  if (stored === "light" || stored === "dark") {
    return stored;
  }
  // Follow the OS unless the user has chosen explicitly.
  return window.matchMedia?.("(prefers-color-scheme: light)").matches ? "light" : "dark";
}

function initialAccent(): string {
  return normalizeAccent(window.localStorage.getItem(ACCENT_STORAGE_KEY)) ?? DEFAULT_ACCENT;
}

/** Provides the active palette and exposes a toggle to any descendant. */
export default function ColorModeProvider({ children }: PropsWithChildren) {
  const [mode, setMode] = useState<PaletteMode>(initialMode);
  const [accent, setAccentState] = useState<string>(initialAccent);

  useEffect(() => {
    window.localStorage.setItem(COLOR_MODE_STORAGE_KEY, mode);
    // Keep the browser chrome (form controls, scrollbars) in step with the app.
    document.documentElement.style.colorScheme = mode;
  }, [mode]);

  useEffect(() => {
    window.localStorage.setItem(ACCENT_STORAGE_KEY, accent);
  }, [accent]);

  const toggle = useCallback(() => {
    setMode((current) => (current === "dark" ? "light" : "dark"));
  }, []);

  // Invalid input is ignored rather than stored, so the theme never breaks.
  const setAccent = useCallback((value: string) => {
    const normalized = normalizeAccent(value);
    if (normalized) {
      setAccentState(normalized);
    }
  }, []);

  const theme = useMemo(() => buildTheme(mode, accent), [mode, accent]);
  const value = useMemo(
    () => ({ mode, toggle, accent, setAccent }),
    [mode, toggle, accent, setAccent],
  );

  return (
    <ColorModeContext.Provider value={value}>
      <ThemeProvider theme={theme}>
        <CssBaseline />
        {children}
      </ThemeProvider>
    </ColorModeContext.Provider>
  );
}
