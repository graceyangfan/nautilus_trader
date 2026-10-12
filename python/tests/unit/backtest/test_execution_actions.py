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
Tests Python strategy execution-action routing through a backtest engine.
"""

from decimal import Decimal

import pytest

from nautilus_trader.adapters.hyperliquid import hyperliquid_outcome_action_profile
from nautilus_trader.backtest import BacktestEngine
from nautilus_trader.backtest import BacktestEngineConfig
from nautilus_trader.execution import MakerTakerFeeModel
from nautilus_trader.live import BinaryOptionAction
from nautilus_trader.live import ExecutionActionStatus
from nautilus_trader.model import AccountId
from nautilus_trader.model import AccountType
from nautilus_trader.model import AssetClass
from nautilus_trader.model import BinaryOption
from nautilus_trader.model import Currency
from nautilus_trader.model import InstrumentId
from nautilus_trader.model import Money
from nautilus_trader.model import OmsType
from nautilus_trader.model import Price
from nautilus_trader.model import Quantity
from nautilus_trader.model import QuoteTick
from nautilus_trader.model import StrategyId
from nautilus_trader.model import Venue
from nautilus_trader.trading import Strategy
from nautilus_trader.trading import StrategyConfig
from tests.providers import TestInstrumentProvider


USDT = Currency.from_str("USDT")
USDC = Currency.from_str("USDC")
ETHUSDT = TestInstrumentProvider.ethusdt_binance()
ACCOUNT_ID = AccountId.from_str("BINANCE-001")
YES_INSTRUMENT_ID = InstrumentId.from_str("42-YES-OUTCOME.BINANCE")
NO_INSTRUMENT_ID = InstrumentId.from_str("42-NO-OUTCOME.BINANCE")
HYPERLIQUID_ACCOUNT_ID = AccountId.from_str("HYPERLIQUID-001")
HYPERLIQUID_YES_INSTRUMENT_ID = InstrumentId.from_str("42-YES-OUTCOME.HYPERLIQUID")
HYPERLIQUID_NO_INSTRUMENT_ID = InstrumentId.from_str("42-NO-OUTCOME.HYPERLIQUID")


class _ExecutionActionStrategy(Strategy):
    """
    Submits a typed action once and records its correlated result.
    """

    def __init__(self) -> None:
        """
        Initialize the strategy.
        """
        super().__init__(
            StrategyConfig(
                strategy_id=StrategyId("PY-ACTION-001"),
                log_commands=False,
                log_events=False,
            ),
        )
        self.command_id = None
        self.results = []

    def on_start(self) -> None:
        """
        Subscribe to the trigger quote.
        """
        self.subscribe_quotes(ETHUSDT.id)

    def on_quote(self, _quote: QuoteTick) -> None:
        """
        Submit one typed binary-option action.
        """
        if self.command_id is not None:
            return

        action = BinaryOptionAction.split(
            yes_instrument_id=YES_INSTRUMENT_ID,
            no_instrument_id=NO_INSTRUMENT_ID,
            quantity=Quantity.from_str("1.25"),
        )
        self.command_id = self.execute_action(
            account_id=ACCOUNT_ID,
            action=action,
        )

    def on_execution_action_result(self, result: object) -> None:
        """
        Record the correlated action result.
        """
        self.results.append(result)


class _SplitThenMergeStrategy(Strategy):
    """
    Exercises the Python action API through accepted split and merge callbacks.
    """

    def __init__(self) -> None:
        """
        Initialize the strategy.
        """
        super().__init__(
            StrategyConfig(
                strategy_id=StrategyId("PY-HIP4-ACTION-001"),
                log_commands=False,
                log_events=False,
            ),
        )
        self.command_id = None
        self.results = []
        self.position_events = []

    def on_start(self) -> None:
        """
        Subscribe to the Yes-leg trigger quote.
        """
        self.subscribe_quotes(HYPERLIQUID_YES_INSTRUMENT_ID)

    def on_quote(self, _quote: QuoteTick) -> None:
        """
        Submit the initial split once.
        """
        if self.command_id is not None:
            return

        self.command_id = self.execute_action(
            account_id=HYPERLIQUID_ACCOUNT_ID,
            action=BinaryOptionAction.split(
                yes_instrument_id=HYPERLIQUID_YES_INSTRUMENT_ID,
                no_instrument_id=HYPERLIQUID_NO_INSTRUMENT_ID,
                quantity=Quantity.from_str("1.25"),
            ),
        )

    def on_execution_action_result(self, result: object) -> None:
        """
        Merge only after the split's accepted result follows its position events.
        """
        self.results.append(result)
        if len(self.results) != 1 or result.status != ExecutionActionStatus.ACCEPTED:
            return

        self.command_id = self.execute_action(
            account_id=HYPERLIQUID_ACCOUNT_ID,
            action=BinaryOptionAction.merge(
                yes_instrument_id=HYPERLIQUID_YES_INSTRUMENT_ID,
                no_instrument_id=HYPERLIQUID_NO_INSTRUMENT_ID,
                quantity=Quantity.from_str("1.25"),
            ),
        )

    def on_position_opened(self, event: object) -> None:
        """
        Record an orderless split leg opening.
        """
        self.position_events.append(("opened", event.instrument_id))

    def on_position_closed(self, event: object) -> None:
        """
        Record an orderless merge leg closing.
        """
        self.position_events.append(("closed", event.instrument_id))


def _binary_option(instrument_id: InstrumentId, outcome: str) -> BinaryOption:
    """
    Create a current HIP-4-shaped binary outcome instrument.
    """
    return BinaryOption(
        instrument_id=instrument_id,
        raw_symbol=instrument_id.symbol,
        asset_class=AssetClass.ALTERNATIVE,
        currency=USDC,
        activation_ns=0,
        expiration_ns=1_000_000_000,
        price_precision=2,
        size_precision=2,
        price_increment=Price.from_str("0.01"),
        size_increment=Quantity.from_str("0.01"),
        ts_event=0,
        ts_init=0,
        outcome=outcome,
    )


def test_python_binary_option_action_factories_preserve_standard_payload() -> None:
    """
    Test Python action factories preserve standard action identity and validation.
    """
    split = BinaryOptionAction.split(
        yes_instrument_id=YES_INSTRUMENT_ID,
        no_instrument_id=NO_INSTRUMENT_ID,
        quantity=Quantity.from_str("1.25"),
        params={},
    )
    merge = BinaryOptionAction.merge(
        yes_instrument_id=YES_INSTRUMENT_ID,
        no_instrument_id=NO_INSTRUMENT_ID,
        quantity=Quantity.from_str("1.25"),
    )

    assert split.name == "binary_option.split"
    assert split.yes_instrument_id == YES_INSTRUMENT_ID
    assert split.no_instrument_id == NO_INSTRUMENT_ID
    assert split.quantity == Quantity.from_str("1.25")
    assert split.params is None
    assert merge.name == "binary_option.merge"
    assert merge.quantity == Quantity.from_str("1.25")

    with pytest.raises(ValueError, match="must be positive"):
        BinaryOptionAction.split(
            yes_instrument_id=YES_INSTRUMENT_ID,
            no_instrument_id=NO_INSTRUMENT_ID,
            quantity=Quantity.from_str("0"),
        )

    with pytest.raises(ValueError, match="must be positive"):
        BinaryOptionAction.merge(
            yes_instrument_id=YES_INSTRUMENT_ID,
            no_instrument_id=NO_INSTRUMENT_ID,
            quantity=Quantity.from_str("0"),
        )

    with pytest.raises(TypeError, match="required positional argument"):
        BinaryOptionAction.merge(
            yes_instrument_id=YES_INSTRUMENT_ID,
            no_instrument_id=NO_INSTRUMENT_ID,
        )

    with pytest.raises(TypeError, match="Quantity"):
        BinaryOptionAction.merge(
            yes_instrument_id=YES_INSTRUMENT_ID,
            no_instrument_id=NO_INSTRUMENT_ID,
            quantity=None,
        )

    with pytest.raises(ValueError, match="must differ"):
        BinaryOptionAction.split(
            yes_instrument_id=YES_INSTRUMENT_ID,
            no_instrument_id=YES_INSTRUMENT_ID,
            quantity=Quantity.from_str("1.25"),
        )


def test_python_strategy_receives_rejection_for_unconfigured_execution_action() -> None:
    """
    Test the public Python action API reaches the backtest callback unchanged.

    The default venue intentionally has no binary-option action profile, so this test
    asserts the safe pre-mutation rejection path. Configured profile success, account
    updates, and position events are covered by the HIP-4 Rust integration test.

    """
    engine = BacktestEngine(BacktestEngineConfig(bypass_logging=True, run_analysis=False))
    engine.add_venue(
        venue=Venue("BINANCE"),
        oms_type=OmsType.NETTING,
        account_type=AccountType.CASH,
        starting_balances=[Money(1_000.0, USDT)],
        base_currency=None,
        fee_model=MakerTakerFeeModel(
            maker_rate=Decimal(0),
            taker_rate=Decimal(0),
        ),
    )
    engine.add_instrument(ETHUSDT)
    strategy = _ExecutionActionStrategy()
    engine.add_strategy(strategy)

    try:
        engine.add_data(
            [
                QuoteTick(
                    instrument_id=ETHUSDT.id,
                    bid_price=Price.from_str("2000.00"),
                    ask_price=Price.from_str("2000.10"),
                    bid_size=Quantity.from_str("10"),
                    ask_size=Quantity.from_str("10"),
                    ts_event=1,
                    ts_init=1,
                ),
            ],
        )
        engine.run()

        assert strategy.command_id is not None
        assert len(strategy.results) == 1
        result = strategy.results[0]
        assert result.command_id == strategy.command_id
        assert result.action == "binary_option.split"
        assert result.status == ExecutionActionStatus.REJECTED
    finally:
        engine.dispose()


def test_python_strategy_executes_hip4_split_and_merge_with_profile() -> None:
    """
    Test Python position events precede accepted split and merge results.
    """
    engine = BacktestEngine(BacktestEngineConfig(bypass_logging=True, run_analysis=False))
    engine.add_venue(
        venue=Venue("HYPERLIQUID"),
        oms_type=OmsType.NETTING,
        account_type=AccountType.CASH,
        starting_balances=[Money(10.0, USDC)],
        base_currency=USDC,
        fee_model=MakerTakerFeeModel(
            maker_rate=Decimal(0),
            taker_rate=Decimal(0),
        ),
        binary_option_action_profile=hyperliquid_outcome_action_profile(
            Decimal(0),
            Decimal(0),
        ),
    )
    engine.add_instrument(_binary_option(HYPERLIQUID_YES_INSTRUMENT_ID, "Yes"))
    engine.add_instrument(_binary_option(HYPERLIQUID_NO_INSTRUMENT_ID, "No"))
    strategy = _SplitThenMergeStrategy()
    engine.add_strategy(strategy)

    try:
        engine.add_data(
            [
                QuoteTick(
                    instrument_id=HYPERLIQUID_YES_INSTRUMENT_ID,
                    bid_price=Price.from_str("0.40"),
                    ask_price=Price.from_str("0.41"),
                    bid_size=Quantity.from_str("100.00"),
                    ask_size=Quantity.from_str("100.00"),
                    ts_event=1,
                    ts_init=1,
                ),
            ],
        )
        engine.run()

        assert [result.status for result in strategy.results] == [
            ExecutionActionStatus.ACCEPTED,
            ExecutionActionStatus.ACCEPTED,
        ]
        assert [result.action for result in strategy.results] == [
            "binary_option.split",
            "binary_option.merge",
        ]
        assert strategy.position_events == [
            ("opened", HYPERLIQUID_YES_INSTRUMENT_ID),
            ("opened", HYPERLIQUID_NO_INSTRUMENT_ID),
            ("closed", HYPERLIQUID_YES_INSTRUMENT_ID),
            ("closed", HYPERLIQUID_NO_INSTRUMENT_ID),
        ]
    finally:
        engine.dispose()
