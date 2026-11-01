import { createTheme, type Theme } from "@mui/material";
import type { PaletteMode } from "@mui/material";

/**
 * Monochrome design language.
 *
 * The UI deliberately avoids accent hues: hierarchy comes from weight, spacing,
 * and surface contrast. A single neutral scale serves both modes so a component
 * looks the same whether it is rendered light or dark.
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

export function buildTheme(mode: PaletteMode): Theme {
  const c = NEUTRALS[mode];
  const isDark = mode === "dark";

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
      // Everything "accent" collapses onto the text colour so the UI stays
      // monochrome; contrast carries the emphasis instead of hue.
      primary: { main: c.text, contrastText: c.inverse },
      secondary: { main: c.textMuted, contrastText: c.inverse },
      success: { main: c.text, contrastText: c.inverse },
      error: { main: c.text, contrastText: c.inverse },
      warning: { main: c.text, contrastText: c.inverse },
      info: { main: c.text, contrastText: c.inverse },
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
            backgroundColor: isDark ? "rgba(250,250,250,0.20)" : "rgba(9,9,11,0.12)",
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
            backgroundColor: c.text,
            color: c.inverse,
            "&:hover": { backgroundColor: isDark ? "#e4e4e7" : "#27272a" },
          },
          outlined: {
            borderColor: c.borderStrong,
            "&:hover": {
              borderColor: c.text,
              backgroundColor: isDark ? "rgba(250,250,250,0.06)" : "rgba(9,9,11,0.04)",
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
              borderColor: c.text,
              borderWidth: 1,
              // A 1px ring keeps focus visible without introducing colour.
              boxShadow: `0 0 0 3px ${isDark ? "rgba(250,250,250,0.10)" : "rgba(9,9,11,0.07)"}`,
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
          icon: { color: c.textMuted },
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
            backgroundColor: c.text,
            color: c.inverse,
            fontSize: "0.6875rem",
            borderRadius: 4,
          },
        },
      },
    },
  });
}

/** Colours that components need but MUI does not expose through the palette. */
export function surfaces(mode: PaletteMode) {
  return NEUTRALS[mode];
}
