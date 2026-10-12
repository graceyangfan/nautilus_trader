#!/usr/bin/env python3
# -------------------------------------------------------------------------------------------------
#  Copyright (C) 2015-2026 Nautech Systems Pty Ltd. All rights reserved.
#  https://nautechsystems.io
#
#  Licensed under the GNU Lesser General Public License Version 3.0 (the "License");
#  you may not use this file except in compliance with the License.
#  You may obtain a copy of the License at https://www.gnu.org/licenses/lgpl-3.0.en.html
#
#  Unless required by applicable law or agreed to in writing, software
#  distributed under the License is distributed on an "AS IS" BASIS,
#  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
#  See the License for the specific language governing permissions and
#  limitations under the License.
# -------------------------------------------------------------------------------------------------
"""
Validate Hyperliquid HIP-4 split and merge actions on testnet.

The tester sends one split, waits for its accepted delivery result and the
two external Yes/No fills, then sends the matching merge. It logs every action
result, fill, and position callback. The execution engine owns the resulting
account and position updates; this strategy only observes them.

The tester is intentionally testnet-only and refuses to submit an action
unless ``HYPERLIQUID_OUTCOME_ACTION_EXECUTE=1`` is set. It selects the
nearest-expiring loaded HIP-4 BTC daily price-binary (up/down) outcome with
sufficient remaining lifetime, or an explicitly requested active outcome.
Use an outcome with no pre-existing inventory or outstanding orders on either
leg.

Required environment variables:

- ``HYPERLIQUID_TESTNET_PK``
- ``HYPERLIQUID_OUTCOME_ACTION_QTY``
- ``HYPERLIQUID_OUTCOME_ACTION_EXECUTE=1``

Set ``HYPERLIQUID_ACCOUNT_ADDRESS`` too when using an API wallet. Stop with
Ctrl-C after the merge callbacks have been observed.

Optional environment variables:

- ``HYPERLIQUID_OUTCOME_ACTION_OUTCOME``: select this loaded active outcome ID
- ``HYPERLIQUID_OUTCOME_ACTION_MIN_REMAINING_SECS``: minimum lifetime (default ``300``)

"""

from __future__ import annotations

import os

from nautilus_trader.adapters.hyperliquid import HyperliquidDataClientConfig
from nautilus_trader.adapters.hyperliquid import HyperliquidDataClientFactory
from nautilus_trader.adapters.hyperliquid import HyperliquidEnvironment
from nautilus_trader.adapters.hyperliquid import HyperliquidExecutionClientConfig
from nautilus_trader.adapters.hyperliquid import HyperliquidExecutionClientFactory
from nautilus_trader.common import Environment
from nautilus_trader.config import LiveRiskEngineConfig
from nautilus_trader.live import BinaryOptionAction
from nautilus_trader.live import ExecutionActionResult
from nautilus_trader.live import ExecutionActionStatus
from nautilus_trader.live import LiveNode
from nautilus_trader.model import AccountId
from nautilus_trader.model import BinaryOption
from nautilus_trader.model import ClientId
from nautilus_trader.model import InstrumentId
from nautilus_trader.model import OmsType
from nautilus_trader.model import OrderFilled
from nautilus_trader.model import PositionChanged
from nautilus_trader.model import PositionClosed
from nautilus_trader.model import PositionOpened
from nautilus_trader.model import Quantity
from nautilus_trader.model import StrategyId
from nautilus_trader.model import TraderId
from nautilus_trader.model import Venue
from nautilus_trader.trading import Strategy
from nautilus_trader.trading import StrategyConfig


HYPERLIQUID = "HYPERLIQUID"
VENUE = Venue(HYPERLIQUID)
CLIENT_ID = ClientId.from_str(HYPERLIQUID)
TRADER_ID = TraderId.from_str("HIP4-TESTNET-001")
ACCOUNT_ID = AccountId.from_str("HYPERLIQUID-001")
STRATEGY_ID = StrategyId.from_str("HIP4_OUTCOME_ACTIONS-001")


def _required_env(name: str) -> str:
    value = os.environ.get(name)
    if not value:
        raise RuntimeError(f"Missing required environment variable: {name}")
    return value


class OutcomeActionsTesterConfig(StrategyConfig):
    """
    Collect the test strategy's action inputs.
    """

    def __init__(
        self,
        *,
        account_id: AccountId,
        client_id: ClientId,
        quantity: Quantity,
        requested_outcome: int | None,
        min_remaining_secs: int,
        **_kwargs: object,
    ) -> None:
        """
        Initialize the test strategy configuration.
        """
        super().__init__()
        self.account_id = account_id
        self.client_id = client_id
        self.quantity = quantity
        self.requested_outcome = requested_outcome
        self.min_remaining_secs = min_remaining_secs


class OutcomeActionsTester(Strategy):
    """
    Test a single matching HIP-4 split and merge cycle.
    """

    def __init__(
        self,
        config: OutcomeActionsTesterConfig,
    ) -> None:
        """
        Initialize the test strategy before resolving its active outcome pair.
        """
        super().__init__(config)
        self._account_id = config.account_id
        self._client_id = config.client_id
        self._quantity = config.quantity
        self._requested_outcome = config.requested_outcome
        self._min_remaining_ns = config.min_remaining_secs * 1_000_000_000
        self._pair: tuple[InstrumentId, InstrumentId] | None = None
        self._pair_instrument_ids: frozenset[InstrumentId] = frozenset()
        self._split_command_id = None
        self._merge_command_id = None
        self._split_accepted = False
        self._merge_accepted = False
        self._split_filled_legs: set[InstrumentId] = set()
        self._merge_filled_legs: set[InstrumentId] = set()
        self._split_sent_ns: int | None = None
        self._merge_sent_ns: int | None = None
        self._last_fill_action_by_instrument: dict[InstrumentId, str] = {}

    def on_start(self) -> None:
        """
        Select the current active pair, claim its external fills, and submit a split.
        """
        selected = self._select_active_pair()
        if selected is None:
            log_msg = "No active HIP-4 outcome pair satisfied the selection criteria"
            self.log.error(log_msg)
            self.stop()
            return

        outcome, yes_instrument_id, no_instrument_id, expiration_ns = selected
        self._pair = (yes_instrument_id, no_instrument_id)
        self._pair_instrument_ids = frozenset(self._pair)
        # Venue-generated fills have no client order ID. These claims associate the
        # materialized external orders, fills, and positions with this strategy.
        self.set_external_order_instrument_ids(list(self._pair))
        log_msg = (
            f"ACTIVE_OUTCOME_SELECTED outcome={outcome} yes={yes_instrument_id} "
            f"no={no_instrument_id} expiration_ns={expiration_ns}"
        )
        self.log.info(log_msg)
        self._log_cached_state("BEFORE_SPLIT")
        action = BinaryOptionAction.split(
            yes_instrument_id=yes_instrument_id,
            no_instrument_id=no_instrument_id,
            quantity=self._quantity,
        )
        self._split_sent_ns = self.clock.timestamp_ns()
        self._split_command_id = self.execute_action(
            account_id=self._account_id,
            client_id=self._client_id,
            action=action,
        )
        log_msg = (
            f"SPLIT_SUBMITTED command_id={self._split_command_id} quantity={self._quantity} "
            f"yes={yes_instrument_id} no={no_instrument_id}"
        )
        self.log.info(log_msg)

    def on_execution_action_result(self, result: ExecutionActionResult) -> None:
        """
        Log the delivery acknowledgement and advance only after split fills converge.
        """
        log_msg = (
            f"ACTION_RESULT command_id={result.command_id} action={result.action} "
            f"status={result.status} reason={result.reason} info={result.info}"
        )
        self.log.info(log_msg)

        if result.command_id == self._split_command_id:
            self._log_latency("ACK_RECEIVED", "SPLIT")
            self._split_accepted = result.status == ExecutionActionStatus.ACCEPTED
            self._log_cached_state("SPLIT_ACK")
            if not self._split_accepted:
                self.log.error("Split was not accepted; merge will not be submitted")
                return
            self._submit_merge_when_split_converged()
        elif result.command_id == self._merge_command_id:
            self._log_latency("ACK_RECEIVED", "MERGE")
            self._merge_accepted = result.status == ExecutionActionStatus.ACCEPTED
            self._log_cached_state("MERGE_ACK")
            if not self._merge_accepted:
                self.log.error(
                    "Merge was not accepted; inspect the two leg fills and account state",
                )
            elif self._merge_filled_legs == self._pair_instrument_ids:
                self.log.info(
                    "HIP-4 split/merge cycle completed; inspect position callbacks above",
                )

        # The ACK does not carry balances. Requesting a snapshot is a separate
        # reconciliation observation, not a replacement for the fill-driven update path.
        self.query_account(account_id=self._account_id, client_id=self._client_id)

    def on_order_filled(self, event: OrderFilled) -> None:
        """
        Observe the venue-generated external fills for the action's two legs.
        """
        if event.instrument_id not in self._pair_instrument_ids:
            return

        if self._split_command_id is None:
            self.log.warning("Ignoring outcome fill received before split submission")
            return

        action = "SPLIT" if self._merge_command_id is None else "MERGE"
        self._last_fill_action_by_instrument[event.instrument_id] = action

        log_msg = (
            f"ORDER_FILLED action={action} instrument={event.instrument_id} "
            f"order_id={event.client_order_id} "
            f"side={event.order_side} quantity={event.last_qty} price={event.last_px} "
            f"commission={event.commission} trade_id={event.trade_id}"
        )
        self.log.info(log_msg)
        self._log_latency("FILL_RECEIVED", action)

        if event.last_qty != self._quantity:
            log_msg = (
                f"Unexpected HIP-4 fill quantity: expected={self._quantity} actual={event.last_qty}"
            )
            self.log.warning(log_msg)
            return

        if action == "SPLIT":
            self._split_filled_legs.add(event.instrument_id)
            self._submit_merge_when_split_converged()
        else:
            self._merge_filled_legs.add(event.instrument_id)
            if self._merge_accepted and self._merge_filled_legs == self._pair_instrument_ids:
                self.log.info(
                    "HIP-4 split/merge cycle completed; inspect position callbacks above",
                )

        self._log_cached_state("FILL")

    def on_position_opened(self, event: PositionOpened) -> None:
        """
        Log a newly materialized outcome position.
        """
        self._log_position_event("POSITION_OPENED", event)

    def on_position_changed(self, event: PositionChanged) -> None:
        """
        Log an updated outcome position.
        """
        self._log_position_event("POSITION_CHANGED", event)

    def on_position_closed(self, event: PositionClosed) -> None:
        """
        Log a closed outcome position.
        """
        self._log_position_event("POSITION_CLOSED", event)

    def _submit_merge_when_split_converged(self) -> None:
        if not self._split_accepted or self._merge_command_id is not None:
            return
        if self._split_filled_legs != self._pair_instrument_ids:
            return

        yes_instrument_id, no_instrument_id = self._require_pair()
        action = BinaryOptionAction.merge(
            yes_instrument_id=yes_instrument_id,
            no_instrument_id=no_instrument_id,
            quantity=self._quantity,
        )
        self._merge_sent_ns = self.clock.timestamp_ns()
        self._merge_command_id = self.execute_action(
            account_id=self._account_id,
            client_id=self._client_id,
            action=action,
        )
        log_msg = (
            f"MERGE_SUBMITTED command_id={self._merge_command_id} quantity={self._quantity} "
            f"yes={yes_instrument_id} no={no_instrument_id}"
        )
        self.log.info(log_msg)

    def _select_active_pair(self) -> tuple[int, InstrumentId, InstrumentId, int] | None:
        now_ns = self.clock.timestamp_ns()
        min_expiration_ns = now_ns + self._min_remaining_ns
        pairs: dict[int, dict[int, BinaryOption]] = {}
        for instrument in self.cache.instruments(VENUE):
            if not isinstance(instrument, BinaryOption):
                continue

            info = instrument.info
            outcome = info.get("outcome_index")
            side = info.get("outcome_side")
            if (
                info.get("underlying") != "BTC"
                or info.get("period") != "1d"
                or info.get("class") != "priceBinary"
            ):
                continue
            if not isinstance(outcome, int) or not isinstance(side, int):
                continue
            if side not in (0, 1) or instrument.expiration_ns <= min_expiration_ns:
                continue
            if self._requested_outcome is not None and outcome != self._requested_outcome:
                continue

            pairs.setdefault(outcome, {})[side] = instrument

        candidates = []

        for outcome, sides in pairs.items():
            yes = sides.get(0)
            no = sides.get(1)
            if yes is None or no is None or yes.expiration_ns != no.expiration_ns:
                continue
            candidates.append((yes.expiration_ns, outcome, yes.id, no.id))

        if not candidates:
            return None

        expiration_ns, outcome, yes_instrument_id, no_instrument_id = min(candidates)
        return outcome, yes_instrument_id, no_instrument_id, expiration_ns

    def _require_pair(self) -> tuple[InstrumentId, InstrumentId]:
        if self._pair is None:
            raise RuntimeError("HIP-4 outcome pair was not selected")
        return self._pair

    def _log_position_event(
        self,
        name: str,
        event: PositionOpened | PositionChanged | PositionClosed,
    ) -> None:
        if event.instrument_id not in self._pair_instrument_ids:
            return

        action = self._last_fill_action_by_instrument.get(event.instrument_id)
        log_msg = (
            f"{name} action={action or 'UNKNOWN'} instrument={event.instrument_id} "
            f"position_id={event.position_id} "
            f"side={event.side} quantity={event.quantity} last_qty={event.last_qty} "
            f"last_px={event.last_px} ts_event={event.ts_event}"
        )
        self.log.info(log_msg)
        if action is not None:
            self._log_latency(f"{name}_RECEIVED", action)
        self._log_cached_state(name)

    def _log_latency(self, event: str, action: str) -> None:
        received_ns = self.clock.timestamp_ns()
        sent_ns = self._split_sent_ns if action == "SPLIT" else self._merge_sent_ns
        if sent_ns is None:
            return

        elapsed_ms = (received_ns - sent_ns) / 1_000_000
        log_msg = (
            f"{event} action={action} elapsed_ms={elapsed_ms:.3f} "
            f"sent_ns={sent_ns} received_ns={received_ns}"
        )
        self.log.info(log_msg)

    def _log_cached_state(self, source: str) -> None:
        account = self.cache.account(self._account_id)
        log_msg = f"{source}_ACCOUNT {account}"
        self.log.info(log_msg)
        for instrument_id in self._require_pair():
            positions = self.cache.positions_open(
                instrument_id=instrument_id,
                strategy_id=STRATEGY_ID,
                account_id=self._account_id,
            )
            log_msg = f"{source}_POSITIONS instrument={instrument_id} positions={positions}"
            self.log.info(log_msg)


def main() -> None:
    """
    Build and run the testnet-only live node.
    """
    if os.environ.get("HYPERLIQUID_OUTCOME_ACTION_EXECUTE") != "1":
        raise RuntimeError(
            "Refusing to submit testnet actions; set HYPERLIQUID_OUTCOME_ACTION_EXECUTE=1",
        )

    quantity = Quantity.from_str(_required_env("HYPERLIQUID_OUTCOME_ACTION_QTY"))

    if not quantity.is_positive():
        raise ValueError("HYPERLIQUID_OUTCOME_ACTION_QTY must be positive")
    requested_outcome = os.environ.get("HYPERLIQUID_OUTCOME_ACTION_OUTCOME")
    min_remaining_secs = int(
        os.environ.get("HYPERLIQUID_OUTCOME_ACTION_MIN_REMAINING_SECS", "300"),
    )

    if min_remaining_secs <= 0:
        raise ValueError(
            "HYPERLIQUID_OUTCOME_ACTION_MIN_REMAINING_SECS must be positive",
        )

    node = (
        LiveNode.builder("HYPERLIQUID-HIP4-TESTNET-001", TRADER_ID, Environment.LIVE)
        .with_reconciliation(reconciliation=True)
        .with_risk_engine_config(LiveRiskEngineConfig(bypass=True))
        .add_data_client(
            None,
            HyperliquidDataClientFactory(),
            HyperliquidDataClientConfig(environment=HyperliquidEnvironment.TESTNET),
        )
        .add_exec_client(
            None,
            HyperliquidExecutionClientFactory(),
            HyperliquidExecutionClientConfig(
                account_id=ACCOUNT_ID,
                environment=HyperliquidEnvironment.TESTNET,
            ),
        )
        .build()
    )
    node.add_strategy(
        OutcomeActionsTester(
            OutcomeActionsTesterConfig(
                strategy_id=STRATEGY_ID,
                oms_type=OmsType.NETTING,
                account_id=ACCOUNT_ID,
                client_id=CLIENT_ID,
                quantity=quantity,
                requested_outcome=int(requested_outcome) if requested_outcome is not None else None,
                min_remaining_secs=min_remaining_secs,
            ),
        ),
    )
    node.run()


if __name__ == "__main__":
    main()
