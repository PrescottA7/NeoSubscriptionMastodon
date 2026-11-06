/*
 * Copyright (C) 2026 Prescott Anderson
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * This file is part of NSM (NeoSubscriptionMastodon), licensed under the
 * GNU Affero General Public License v3.0 or later. See the LICENSE file
 * in the project root for the full text.
 */

import { People, Settings, ViewList } from "@mui/icons-material";

import type { PageInfo } from "../components/SideMenu";

export const pageList: PageInfo[] = [
  { label: "节点", path: "/", icon: <ViewList fontSize="small" /> },
  { label: "用户", path: "/users", icon: <People fontSize="small" /> },
  { label: "设置", path: "/settings", icon: <Settings fontSize="small" /> },
];
