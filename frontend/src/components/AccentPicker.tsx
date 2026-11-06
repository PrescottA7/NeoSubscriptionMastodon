/*
 * Copyright (C) 2026 Prescott Anderson
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * This file is part of NSM (NeoSubscriptionMastodon), licensed under the
 * GNU Affero General Public License v3.0 or later. See the LICENSE file
 * in the project root for the full text.
 */

import { useState } from "react";
import { Box, ButtonBase, Link, Stack, TextField } from "@mui/material";
import { Check } from "@mui/icons-material";

import { useColorMode } from "./colorMode";
import { ACCENT_PRESETS, DEFAULT_ACCENT, normalizeAccent } from "../theme";

/**
 * Preset swatches plus a free-form hex field.
 *
 * The value is applied on every valid keystroke so the whole UI previews the
 * colour immediately; invalid input only marks the field, never the theme.
 */
export default function AccentPicker() {
  const { accent, setAccent } = useColorMode();
  // Empty until the user types, so a change made elsewhere is reflected here.
  const [typed, setTyped] = useState("");
  const draft = typed || accent;

  const normalized = normalizeAccent(draft);

  function commit(value: string) {
    setTyped(value);
    const next = normalizeAccent(value);
    if (next) {
      setAccent(next);
    }
  }

  return (
    <Stack spacing={1.5}>
      <Stack direction="row" spacing={1} sx={{ flexWrap: "wrap", rowGap: 1 }}>
        {ACCENT_PRESETS.map((preset) => {
          const selected = preset.value === accent;
          return (
            <ButtonBase
              key={preset.value}
              onClick={() => commit(preset.value)}
              aria-label={`主题色：${preset.label}`}
              aria-pressed={selected}
              title={preset.label}
              sx={{
                width: 30,
                height: 30,
                borderRadius: 1,
                border: "1px solid",
                borderColor: selected ? "text.primary" : "divider",
                backgroundColor: preset.value,
                color: "#fff",
                "&:hover": { borderColor: "text.primary" },
              }}
            >
              {selected && <Check sx={{ fontSize: 16 }} />}
            </ButtonBase>
          );
        })}
      </Stack>

      <Box sx={{ display: "flex", alignItems: "center", gap: 1.5, flexWrap: "wrap" }}>
        <Box
          sx={{
            width: 30,
            height: 30,
            borderRadius: 1,
            border: "1px solid",
            borderColor: "divider",
            backgroundColor: normalized ?? "transparent",
            flexShrink: 0,
          }}
        />
        <TextField
          size="small"
          label="自定义颜色"
          value={draft}
          onChange={(event) => commit(event.target.value)}
          error={!normalized}
          helperText={normalized ? "十六进制色值，例如 #0078d4" : "请填写 #rrggbb 形式的十六进制色值"}
          sx={{ width: 220 }}
          slotProps={{ htmlInput: { maxLength: 7, spellCheck: false } }}
        />
        <Link
          component="button"
          type="button"
          variant="caption"
          onClick={() => commit(DEFAULT_ACCENT)}
          sx={{ color: "text.secondary" }}
        >
          恢复默认（微软蓝）
        </Link>
      </Box>
    </Stack>
  );
}
