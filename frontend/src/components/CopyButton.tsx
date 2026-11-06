/*
 * Copyright (C) 2026 Prescott Anderson
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * This file is part of NSM (NeoSubscriptionMastodon), licensed under the
 * GNU Affero General Public License v3.0 or later. See the LICENSE file
 * in the project root for the full text.
 */

import { useState } from "react";
import { Box, IconButton, Snackbar, Tooltip, Typography } from "@mui/material";
import { ContentCopy } from "@mui/icons-material";

export function CopyButton({
  value,
  label = "复制",
  size = "small",
}: {
  value: string;
  label?: string;
  size?: "small" | "medium";
}) {
  const [copied, setCopied] = useState(false);

  async function handleCopy() {
    try {
      await navigator.clipboard.writeText(value);
    } catch {
      // Clipboard access can be denied on insecure origins; fall back silently.
      return;
    }
    setCopied(true);
    window.setTimeout(() => setCopied(false), 1800);
  }

  return (
    <>
      <Tooltip title={label}>
        <IconButton size={size} onClick={handleCopy} aria-label={label}>
          <ContentCopy fontSize="inherit" />
        </IconButton>
      </Tooltip>
      <Snackbar
        open={copied}
        autoHideDuration={1500}
        onClose={() => setCopied(false)}
        message="已复制到剪贴板"
        anchorOrigin={{ vertical: "bottom", horizontal: "center" }}
      />
    </>
  );
}

/** Read-only monospace block, used for links and rendered configs. */
export function MonoBlock({
  value,
  maxHeight = 320,
}: {
  value: string;
  maxHeight?: number | string;
}) {
  return (
    <Box
      component="pre"
      sx={{
        m: 0,
        p: 1.5,
        bgcolor: "grey.100",
        border: "1px solid",
        borderColor: "divider",
        borderRadius: 1,
        fontSize: 12.5,
        lineHeight: 1.55,
        maxHeight,
        overflow: "auto",
        whiteSpace: "pre-wrap",
        wordBreak: "break-all",
        fontFamily:
          'ui-monospace, SFMono-Regular, Menlo, Consolas, "Liberation Mono", monospace',
      }}
    >
      {value}
    </Box>
  );
}

export function SectionTitle({ children, hint }: { children: React.ReactNode; hint?: string }) {
  return (
    <Box sx={{ mb: 1.5 }}>
      <Typography variant="h6" component="h2">
        {children}
      </Typography>
      {hint && (
        <Typography variant="body2" color="text.secondary">
          {hint}
        </Typography>
      )}
    </Box>
  );
}
