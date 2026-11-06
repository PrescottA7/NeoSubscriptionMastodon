/*
 * Copyright (C) 2026 Prescott Anderson
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * This file is part of NSM (NeoSubscriptionMastodon), licensed under the
 * GNU Affero General Public License v3.0 or later. See the LICENSE file
 * in the project root for the full text.
 */

import { createAsyncThunk, createSlice } from "@reduxjs/toolkit";

import { listNodes, listProtocols, listUsers } from "../api";
import i18n from "../i18n";
import type { Node, ProtocolDescriptor, User } from "../types";

type CatalogState = {
  users: User[];
  nodes: Node[];
  protocols: ProtocolDescriptor[];
  loading: boolean;
  loaded: boolean;
  error: string | null;
};

const initialState: CatalogState = {
  users: [],
  nodes: [],
  protocols: [],
  loading: false,
  loaded: false,
  error: null,
};

/** Loads every list the admin UI renders, in parallel. */
export const loadCatalog = createAsyncThunk("catalog/load", async () => {
  const [users, nodes, protocols] = await Promise.all([listUsers(), listNodes(), listProtocols()]);
  return { users, nodes, protocols };
});

const catalogSlice = createSlice({
  name: "catalog",
  initialState,
  reducers: {
    catalogReset(state) {
      state.users = [];
      state.nodes = [];
      state.protocols = [];
      state.loaded = false;
      state.error = null;
    },
    usersReplaced(state, action: { payload: User[] }) {
      state.users = action.payload;
    },
    nodesReplaced(state, action: { payload: Node[] }) {
      state.nodes = action.payload;
    },
    nodeUpdated(state, action: { payload: Node }) {
      const index = state.nodes.findIndex((node) => node.id === action.payload.id);
      if (index >= 0) {
        state.nodes[index] = action.payload;
      }
    },
  },
  extraReducers: (builder) => {
    builder
      .addCase(loadCatalog.pending, (state) => {
        state.loading = true;
        state.error = null;
      })
      .addCase(loadCatalog.fulfilled, (state, action) => {
        state.users = action.payload.users;
        state.nodes = action.payload.nodes;
        state.protocols = action.payload.protocols;
        state.loading = false;
        state.loaded = true;
      })
      .addCase(loadCatalog.rejected, (state, action) => {
        state.loading = false;
        state.error = action.error.message ?? i18n.t("common.loadFailed");
      });
  },
});

export const { catalogReset, usersReplaced, nodesReplaced, nodeUpdated } = catalogSlice.actions;
export const catalogReducer = catalogSlice.reducer;
