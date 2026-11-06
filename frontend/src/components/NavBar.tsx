/*
 * Copyright (C) 2026 Prescott Anderson
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * This file is part of NSM (NeoSubscriptionMastodon), licensed under the
 * GNU Affero General Public License v3.0 or later. See the LICENSE file
 * in the project root for the full text.
 */

import { useState } from "react";
import {
  AppBar,
  Box,
  Button,
  Drawer,
  IconButton,
  List,
  ListItemButton,
  Stack,
  Toolbar,
  Typography,
} from "@mui/material";
import { Menu as MenuIcon } from "@mui/icons-material";
import { Link, useLocation } from "react-router-dom";

import ThemeToggle from "./ThemeToggle";
import type { PageInfo } from "./SideMenu";

/** Compact top bar shown below the `md` breakpoint, where the rail is hidden. */
export default function NavBar({
  pages,
  username,
  onLogout,
}: {
  pages: PageInfo[];
  username: string;
  onLogout: () => void;
}) {
  const [open, setOpen] = useState(false);
  const location = useLocation();

  /** `/` is the nodes section, so its nested routes highlight it too. */
  function isActive(path: string) {
    if (path === "/") {
      return location.pathname === "/" || location.pathname.startsWith("/nodes");
    }
    return location.pathname === path || location.pathname.startsWith(`${path}/`);
  }

  return (
    <>
      <AppBar
        position="sticky"
        sx={{
          display: { xs: "auto", md: "none" },
          boxShadow: 0,
          bgcolor: "background.paper",
          backgroundImage: "none",
          borderBottom: "1px solid",
          borderColor: "divider",
          color: "text.primary",
        }}
      >
        <Toolbar variant="dense" sx={{ minHeight: 56, gap: 1 }}>
          <IconButton
            edge="start"
            aria-label="打开菜单"
            size="small"
            onClick={() => setOpen(true)}
          >
            <MenuIcon sx={{ fontSize: 20 }} />
          </IconButton>
          <Typography sx={{ fontWeight: 600, fontSize: "0.9375rem", flexGrow: 1 }}>NSM</Typography>
          <ThemeToggle />
        </Toolbar>
      </AppBar>

      <Drawer open={open} onClose={() => setOpen(false)} sx={{ display: { md: "none" } }}>
        <Stack sx={{ width: 260, height: "100%" }}>
          <Box
            sx={{
              px: 2.5,
              height: 56,
              display: "flex",
              alignItems: "center",
              borderBottom: "1px solid",
              borderColor: "divider",
            }}
          >
            <Typography sx={{ fontWeight: 600 }}>NSM</Typography>
          </Box>
          <List sx={{ p: 1.5, flexGrow: 1 }}>
            {pages.map((item) => {
              const active = isActive(item.path);
              return (
                <ListItemButton
                  key={item.path}
                  component={Link}
                  to={item.path}
                  selected={active}
                  onClick={() => setOpen(false)}
                  sx={{
                    gap: 1.25,
                    borderRadius: 6,
                    minHeight: 38,
                    mb: 0.25,
                    color: active ? "text.primary" : "text.secondary",
                    fontWeight: active ? 600 : 400,
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
                >
                  {username}
                </Typography>
                <Typography
                  sx={{ fontSize: "0.6875rem", color: "text.disabled", lineHeight: 1.3 }}
                >
                  管理员
                </Typography>
              </Box>
            </Box>
            <Button
              fullWidth
              size="small"
              color="inherit"
              onClick={() => {
                setOpen(false);
                onLogout();
              }}
              sx={{
                justifyContent: "flex-start",
                px: 1.5,
                minHeight: 34,
                fontSize: "0.8125rem",
                color: "text.secondary",
                "&:hover": { backgroundColor: "action.hover", color: "text.primary" },
              }}
            >
              退出登录
            </Button>
          </Box>
        </Stack>
      </Drawer>
    </>
  );
}
