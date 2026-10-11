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

//! Commands and delivery results for adapter-owned execution actions.

use std::fmt::Display;

use derive_builder::Builder;
use nautilus_core::{Params, UUID4, UnixNanos};
use nautilus_model::identifiers::{AccountId, ClientId, StrategyId, TraderId};
use serde::{Deserialize, Serialize};
use strum::{Display as StrumDisplay, EnumString};

/// The delivery outcome of an [`ExecuteAction`] command.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, StrumDisplay, EnumString,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(
        frozen,
        eq,
        module = "nautilus_trader.live",
        from_py_object,
        rename_all = "SCREAMING_SNAKE_CASE",
    )
)]
#[cfg_attr(
    feature = "python",
    pyo3_stub_gen::derive::gen_stub_pyclass_enum(module = "nautilus_trader.live")
)]
pub enum ExecutionActionStatus {
    /// The venue returned a definitive success response.
    Accepted,
    /// The command was not sent or the venue explicitly rejected it.
    Rejected,
    /// The request may have reached the venue, but no definitive response exists.
    #[default]
    Ambiguous,
}

/// A strategy-owned request for an adapter-defined execution action.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize, Builder)]
#[serde(tag = "type")]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "nautilus_trader.live", frozen, from_py_object)
)]
#[cfg_attr(
    feature = "python",
    pyo3_stub_gen::derive::gen_stub_pyclass(module = "nautilus_trader.live")
)]
pub struct ExecuteAction {
    pub trader_id: TraderId,
    pub client_id: Option<ClientId>,
    pub strategy_id: StrategyId,
    pub account_id: AccountId,
    pub action: String,
    pub command_id: UUID4,
    pub ts_init: UnixNanos,
    pub params: Option<Params>,
    #[builder(default)]
    pub correlation_id: Option<UUID4>,
    #[builder(default)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub causation_id: Option<UUID4>,
}

impl ExecuteAction {
    /// Creates a new [`ExecuteAction`] instance.
    #[expect(clippy::too_many_arguments)]
    #[must_use]
    pub fn new(
        trader_id: TraderId,
        client_id: Option<ClientId>,
        strategy_id: StrategyId,
        account_id: AccountId,
        action: impl Into<String>,
        command_id: UUID4,
        ts_init: UnixNanos,
        params: Option<Params>,
        correlation_id: Option<UUID4>,
    ) -> Self {
        Self {
            trader_id,
            client_id,
            strategy_id,
            account_id,
            action: action.into(),
            command_id,
            ts_init,
            params,
            correlation_id,
            causation_id: None,
        }
    }
}

impl Display for ExecuteAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ExecuteAction(client_id={:?}, account_id={}, action={})",
            self.client_id, self.account_id, self.action,
        )
    }
}

/// The correlated delivery result for an [`ExecuteAction`] command.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize, Builder)]
#[serde(tag = "type")]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "nautilus_trader.live", frozen, from_py_object)
)]
#[cfg_attr(
    feature = "python",
    pyo3_stub_gen::derive::gen_stub_pyclass(module = "nautilus_trader.live")
)]
pub struct ExecutionActionResult {
    pub trader_id: TraderId,
    pub client_id: Option<ClientId>,
    pub strategy_id: StrategyId,
    pub account_id: AccountId,
    pub action: String,
    pub command_id: UUID4,
    pub status: ExecutionActionStatus,
    pub reason: Option<String>,
    pub info: Option<Params>,
    pub ts_event: UnixNanos,
}

impl ExecutionActionResult {
    /// Creates a new result for `command`.
    #[must_use]
    pub fn new(
        command: &ExecuteAction,
        status: ExecutionActionStatus,
        reason: Option<String>,
        info: Option<Params>,
        ts_event: UnixNanos,
    ) -> Self {
        Self {
            trader_id: command.trader_id,
            client_id: command.client_id,
            strategy_id: command.strategy_id,
            account_id: command.account_id,
            action: command.action.clone(),
            command_id: command.command_id,
            status,
            reason,
            info,
            ts_event,
        }
    }
}

impl Display for ExecutionActionResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ExecutionActionResult(command_id={}, action={}, status={})",
            self.command_id, self.action, self.status,
        )
    }
}

#[cfg(test)]
mod tests {
    use nautilus_core::{UUID4, UnixNanos};
    use nautilus_model::identifiers::{AccountId, ClientId, StrategyId, TraderId};
    use rstest::rstest;

    use super::*;

    #[rstest]
    fn test_result_retains_command_identity() {
        let command = ExecuteAction::new(
            TraderId::from("TRADER-001"),
            Some(ClientId::from("HYPERLIQUID")),
            StrategyId::from("STRATEGY-001"),
            AccountId::from("HYPERLIQUID-001"),
            "hyperliquid.user_outcome.split",
            UUID4::from("00000000-0000-4000-8000-000000000001"),
            UnixNanos::from(1),
            None,
            None,
        );

        let result = ExecutionActionResult::new(
            &command,
            ExecutionActionStatus::Accepted,
            None,
            None,
            UnixNanos::from(2),
        );

        assert_eq!(result.command_id, command.command_id);
        assert_eq!(result.strategy_id, command.strategy_id);
        assert_eq!(result.status, ExecutionActionStatus::Accepted);
    }
}
