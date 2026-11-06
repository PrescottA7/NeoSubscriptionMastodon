// Copyright (C) 2026 Prescott Anderson
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// This file is part of NSM (NeoSubscriptionMastodon), licensed under the
// GNU Affero General Public License v3.0 or later. See the LICENSE file
// in the project root for the full text.

//! Per-protocol parameter sets.

pub mod anytls;
pub mod hysteria2;
pub mod trojan;
pub mod tuic;
pub mod vless;

pub use anytls::AnytlsConfig;
pub use hysteria2::{Hysteria2Config, ObfsConfig, ObfsType, RealmConfig};
pub use trojan::TrojanConfig;
pub use tuic::{CongestionControl, TuicConfig, UdpRelayMode};
pub use vless::{VlessConfig, VlessFlow};
