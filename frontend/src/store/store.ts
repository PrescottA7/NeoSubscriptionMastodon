/*
 * Copyright (C) 2026 Prescott Anderson
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * This file is part of NSM (NeoSubscriptionMastodon), licensed under the
 * GNU Affero General Public License v3.0 or later. See the LICENSE file
 * in the project root for the full text.
 */

import { configureStore } from "@reduxjs/toolkit";

import { authReducer } from "./authSlice";
import { catalogReducer } from "./catalogSlice";
import { noticesReducer } from "./noticesSlice";

export const store = configureStore({
  reducer: {
    auth: authReducer,
    catalog: catalogReducer,
    notices: noticesReducer,
  },
});

export type RootState = ReturnType<typeof store.getState>;
export type AppDispatch = typeof store.dispatch;
