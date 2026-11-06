/*
 * Copyright (C) 2026 Prescott Anderson
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * This file is part of NSM (NeoSubscriptionMastodon), licensed under the
 * GNU Affero General Public License v3.0 or later. See the LICENSE file
 * in the project root for the full text.
 */

import { MenuItem, TextField } from "@mui/material";
import { useTranslation } from "react-i18next";

import { LANGUAGES, setLanguage, type LanguageCode } from "../i18n";

/**
 * Language picker.
 *
 * The choice is stored in `localStorage` rather than the database: it is a
 * display preference of this browser, like the colour mode.
 */
export default function LanguageSelect({ size = "small" }: { size?: "small" | "medium" }) {
  const { t, i18n } = useTranslation();
  const current = (LANGUAGES.find((item) => item.code === i18n.language)?.code ??
    "en") as LanguageCode;

  return (
    <TextField
      select
      size={size}
      label={t("language.label")}
      value={current}
      onChange={(event) => setLanguage(event.target.value as LanguageCode)}
      sx={{ minWidth: 130 }}
    >
      {LANGUAGES.map((language) => (
        <MenuItem key={language.code} value={language.code}>
          {language.label}
        </MenuItem>
      ))}
    </TextField>
  );
}
