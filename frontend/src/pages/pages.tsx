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
  { labelKey: "nav.nodes", path: "/", icon: <ViewList fontSize="small" /> },
  { labelKey: "nav.users", path: "/users", icon: <People fontSize="small" /> },
  { labelKey: "nav.settings", path: "/settings", icon: <Settings fontSize="small" /> },
];
