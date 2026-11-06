/*
 * Copyright (C) 2026 Prescott Anderson
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * This file is part of NSM (NeoSubscriptionMastodon), licensed under the
 * GNU Affero General Public License v3.0 or later. See the LICENSE file
 * in the project root for the full text.
 */

import { createTheme, type Theme } from "@mui/material";
import type { PaletteMode } from "@mui/material";

/**
 * Neutral surfaces plus one configurable accent.
 *
 * Hierarchy still comes from weight, spacing, and surface contrast; the accent
 * only marks interactive emphasis (primary buttons, focus rings, selection) so
 * the layout keeps its calm, developer-facing look.
 */
const NEUTRALS = {
  light: {
    background: "#ffffff",
    surface: "#fafafa",
    sunken: "#f4f4f5",
    border: "#e4e4e7",
    borderStrong: "#d4d4d8",
    text: "#09090b",
    textMuted: "#71717a",
    textFaint: "#a1a1aa",
    inverse: "#ffffff",
  },
  dark: {
    background: "#09090b",
    surface: "#111113",
    sunken: "#18181b",
    border: "#27272a",
    borderStrong: "#3f3f46",
    text: "#fafafa",
    textMuted: "#a1a1aa",
    textFaint: "#71717a",
    inverse: "#09090b",
  },
} as const;

export const MONO_FONT =
  'ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, "Liberation Mono", monospace';

/** Accent used when none is configured: the familiar Windows/Microsoft blue. */
export const DEFAULT_ACCENT = "#16a34a";

/** Preset accents offered in the settings screen; `labelKey` is translated. */
export const ACCENT_PRESETS: { labelKey: string; value: string }[] = [
  { labelKey: "theme.microsoftBlue", value: "#0078d4" },
  { labelKey: "theme.skyBlue", value: "#0ea5e9" },
  { labelKey: "theme.teal", value: "#0d9488" },
  { labelKey: "theme.emerald", value: DEFAULT_ACCENT },
  { labelKey: "theme.amber", value: "#d97706" },
  { labelKey: "theme.rose", value: "#e11d48" },
  { labelKey: "theme.violet", value: "#7c3aed" },
  { labelKey: "theme.indigo", value: "#09090b" },
];

const HEX_COLOUR = /^#[0-9a-fA-F]{6}$/;

/** Accepts `#rrggbb` (and `#rgb`, expanded) so stored values stay canonical. */
export function normalizeAccent(value: string | null | undefined): string | null {
  const trimmed = (value ?? "").trim();
  if (HEX_COLOUR.test(trimmed)) {
    return trimmed.toLowerCase();
  }
  if (/^#[0-9a-fA-F]{3}$/.test(trimmed)) {
    const [r, g, b] = trimmed.slice(1).toLowerCase().split("");
    return `#${r}${r}${g}${g}${b}${b}`;
  }
  return null;
}

/** Relative luminance per WCAG 2.x, used to pick readable on-accent text. */
function luminance(hex: string): number {
  const channels = [1, 3, 5].map((offset) => {
    const value = parseInt(hex.slice(offset, offset + 2), 16) / 255;
    return value <= 0.03928 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * channels[0] + 0.7152 * channels[1] + 0.0722 * channels[2];
}

/** Mixes two `#rrggbb` colours; `weight` is how much of `hex` to keep. */
function mix(hex: string, other: string, weight: number): string {
  const channels = (value: string) =>
    [1, 3, 5].map((offset) => parseInt(value.slice(offset, offset + 2), 16));
  const from = channels(hex);
  const to = channels(other);
  const blended = from.map((channel, index) =>
    Math.round(channel * weight + to[index] * (1 - weight))
      .toString(16)
      .padStart(2, "0"),
  );
  return `#${blended.join("")}`;
}

export function buildTheme(mode: PaletteMode, accent: string = DEFAULT_ACCENT): Theme {
  const c = NEUTRALS[mode];
  const isDark = mode === "dark";
  const spot = normalizeAccent(accent) ?? DEFAULT_ACCENT;
  // Dark mode lifts the accent so it keeps contrast against near-black surfaces.
  const accentMain = isDark ? mix(spot, "#ffffff", 0.88) : spot;
  const accentHover = isDark ? mix(spot, "#ffffff", 0.75) : mix(spot, "#000000", 0.85);
  const accentContrast = luminance(accentMain) > 0.5 ? "#09090b" : "#ffffff";
  const accentRgb = [1, 3, 5]
    .map((offset) => parseInt(accentMain.slice(offset, offset + 2), 16))
    .join(",");
  const accentWash = (alpha: number) => `rgba(${accentRgb},${alpha})`;

  return createTheme({
    palette: {
      mode,
      background: { default: c.background, paper: c.surface },
      divider: c.border,
      text: {
        primary: c.text,
        secondary: c.textMuted,
        disabled: c.textFaint,
      },
      // Semantic state colours keep their hue; `primary` carries the configured
      // accent so interactive emphasis follows the operator's choice.
      primary: { main: accentMain, dark: accentHover, contrastText: accentContrast },
      secondary: { main: c.textMuted, contrastText: c.inverse },
      success: { main: isDark ? "#4ade80" : "#15803d" },
      error: { main: isDark ? "#f87171" : "#b91c1c" },
      warning: { main: isDark ? "#fbbf24" : "#b45309" },
      info: { main: isDark ? "#60a5fa" : "#1d4ed8" },
      action: {
        hover: isDark ? "rgba(250,250,250,0.06)" : "rgba(9,9,11,0.04)",
        selected: isDark ? "rgba(250,250,250,0.10)" : "rgba(9,9,11,0.06)",
        focus: isDark ? "rgba(250,250,250,0.12)" : "rgba(9,9,11,0.08)",
      },
    },
    shape: { borderRadius: 6 },
    typography: {
      fontFamily:
        '"Inter", "Roboto", -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif',
      // Tight, large headings in the style of modern developer-facing sites.
      h1: { fontSize: "2.5rem", fontWeight: 600, letterSpacing: "-0.035em", lineHeight: 1.1 },
      h2: { fontSize: "1.875rem", fontWeight: 600, letterSpacing: "-0.03em", lineHeight: 1.15 },
      h3: { fontSize: "1.375rem", fontWeight: 600, letterSpacing: "-0.02em", lineHeight: 1.25 },
      h4: { fontSize: "1.125rem", fontWeight: 600, letterSpacing: "-0.015em" },
      h5: { fontSize: "1rem", fontWeight: 600, letterSpacing: "-0.01em" },
      h6: { fontSize: "0.9375rem", fontWeight: 600, letterSpacing: "-0.01em" },
      body1: { fontSize: "0.875rem", lineHeight: 1.65 },
      body2: { fontSize: "0.8125rem", lineHeight: 1.6 },
      caption: { fontSize: "0.75rem", letterSpacing: "0.01em" },
      button: { textTransform: "none", fontWeight: 500, letterSpacing: 0 },
    },
    components: {
      MuiCssBaseline: {
        styleOverrides: {
          body: {
            backgroundColor: c.background,
            // Tabular numbers keep tables from jittering as values change.
            fontFeatureSettings: '"cv11", "ss01"',
            WebkitFontSmoothing: "antialiased",
          },
          "*::selection": {
            backgroundColor: accentWash(isDark ? 0.32 : 0.2),
          },
          code: { fontFamily: MONO_FONT, fontSize: "0.85em" },
        },
      },
      MuiPaper: {
        defaultProps: { elevation: 0 },
        styleOverrides: {
          root: { backgroundImage: "none" },
          outlined: { borderColor: c.border },
        },
      },
      MuiCard: {
        defaultProps: { elevation: 0, variant: "outlined" },
        styleOverrides: {
          root: {
            borderColor: c.border,
            backgroundImage: "none",
            // A card is a container, not an ornament: no shadow, no radius bloat.
            borderRadius: 8,
          },
        },
      },
      MuiButton: {
        defaultProps: { disableElevation: true },
        styleOverrides: {
          root: { borderRadius: 6, paddingInline: 14, minHeight: 34 },
          containedPrimary: {
            backgroundColor: accentMain,
            color: accentContrast,
            "&:hover": { backgroundColor: accentHover },
          },
          outlined: {
            borderColor: c.borderStrong,
            "&:hover": {
              borderColor: accentMain,
              backgroundColor: accentWash(0.06),
            },
          },
        },
      },
      MuiOutlinedInput: {
        styleOverrides: {
          root: {
            backgroundColor: c.background,
            "& fieldset": { borderColor: c.border },
            "&:hover fieldset": { borderColor: c.borderStrong },
            "&.Mui-focused fieldset": {
              borderColor: accentMain,
              borderWidth: 1,
              // A 1px ring keeps focus visible; the accent makes it findable.
              boxShadow: `0 0 0 3px ${accentWash(0.18)}`,
            },
          },
          input: { fontSize: "0.8125rem" },
        },
      },
      MuiInputLabel: { styleOverrides: { root: { fontSize: "0.8125rem" } } },
      MuiFormHelperText: {
        styleOverrides: {
          root: { fontSize: "0.6875rem", marginInline: 0, color: c.textMuted },
        },
      },
      MuiTableHead: {
        styleOverrides: {
          root: {
            "& .MuiTableCell-root": {
              fontSize: "0.6875rem",
              fontWeight: 500,
              letterSpacing: "0.06em",
              textTransform: "uppercase",
              color: c.textMuted,
              backgroundColor: c.sunken,
              borderBottom: `1px solid ${c.border}`,
              paddingBlock: 10,
            },
          },
        },
      },
      MuiTableCell: {
        styleOverrides: {
          root: { borderColor: c.border, fontSize: "0.8125rem" },
        },
      },
      MuiChip: {
        styleOverrides: {
          root: { borderRadius: 4, fontSize: "0.6875rem", fontWeight: 500 },
          outlined: { borderColor: c.borderStrong },
        },
      },
      MuiDivider: { styleOverrides: { root: { borderColor: c.border } } },
      MuiAccordion: {
        defaultProps: { elevation: 0, disableGutters: true },
        styleOverrides: {
          root: {
            border: `1px solid ${c.border}`,
            borderRadius: 8,
            backgroundImage: "none",
            backgroundColor: c.background,
            "&::before": { display: "none" },
            "&.Mui-expanded": { margin: 0 },
          },
        },
      },
      MuiAccordionSummary: {
        styleOverrides: {
          root: { minHeight: 48, backgroundColor: c.surface },
          content: { marginBlock: 10 },
        },
      },
      MuiAlert: {
        styleOverrides: {
          root: {
            border: `1px solid ${c.border}`,
            backgroundColor: c.surface,
            color: c.text,
            borderRadius: 6,
            fontSize: "0.8125rem",
          },
          // Alert colours are owned by the variant itself, which lets the
          // severity tint reach the icon and background instead of a flat grey.
        },
      },
      MuiDialog: {
        styleOverrides: {
          paper: { border: `1px solid ${c.border}`, backgroundImage: "none" },
        },
      },
      MuiTooltip: {
        styleOverrides: {
          tooltip: {
            backgroundColor: isDark ? c.borderStrong : c.text,
            color: c.inverse,
            fontSize: "0.6875rem",
            borderRadius: 4,
          },
        },
      },
      MuiLink: {
        styleOverrides: {
          root: { color: accentMain },
        },
      },
      MuiCheckbox: {
        styleOverrides: {
          root: { color: c.borderStrong, "&.Mui-checked": { color: accentMain } },
        },
      },
      MuiRadio: {
        styleOverrides: {
          root: { color: c.borderStrong, "&.Mui-checked": { color: accentMain } },
        },
      },
      MuiSwitch: {
        styleOverrides: {
          root: {
            "& .MuiSwitch-switchBase.Mui-checked": { color: accentMain },
            "& .MuiSwitch-switchBase.Mui-checked + .MuiSwitch-track": {
              backgroundColor: accentMain,
            },
          },
        },
      },
      MuiTabs: {
        styleOverrides: {
          indicator: { backgroundColor: accentMain },
        },
      },
      MuiTab: {
        styleOverrides: {
          root: { "&.Mui-selected": { color: accentMain } },
        },
      },
    },
  });
}

/** Colours that components need but MUI does not expose through the palette. */
export function surfaces(mode: PaletteMode) {
  return NEUTRALS[mode];
}