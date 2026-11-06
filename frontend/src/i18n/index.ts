/*
 * Copyright (C) 2026 Prescott Anderson
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * This file is part of NSM (NeoSubscriptionMastodon), licensed under the
 * GNU Affero General Public License v3.0 or later. See the LICENSE file
 * in the project root for the full text.
 */

import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import en from "./locales/en.json";
import zhCN from "./locales/zh-CN.json";

/** Languages the UI ships with. English is the source of truth. */
export const LANGUAGES = [
  { code: "en", label: "English" },
  { code: "zh-CN", label: "简体中文" },
] as const;

export type LanguageCode = (typeof LANGUAGES)[number]["code"];

const STORAGE_KEY = "nsm.language";

/** Stored preference, falling back to English rather than the browser locale. */
function initialLanguage(): LanguageCode {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored && LANGUAGES.some((item) => item.code === stored)) {
      return stored as LanguageCode;
    }
  } catch {
    // Private mode or a disabled storage API: English is always safe.
  }
  return "en";
}

void i18n.use(initReactI18next).init({
  resources: {
    en: { translation: en },
    "zh-CN": { translation: zhCN },
  },
  lng: initialLanguage(),
  fallbackLng: "en",
  supportedLngs: LANGUAGES.map((item) => item.code),
  interpolation: {
    escapeValue: false,
  },
});

/** Switches language and remembers the choice for this browser. */
export function setLanguage(code: LanguageCode): void {
  void i18n.changeLanguage(code);
  try {
    localStorage.setItem(STORAGE_KEY, code);
  } catch {
    // Persisting is a nicety; the switch still applies for this session.
  }
}

export default i18n;
