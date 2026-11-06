/*
 * Copyright (C) 2026 Prescott Anderson
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * This file is part of NSM (NeoSubscriptionMastodon), licensed under the
 * GNU Affero General Public License v3.0 or later. See the LICENSE file
 * in the project root for the full text.
 */

import { createSlice, type PayloadAction } from "@reduxjs/toolkit";

export type NoticeSeverity = "success" | "error" | "warning" | "info";

export type Notice = {
  id: number;
  message: string;
  severity: NoticeSeverity;
};

type NoticesState = {
  latest: Notice | null;
};

const initialState: NoticesState = {
  latest: null,
};

let nextId = 1;

const noticesSlice = createSlice({
  name: "notices",
  initialState,
  reducers: {
    noticeShown(state, action: PayloadAction<Notice>) {
      state.latest = action.payload;
    },
    noticeDismissed(state, action: PayloadAction<number>) {
      if (state.latest?.id === action.payload) {
        state.latest = null;
      }
    },
  },
});

export const { noticeDismissed } = noticesSlice.actions;

/** Queues a notice; only the most recent one is displayed. */
export function noticeAdded(payload: { message: string; severity?: NoticeSeverity }) {
  const notice: Notice = {
    id: nextId,
    message: payload.message,
    severity: payload.severity ?? "info",
  };
  nextId += 1;
  return noticesSlice.actions.noticeShown(notice);
}

export const noticesReducer = noticesSlice.reducer;
