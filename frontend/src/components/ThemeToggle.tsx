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
