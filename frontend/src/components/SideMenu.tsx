import {
  Box,
  Button,
  List,
  ListItemButton,
  Typography,
  styled,
} from "@mui/material";

import MuiDrawer, { drawerClasses } from "@mui/material/Drawer";
import { useLocation, Link } from "react-router-dom";
import type { ReactNode } from "react";

import ThemeToggle from "./ThemeToggle";
import { useColorMode } from "./colorMode";

export const drawerWidth = 236;

const Drawer = styled(MuiDrawer)({
  width: drawerWidth,
  flexShrink: 0,
  boxSizing: "border-box",
  [`& .${drawerClasses.paper}`]: {
    width: drawerWidth,
    boxSizing: "border-box",
  },
});

export interface PageInfo {
  label: string;
  path: string;
  icon?: ReactNode;
}

interface SideMenuProps {
  pages: PageInfo[];
  username: string;
  onLogout: () => void;
}

export default function SideMenu({ pages, username, onLogout }: SideMenuProps) {
  const location = useLocation();
  const { mode } = useColorMode();

  /** `/` is the nodes section, so its nested routes highlight it too. */
  function isActive(path: string) {
    if (path === "/") {
      return location.pathname === "/" || location.pathname.startsWith("/nodes");
    }
    return location.pathname === path || location.pathname.startsWith(`${path}/`);
  }

  return (
    <Drawer
      variant="permanent"
      sx={{
        display: { xs: "none", md: "block" },
        [`& .${drawerClasses.paper}`]: {
          backgroundColor: "background.paper",
          borderRight: "1px solid",
          borderColor: "divider",
        },
      }}
    >
      <Box
        sx={{
          height: 60,
          px: 2.5,
          display: "flex",
          alignItems: "center",
          gap: 1,
          borderBottom: "1px solid",
          borderColor: "divider",
        }}
      >
        <Box
          aria-hidden
          sx={{
            width: 18,
            height: 18,
            border: "1.5px solid",
            borderColor: "text.primary",
            borderRadius: "3px",
            position: "relative",
            "&::after": {
              content: '""',
              position: "absolute",
              inset: 4,
              backgroundColor: "text.primary",
              borderRadius: "1px",
            },
          }}
        />
        <Typography sx={{ fontWeight: 600, fontSize: "0.9375rem", letterSpacing: "-0.01em" }}>
          NSM
        </Typography>
      </Box>

      <List sx={{ px: 1.5, py: 2, flexGrow: 1 }}>
        <Typography
          variant="caption"
          sx={{
            display: "block",
            px: 1.5,
            pb: 1,
            color: "text.disabled",
            letterSpacing: "0.08em",
            textTransform: "uppercase",
            fontSize: "0.625rem",
          }}
        >
          管理
        </Typography>
        {pages.map((item) => {
          const active = isActive(item.path);
          return (
            <ListItemButton
              key={item.path}
              component={Link}
              to={item.path}
              selected={active}
              sx={{
                gap: 1.25,
                borderRadius: 6,
                mb: 0.25,
                minHeight: 36,
                px: 1.5,
                color: active ? "text.primary" : "text.secondary",
                fontWeight: active ? 600 : 400,
                position: "relative",
                // The rule, the weight and the colour already mark the active
                // section; a filled pill on top of that is redundant.
                "&.Mui-selected": {
                  backgroundColor: "transparent",
                  "&:hover": { backgroundColor: active ? "transparent" : "action.hover" },
                },
                // A left rule marks the active section without adding colour.
                "&::before": active
                  ? {
                      content: '""',
                      position: "absolute",
                      left: 0,
                      top: 8,
                      bottom: 8,
                      width: 2,
                      borderRadius: 2,
                      backgroundColor: "text.primary",
                    }
                  : undefined,
              }}
            >
              <Box
                component="span"
                aria-hidden
                sx={{
                  display: "flex",
                  width: 18,
                  flexShrink: 0,
                  color: active ? "text.primary" : "text.disabled",
                }}
              >
                {item.icon}
              </Box>
              {item.label}
            </ListItemButton>
          );
        })}
      </List>

      <Box sx={{ px: 1.5, py: 1.5, borderTop: "1px solid", borderColor: "divider" }}>
        <Box sx={{ display: "flex", alignItems: "center", gap: 1, px: 1.5, pb: 1 }}>
          <Box
            aria-hidden
            sx={{
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              width: 24,
              height: 24,
              borderRadius: "50%",
              border: "1px solid",
              borderColor: "divider",
              backgroundColor: "action.hover",
              fontSize: "0.6875rem",
              fontWeight: 600,
              textTransform: "uppercase",
              color: "text.secondary",
              flexShrink: 0,
            }}
          >
            {username.slice(0, 1)}
          </Box>
          <Box sx={{ minWidth: 0, flexGrow: 1 }}>
            <Typography
              sx={{ fontSize: "0.8125rem", fontWeight: 500, lineHeight: 1.3 }}
              noWrap
              title={username}
            >
              {username}
            </Typography>
            <Typography sx={{ fontSize: "0.6875rem", color: "text.disabled", lineHeight: 1.3 }}>
              管理员
            </Typography>
          </Box>
        </Box>

        <Box
          sx={{
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            pl: 1.5,
            pr: 0.5,
            pb: 0.5,
          }}
        >
          <Typography variant="caption" color="text.disabled" sx={{ fontSize: "0.6875rem" }}>
            {mode === "dark" ? "深色" : "浅色"}主题
          </Typography>
          <ThemeToggle />
        </Box>

        <Button
          fullWidth
          size="small"
          color="inherit"
          onClick={onLogout}
          sx={{
            justifyContent: "flex-start",
            px: 1.5,
            minHeight: 32,
            fontSize: "0.8125rem",
            color: "text.secondary",
            "&:hover": { backgroundColor: "action.hover", color: "text.primary" },
          }}
        >
          退出登录
        </Button>
      </Box>
    </Drawer>
  );
}
