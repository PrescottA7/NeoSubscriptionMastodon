import { createContext, useContext } from "react";
import type { PaletteMode } from "@mui/material";

export type ColorModeContextValue = {
  mode: PaletteMode;
  toggle: () => void;
};

export const ColorModeContext = createContext<ColorModeContextValue>({
  mode: "dark",
  toggle: () => undefined,
});

export function useColorMode() {
  return useContext(ColorModeContext);
}

export const COLOR_MODE_STORAGE_KEY = "nsm.color-mode";
