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
import { useTranslation } from "react-i18next";

import { useColorMode } from "./colorMode";
import { ACCENT_PRESETS, DEFAULT_ACCENT, normalizeAccent } from "../theme";

/**
 * Preset swatches plus a free-form hex field.
 *
 * The value is applied on every valid keystroke so the whole UI previews the
 * colour immediately; invalid input only marks the field, never the theme.
 */
export default function AccentPicker() {
  const { t } = useTranslation();
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
          const label = t(preset.labelKey);
          return (
            <ButtonBase
              key={preset.value}
              onClick={() => commit(preset.value)}
              aria-label={t("theme.accentAria", { name: label })}
              aria-pressed={selected}
              title={label}
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
          label={t("theme.customColor")}
          value={draft}
          onChange={(event) => commit(event.target.value)}
          error={!normalized}
          helperText={
            normalized ? t("theme.customColorHelper") : t("theme.customColorInvalid")
          }
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
          {t("theme.resetDefault")}
        </Link>
      </Box>
    </Stack>
  );
}
