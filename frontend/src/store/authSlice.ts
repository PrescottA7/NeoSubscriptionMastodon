/*
 * Copyright (C) 2026 Prescott Anderson
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * This file is part of NSM (NeoSubscriptionMastodon), licensed under the
 * GNU Affero General Public License v3.0 or later. See the LICENSE file
 * in the project root for the full text.
 */

import { createAsyncThunk, createSlice } from "@reduxjs/toolkit";

import { getSetupStatus, isUnauthorizedError, login, logout, me, setupAdmin } from "../api";
import type { Admin } from "../types";

type AuthStatus = "idle" | "loading" | "authenticated" | "anonymous";

type AuthState = {
  status: AuthStatus;
  initialized: boolean;
  admin: Admin | null;
  needsSetup: boolean;
  siteName: string;
  error: string | null;
};

const initialState: AuthState = {
  status: "idle",
  initialized: false,
  admin: null,
  needsSetup: false,
  siteName: "NSM",
  error: null,
};

export const bootstrapAuth = createAsyncThunk<{
  admin: Admin | null;
  needsSetup: boolean;
  siteName: string;
}>("auth/bootstrap", async () => {
  const status = await getSetupStatus();
  if (status.needs_setup) {
    return { admin: null, needsSetup: true, siteName: status.site_name };
  }

  try {
    const admin = await me();
    return { admin, needsSetup: false, siteName: status.site_name };
  } catch (error) {
    if (isUnauthorizedError(error)) {
      return { admin: null, needsSetup: false, siteName: status.site_name };
    }
    throw error;
  }
});

export const loginAction = createAsyncThunk<Admin, { username: string; password: string }>(
  "auth/login",
  async ({ username, password }) => login(username, password),
);

export const setupAction = createAsyncThunk<Admin, { username: string; password: string }>(
  "auth/setup",
  async ({ username, password }) => setupAdmin(username, password),
);

export const logoutAction = createAsyncThunk<void>("auth/logout", async () => {
  await logout();
});

const authSlice = createSlice({
  name: "auth",
  initialState,
  reducers: {
    sessionExpired(state) {
      state.status = "anonymous";
      state.admin = null;
      state.initialized = true;
    },
  },
  extraReducers: (builder) => {
    builder
      .addCase(bootstrapAuth.pending, (state) => {
        state.status = "loading";
      })
      .addCase(bootstrapAuth.fulfilled, (state, action) => {
        state.admin = action.payload.admin;
        state.needsSetup = action.payload.needsSetup;
        state.siteName = action.payload.siteName;
        state.status = action.payload.admin ? "authenticated" : "anonymous";
        state.initialized = true;
        state.error = null;
      })
      .addCase(bootstrapAuth.rejected, (state, action) => {
        state.status = "anonymous";
        state.initialized = true;
        state.error = action.error.message ?? "无法连接服务器";
      })
      .addCase(loginAction.pending, (state) => {
        state.error = null;
      })
      .addCase(loginAction.fulfilled, (state, action) => {
        state.admin = action.payload;
        state.status = "authenticated";
        state.initialized = true;
        state.needsSetup = false;
      })
      .addCase(loginAction.rejected, (state, action) => {
        state.error = action.error.message ?? "登录失败";
      })
      .addCase(setupAction.fulfilled, (state, action) => {
        state.admin = action.payload;
        state.status = "authenticated";
        state.initialized = true;
        state.needsSetup = false;
      })
      .addCase(setupAction.rejected, (state, action) => {
        state.error = action.error.message ?? "初始化失败";
      })
      .addCase(logoutAction.fulfilled, (state) => {
        state.admin = null;
        state.status = "anonymous";
      });
  },
});

export const { sessionExpired } = authSlice.actions;
export const authReducer = authSlice.reducer;
