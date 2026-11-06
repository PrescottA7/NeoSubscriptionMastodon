/*
 * Copyright (C) 2026 Prescott Anderson
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * This file is part of NSM (NeoSubscriptionMastodon), licensed under the
 * GNU Affero General Public License v3.0 or later. See the LICENSE file
 * in the project root for the full text.
 */

import { useEffect, useState, type PropsWithChildren } from "react";
import { Alert, Box, Snackbar } from "@mui/material";

import NavBar from "./NavBar";
import SideMenu, { type PageInfo } from "./SideMenu";
import { noticeDismissed } from "../store/noticesSlice";
import { useAppDispatch, useAppSelector } from "../store/hooks";

interface PageContainerProps extends PropsWithChildren {
  pages: PageInfo[];
  username: string;
  onLogout: () => void;
  toolbar?: React.ReactNode;
}

export default function PageContainer({
  pages,
  username,
  onLogout,
  toolbar,
  children,
}: PageContainerProps) {
  const dispatch = useAppDispatch();
  const notice = useAppSelector((state) => state.notices.latest);
  const [dismissedId, setDismissedId] = useState<number | null>(null);

  const active = notice && notice.id !== dismissedId ? notice : null;

  useEffect(() => {
    if (!notice) {
      return undefined;
    }
    const timer = window.setTimeout(() => setDismissedId(notice.id), 3500);
    return () => window.clearTimeout(timer);
  }, [notice]);

  function handleClose() {
    if (notice) {
      setDismissedId(notice.id);
      dispatch(noticeDismissed(notice.id));
    }
  }

  return (
    <>
      <NavBar pages={pages} username={username} onLogout={onLogout} />
      <Snackbar
        open={Boolean(active)}
        autoHideDuration={3500}
        onClose={handleClose}
        anchorOrigin={{ vertical: "bottom", horizontal: "right" }}
      >
        <Alert
          severity={active?.severity ?? "info"}
          variant="outlined"
          onClose={handleClose}
          sx={{ width: "100%", maxWidth: 380 }}
        >
          {active?.message ?? ""}
        </Alert>
      </Snackbar>
      <Box sx={{ display: "flex", minHeight: "100dvh" }}>
        <SideMenu pages={pages} username={username} onLogout={onLogout} />
        <Box component="main" sx={{ flexGrow: 1, minWidth: 0 }}>
          {toolbar}
          <Box
            sx={{
              width: "100%",
              maxWidth: 1280,
              px: { xs: 2, md: 4 },
              py: { xs: 3, md: 4 },
            }}
          >
            {children}
          </Box>
        </Box>
      </Box>
    </>
  );
}
