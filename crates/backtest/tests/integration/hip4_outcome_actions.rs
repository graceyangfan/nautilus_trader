// -------------------------------------------------------------------------------------------------
//  Copyright (C) 2015-2026 Nautech Systems Pty Ltd. All rights reserved.
//
//  Licensed under the GNU Lesser General Public License Version 3.0 (the "License");
//  you may not use this file except in compliance with the License.
//  You may obtain a copy of the License at https://www.gnu.org/licenses/lgpl-3.0.en.html
//
//  Unless required by applicable law or agreed to in writing, software
//  distributed under the License is distributed on an "AS IS" BASIS,
//  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//  See the License for the specific language governing permissions and
//  limitations under the License.
// -------------------------------------------------------------------------------------------------

//! HIP-4 strategy-level execution-action coverage.

use std::{cell::RefCell, fmt::Debug, rc::Rc};

use nautilus_backtest::{
    config::{BacktestEngineConfig, SimulatedVenueConfig},
    engine::BacktestEngine,
};
use nautilus_common::{
    actor::DataActor,
    messages::execution::{
        BinaryOptionAction, BinaryOptionPair, ExecutionActionResult, ExecutionActionStatus,
    },
};
use nautilus_core::UnixNanos;
use nautilus_execution::models::fee::{FeeModelAny, MakerTakerFeeModel};
use nautilus_hyperliquid::{
    common::outcome_pair_instrument_ids, hyperliquid_outcome_action_profile,
};
use nautilus_model::{
    accounts::Account,
    data::{Data, QuoteTick},
    enums::{AccountType, BookType, OmsType},
    events::{PositionChanged, PositionClosed, PositionOpened},
    identifiers::{AccountId, ClientId, InstrumentId, StrategyId, Venue},
    instruments::{InstrumentAny, stubs::binary_option},
    types::{Currency, Money, Price, Quantity},
};
use nautilus_trading::{Strategy, StrategyConfig, StrategyCore, nautilus_strategy};
use rstest::rstest;
use rust_decimal::Decimal;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActionPhase {
    AwaitingInitialSplit,
    AwaitingPartialMerge,
    AwaitingFinalMerge,
    AwaitingFinalSplit,
    Complete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StrategyCallback {
    PositionOpened(InstrumentId),
    PositionChanged(InstrumentId),
    PositionClosed(InstrumentId),
    ActionAccepted,
}

struct Hip4SplitStrategy {
    core: StrategyCore,
    account_id: AccountId,
    pair: BinaryOptionPair,
    phase: Option<ActionPhase>,
    results: Rc<RefCell<Vec<ExecutionActionResult>>>,
    callbacks: Rc<RefCell<Vec<StrategyCallback>>>,
}

impl Hip4SplitStrategy {
    fn new(
        account_id: AccountId,
        pair: BinaryOptionPair,
        results: Rc<RefCell<Vec<ExecutionActionResult>>>,
        callbacks: Rc<RefCell<Vec<StrategyCallback>>>,
    ) -> Self {
        Self {
            core: StrategyCore::new(StrategyConfig {
                strategy_id: Some(StrategyId::from("HIP4-SPLIT-001")),
                oms_type: Some(OmsType::Netting),
                ..Default::default()
            }),
            account_id,
            pair,
            phase: None,
            results,
            callbacks,
        }
    }
}

nautilus_strategy!(Hip4SplitStrategy, {
    fn on_execution_action_result(&mut self, result: ExecutionActionResult) {
        self.results.borrow_mut().push(result.clone());
        if result.status != ExecutionActionStatus::Accepted {
            self.phase = Some(ActionPhase::Complete);
            return;
        }

        self.callbacks
            .borrow_mut()
            .push(StrategyCallback::ActionAccepted);
        let next = match self.phase {
            Some(ActionPhase::AwaitingInitialSplit) => (
                ActionPhase::AwaitingPartialMerge,
                BinaryOptionAction::Merge {
                    pair: self.pair,
                    quantity: Quantity::from("0.5"),
                    params: None,
                },
            ),
            Some(ActionPhase::AwaitingPartialMerge) => (
                ActionPhase::AwaitingFinalMerge,
                BinaryOptionAction::Merge {
                    pair: self.pair,
                    quantity: Quantity::from("0.75"),
                    params: None,
                },
            ),
            Some(ActionPhase::AwaitingFinalMerge) => (
                ActionPhase::AwaitingFinalSplit,
                BinaryOptionAction::Split {
                    pair: self.pair,
                    quantity: Quantity::from("0.5"),
                    params: None,
                },
            ),
            Some(ActionPhase::AwaitingFinalSplit) => {
                self.phase = Some(ActionPhase::Complete);
                return;
            }
            Some(ActionPhase::Complete) | None => return,
        };
        self.phase = Some(next.0);
        self.execute_action(self.account_id, Some(ClientId::from("HYPERLIQUID")), next.1)
            .expect("test HIP-4 action must be routable");
    }

    fn on_position_opened(&mut self, event: PositionOpened) {
        self.callbacks
            .borrow_mut()
            .push(StrategyCallback::PositionOpened(event.instrument_id));
    }

    fn on_position_changed(&mut self, event: PositionChanged) {
        self.callbacks
            .borrow_mut()
            .push(StrategyCallback::PositionChanged(event.instrument_id));
    }

    fn on_position_closed(&mut self, event: PositionClosed) {
        self.callbacks
            .borrow_mut()
            .push(StrategyCallback::PositionClosed(event.instrument_id));
    }
});

impl Debug for Hip4SplitStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(stringify!(Hip4SplitStrategy)).finish()
    }
}

impl DataActor for Hip4SplitStrategy {
    fn on_start(&mut self) -> anyhow::Result<()> {
        self.subscribe_quotes(self.pair.yes_instrument_id, None, None);
        Ok(())
    }

    fn on_quote(&mut self, _quote: &QuoteTick) -> anyhow::Result<()> {
        if self.phase.is_some() {
            return Ok(());
        }
        self.phase = Some(ActionPhase::AwaitingInitialSplit);
        self.execute_action(
            self.account_id,
            Some(ClientId::from("HYPERLIQUID")),
            BinaryOptionAction::Split {
                pair: self.pair,
                quantity: Quantity::from("1.25"),
                params: None,
            },
        )
        .map(|_| ())
    }
}

#[rstest]
fn test_hip4_split_strategy_runs_through_backtest() {
    let venue = Venue::from("HYPERLIQUID");
    let account_id = AccountId::from("HYPERLIQUID-001");
    let [yes_id, no_id] = outcome_pair_instrument_ids(42).unwrap();
    let mut yes = binary_option();
    yes.id = yes_id;
    yes.expiration_ns = UnixNanos::from(1_000_000_000);
    let mut no = yes.clone();
    no.id = no_id;

    let mut engine = BacktestEngine::new(BacktestEngineConfig::default()).unwrap();
    engine
        .add_venue(
            SimulatedVenueConfig::builder()
                .venue(venue)
                .oms_type(OmsType::Netting)
                .account_type(AccountType::Cash)
                .book_type(BookType::L1_MBP)
                .starting_balances(vec![Money::from("100 USDC")])
                .base_currency(Currency::USDC())
                .fee_model(FeeModelAny::MakerTaker(MakerTakerFeeModel::zero()).into())
                .binary_option_action_profile(
                    hyperliquid_outcome_action_profile(Decimal::ZERO, Decimal::ZERO).unwrap(),
                )
                .build()
                .unwrap(),
        )
        .unwrap();
    engine
        .add_instrument(&InstrumentAny::BinaryOption(yes))
        .unwrap();
    engine
        .add_instrument(&InstrumentAny::BinaryOption(no))
        .unwrap();

    let results = Rc::new(RefCell::new(Vec::new()));
    let callbacks = Rc::new(RefCell::new(Vec::new()));
    engine
        .add_strategy(Hip4SplitStrategy::new(
            account_id,
            BinaryOptionPair::new(yes_id, no_id).unwrap(),
            Rc::clone(&results),
            Rc::clone(&callbacks),
        ))
        .unwrap();
    engine
        .add_data(
            vec![Data::Quote(QuoteTick::new(
                yes_id,
                Price::from("0.400"),
                Price::from("0.410"),
                Quantity::from("100.00"),
                Quantity::from("100.00"),
                UnixNanos::from(1),
                UnixNanos::from(1),
            ))],
            None,
            true,
            true,
        )
        .unwrap();
    engine.run(None, None, None, false).unwrap();

    assert_eq!(results.borrow().len(), 4);
    assert_eq!(
        results
            .borrow()
            .iter()
            .map(|result| result.status)
            .collect::<Vec<_>>(),
        vec![ExecutionActionStatus::Accepted; 4]
    );
    assert_eq!(
        results
            .borrow()
            .iter()
            .map(|result| result.action.as_str())
            .collect::<Vec<_>>(),
        vec![
            "binary_option.split",
            "binary_option.merge",
            "binary_option.merge",
            "binary_option.split",
        ]
    );
    assert_eq!(
        *callbacks.borrow(),
        vec![
            StrategyCallback::PositionOpened(yes_id),
            StrategyCallback::PositionOpened(no_id),
            StrategyCallback::ActionAccepted,
            StrategyCallback::PositionChanged(yes_id),
            StrategyCallback::PositionChanged(no_id),
            StrategyCallback::ActionAccepted,
            StrategyCallback::PositionClosed(yes_id),
            StrategyCallback::PositionClosed(no_id),
            StrategyCallback::ActionAccepted,
            StrategyCallback::PositionOpened(yes_id),
            StrategyCallback::PositionOpened(no_id),
            StrategyCallback::ActionAccepted,
        ]
    );
    let cache = engine.kernel().cache();
    let cache = cache.borrow();
    assert_eq!(
        cache
            .account(&account_id)
            .and_then(|account| account.balance_total(Some(Currency::USDC()))),
        Some(Money::from("99.5 USDC"))
    );
    let positions = cache.positions_open_refs(
        Some(&venue),
        None,
        Some(&StrategyId::from("HIP4-SPLIT-001")),
        Some(&account_id),
        None,
    );
    assert_eq!(positions.len(), 2);
    assert!(
        positions
            .iter()
            .all(|position| position.quantity == Quantity::from("0.5"))
    );
    assert_eq!(cache.orders_total_count(None, None, None, None, None), 0);
}
