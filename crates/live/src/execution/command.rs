// -------------------------------------------------------------------------------------------------
//  Copyright (C) 2015-2026 Nautech Systems Pty Ltd. All rights reserved.
//  https://nautechsystems.io
//
//  Licensed under the GNU Lesser General Public License Version 3.0 (the "License");
//  You may not use this file except in compliance with the License.
//  You may obtain a copy of the License at https://www.gnu.org/licenses/lgpl-3.0.en.html
//
//  Unless required by applicable law or agreed to in writing, software
//  distributed under the License is distributed on an "AS IS" BASIS,
//  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//  See the License for the specific language governing permissions and
//  limitations under the License.
// -------------------------------------------------------------------------------------------------

//! Diagnostics for cancel and modify commands with an unresolved venue outcome.

use nautilus_core::UnixNanos;
use nautilus_model::{
    enums::OrderStatus,
    identifiers::{
        AccountId, ClientId, ClientOrderId, InstrumentId, StrategyId, TraderId, VenueOrderId,
    },
};

/// A pending cancel or modify exhausted its automatic recovery budget.
///
/// This diagnostic is not an order event. The order retains its pending state,
/// confirmed fills and risk ownership until venue evidence resolves the command.
/// The live node publishes it after processing reconciliation events.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandRecoveryExhausted {
    /// Trader which owns the order.
    pub trader_id: TraderId,
    /// Execution client responsible for the order, when known.
    pub client_id: Option<ClientId>,
    /// Account which owns the order, when known.
    pub account_id: Option<AccountId>,
    /// Strategy which owns the order.
    pub strategy_id: StrategyId,
    /// Instrument traded by the order.
    pub instrument_id: InstrumentId,
    /// Original client order identity.
    pub client_order_id: ClientOrderId,
    /// Current venue order identity, when known.
    pub venue_order_id: Option<VenueOrderId>,
    /// Unresolved `PENDING_CANCEL` or `PENDING_UPDATE` state.
    pub order_status: OrderStatus,
    /// Native retry counter at exhaustion, not a count of wire requests.
    pub retry_count: u32,
    /// UNIX timestamp in nanoseconds when recovery was exhausted.
    pub ts_event: UnixNanos,
}
