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

//! Shared models for simulated binary-option execution actions.

use std::{fmt::Debug, rc::Rc};

use indexmap::IndexMap;
use nautilus_common::{
    cache::Cache,
    messages::execution::{BinaryOptionAction, BinaryOptionPair, ExecuteAction},
};
use nautilus_core::{UUID4, UnixNanos};
use nautilus_model::{
    accounts::{Account, AccountAny},
    enums::{LiquiditySide, OrderSide, OrderType, PositionSide},
    events::OrderFilled,
    identifiers::{ClientOrderId, InstrumentId, TradeId, VenueOrderId},
    instruments::{Instrument, InstrumentAny},
    types::{Currency, Money, Price, Quantity},
};
use rust_decimal::Decimal;
use ustr::Ustr;

/// Validates a binary-option action against a venue's conversion rules.
pub trait BinaryOptionActionValidator: Debug {
    /// Validates `action` against current venue state.
    ///
    /// # Errors
    ///
    /// Returns an error if `action` is invalid or unsupported by the venue.
    fn validate(&self, action: &BinaryOptionAction, cache: &Cache) -> anyhow::Result<()>;
}

/// Shared runtime handle for a binary-option action validator.
#[derive(Clone)]
pub struct BinaryOptionActionValidatorHandle(Rc<dyn BinaryOptionActionValidator>);

impl BinaryOptionActionValidatorHandle {
    /// Creates a new [`BinaryOptionActionValidatorHandle`].
    #[must_use]
    pub fn new<T>(resolver: T) -> Self
    where
        T: BinaryOptionActionValidator + 'static,
    {
        Self(Rc::new(resolver))
    }
}

impl Debug for BinaryOptionActionValidatorHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple(stringify!(BinaryOptionActionValidatorHandle))
            .field(&"<dyn BinaryOptionActionValidator>")
            .finish()
    }
}

impl BinaryOptionActionValidator for BinaryOptionActionValidatorHandle {
    fn validate(&self, action: &BinaryOptionAction, cache: &Cache) -> anyhow::Result<()> {
        self.0.validate(action, cache)
    }
}

/// Immutable fee configuration for binary-option execution actions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BinaryOptionActionConfig {
    split_fee_rate: Decimal,
    merge_fee_rate: Decimal,
}

impl BinaryOptionActionConfig {
    /// Creates a new [`BinaryOptionActionConfig`].
    ///
    /// # Errors
    ///
    /// Returns an error if a fee rate is outside the inclusive `[0, 1]` range.
    pub fn new(split_fee_rate: Decimal, merge_fee_rate: Decimal) -> anyhow::Result<Self> {
        anyhow::ensure!(
            (Decimal::ZERO..=Decimal::ONE).contains(&split_fee_rate),
            "Split fee rate must be within [0, 1], was {split_fee_rate}"
        );
        anyhow::ensure!(
            (Decimal::ZERO..=Decimal::ONE).contains(&merge_fee_rate),
            "Merge fee rate must be within [0, 1], was {merge_fee_rate}"
        );

        Ok(Self {
            split_fee_rate,
            merge_fee_rate,
        })
    }

    /// Returns the configured split fee rate.
    #[must_use]
    pub const fn split_fee_rate(&self) -> Decimal {
        self.split_fee_rate
    }

    /// Returns the configured merge fee rate.
    #[must_use]
    pub const fn merge_fee_rate(&self) -> Decimal {
        self.merge_fee_rate
    }
}

/// A configured binary-option action profile.
#[derive(Clone, Debug)]
pub struct BinaryOptionActionProfile {
    config: BinaryOptionActionConfig,
    validator: BinaryOptionActionValidatorHandle,
}

impl BinaryOptionActionProfile {
    /// Creates a new [`BinaryOptionActionProfile`].
    #[must_use]
    pub const fn new(
        config: BinaryOptionActionConfig,
        validator: BinaryOptionActionValidatorHandle,
    ) -> Self {
        Self { config, validator }
    }

    /// Returns the immutable action configuration.
    #[must_use]
    pub const fn config(&self) -> &BinaryOptionActionConfig {
        &self.config
    }

    /// Validates an action through the configured venue action validator.
    ///
    /// # Errors
    ///
    /// Returns an error if `action` is unsupported or invalid for the venue.
    pub fn validate(&self, action: &BinaryOptionAction, cache: &Cache) -> anyhow::Result<()> {
        self.validator.validate(action, cache)
    }
}

/// Plans a binary-option action as two orderless fill events.
///
/// # Errors
///
/// Returns an error if the action cannot be resolved or represented against the current cache
/// and account state. The function never mutates the cache or account.
pub fn plan_binary_option_action(
    profile: &BinaryOptionActionProfile,
    command: &ExecuteAction,
    cache: &Cache,
    ts_event: UnixNanos,
) -> anyhow::Result<[OrderFilled; 2]> {
    command.action.validate()?;
    let action = command.action.binary_option().ok_or_else(|| {
        anyhow::anyhow!(
            "Execution action '{}' is not a binary-option action",
            command.action.name()
        )
    })?;
    profile.validate(action, cache)?;
    let (yes, no) = current_binary_option_pair(cache, action.pair())?;
    let quote_currency = yes.quote_currency();

    anyhow::ensure!(
        no.quote_currency() == quote_currency,
        "Binary option pair quote currencies differ"
    );
    anyhow::ensure!(
        yes.id().venue == no.id().venue,
        "Binary option pair venues differ"
    );
    anyhow::ensure!(
        !cache.has_instrument_close(&yes.id()) && !cache.has_instrument_close(&no.id()),
        "Binary option pair is closed"
    );

    let yes_expiration = yes
        .expiration_ns()
        .filter(|expiration| *expiration > UnixNanos::default())
        .ok_or_else(|| anyhow::anyhow!("Binary option Yes leg has no expiry"))?;
    let no_expiration = no
        .expiration_ns()
        .filter(|expiration| *expiration > UnixNanos::default())
        .ok_or_else(|| anyhow::anyhow!("Binary option No leg has no expiry"))?;
    anyhow::ensure!(
        yes_expiration == no_expiration,
        "Binary option pair expirations differ"
    );
    anyhow::ensure!(
        ts_event < yes_expiration,
        "Binary option pair expired at {yes_expiration}"
    );

    let price = normalized_action_price(yes, no)?;
    let amount = action.quantity().as_decimal();
    let quantity = exact_action_quantity(yes, no, amount)?;
    let (yes_commission, no_commission) = action_commissions(
        amount,
        quote_currency,
        match action {
            BinaryOptionAction::Split { .. } => profile.config.split_fee_rate(),
            BinaryOptionAction::Merge { .. } => profile.config.merge_fee_rate(),
        },
    )?;

    let account = cache.try_account(&command.account_id)?;
    anyhow::ensure!(
        matches!(&*account, AccountAny::Cash(_)),
        "Binary option actions require a cash account"
    );

    match action {
        BinaryOptionAction::Split { .. } => {
            let fees = yes_commission
                .as_decimal()
                .checked_add(no_commission.as_decimal())
                .ok_or_else(|| anyhow::anyhow!("Binary option action fee overflow"))?;
            let required_amount = amount
                .checked_add(fees)
                .ok_or_else(|| anyhow::anyhow!("Binary option action cash amount overflow"))?;
            let required = Money::from_decimal(required_amount, quote_currency)?;
            anyhow::ensure!(
                required.as_decimal() == required_amount,
                "Split cash amount cannot be represented exactly"
            );
            let free = account
                .balance_free(Some(quote_currency))
                .ok_or_else(|| anyhow::anyhow!("No free {quote_currency} balance is available"))?;
            anyhow::ensure!(
                free >= required,
                "Insufficient free {quote_currency} balance for split"
            );
        }
        BinaryOptionAction::Merge { .. } => {
            ensure_merge_inventory(cache, command, yes, amount)?;
            ensure_merge_inventory(cache, command, no, amount)?;
        }
    }

    let side = match action {
        BinaryOptionAction::Split { .. } => OrderSide::Buy,
        BinaryOptionAction::Merge { .. } => OrderSide::Sell,
    };
    let [yes_ids, no_ids] = action_leg_identifiers(command)?;
    Ok([
        action_leg_fill(
            command,
            yes.id(),
            yes_ids,
            side,
            quantity,
            price,
            quote_currency,
            yes_commission,
            "yes",
            ts_event,
        ),
        action_leg_fill(
            command,
            no.id(),
            no_ids,
            side,
            quantity,
            price,
            quote_currency,
            no_commission,
            "no",
            ts_event,
        ),
    ])
}

fn current_binary_option_pair(
    cache: &Cache,
    pair: BinaryOptionPair,
) -> anyhow::Result<(&InstrumentAny, &InstrumentAny)> {
    let yes = cache.try_instrument(&pair.yes_instrument_id)?;
    let no = cache.try_instrument(&pair.no_instrument_id)?;
    anyhow::ensure!(
        matches!(yes, InstrumentAny::BinaryOption(_)),
        "Binary option Yes leg is not a BinaryOption"
    );
    anyhow::ensure!(
        matches!(no, InstrumentAny::BinaryOption(_)),
        "Binary option No leg is not a BinaryOption"
    );
    Ok((yes, no))
}

fn normalized_action_price(yes: &InstrumentAny, no: &InstrumentAny) -> anyhow::Result<Price> {
    let price = Price::from_decimal(Decimal::new(5, 1))?;
    let yes_price = yes.try_normalize_price(price)?;
    let no_price = no.try_normalize_price(price)?;
    anyhow::ensure!(
        yes_price.as_decimal() == Decimal::new(5, 1) && no_price.as_decimal() == Decimal::new(5, 1),
        "Binary option pair cannot represent an action price of 0.5"
    );
    Ok(yes_price)
}

fn exact_action_quantity(
    yes: &InstrumentAny,
    no: &InstrumentAny,
    amount: Decimal,
) -> anyhow::Result<Quantity> {
    let quantity = Quantity::from_decimal(amount)?;
    let yes_quantity = yes.try_normalize_qty(quantity)?;
    let no_quantity = no.try_normalize_qty(quantity)?;
    anyhow::ensure!(
        yes_quantity.as_decimal() == amount && no_quantity.as_decimal() == amount,
        "Binary option pair cannot represent action amount {amount} exactly"
    );
    Ok(yes_quantity)
}

fn action_commissions(
    amount: Decimal,
    currency: Currency,
    fee_rate: Decimal,
) -> anyhow::Result<(Money, Money)> {
    let total_fee = amount
        .checked_mul(fee_rate)
        .ok_or_else(|| anyhow::anyhow!("Binary option action fee overflow"))?;
    let total = Money::from_decimal(total_fee, currency)?;
    let no_raw = total.raw() / 2;
    let yes_raw = total.raw() - no_raw;
    Ok((
        Money::from_raw_checked(yes_raw, currency)?,
        Money::from_raw_checked(no_raw, currency)?,
    ))
}

fn ensure_merge_inventory(
    cache: &Cache,
    command: &ExecuteAction,
    instrument: &InstrumentAny,
    amount: Decimal,
) -> anyhow::Result<()> {
    let available = open_long_quantity(cache, command, instrument)?;
    anyhow::ensure!(
        available >= amount,
        "Insufficient {} inventory for merge",
        instrument.id()
    );
    Ok(())
}

fn open_long_quantity(
    cache: &Cache,
    command: &ExecuteAction,
    instrument: &InstrumentAny,
) -> anyhow::Result<Decimal> {
    cache
        .positions_open_refs(
            Some(&instrument.id().venue),
            Some(&instrument.id()),
            Some(&command.strategy_id),
            Some(&command.account_id),
            Some(PositionSide::Long),
        )
        .into_iter()
        .try_fold(Decimal::ZERO, |total, position| {
            total
                .checked_add(position.quantity.as_decimal())
                .ok_or_else(|| anyhow::anyhow!("Binary option position quantity overflow"))
        })
}

fn action_leg_identifiers(
    command: &ExecuteAction,
) -> anyhow::Result<[(ClientOrderId, VenueOrderId, TradeId); 2]> {
    let compact_command_id = command.command_id.as_str().replace('-', "");
    let create = |leg: &str, trade_suffix: &str| -> anyhow::Result<_> {
        Ok((
            ClientOrderId::new_checked(format!("ACTION-{compact_command_id}-LEG-{leg}"))?,
            VenueOrderId::new_checked(format!("ACTION-{compact_command_id}-LEG-{leg}"))?,
            TradeId::new_checked(format!("{compact_command_id}{trade_suffix}"))?,
        ))
    };
    Ok([create("YES", "Y")?, create("NO", "N")?])
}

#[expect(clippy::too_many_arguments)]
fn action_leg_fill(
    command: &ExecuteAction,
    instrument_id: InstrumentId,
    identifiers: (ClientOrderId, VenueOrderId, TradeId),
    order_side: OrderSide,
    quantity: Quantity,
    price: Price,
    currency: Currency,
    commission: Money,
    leg: &str,
    ts_event: UnixNanos,
) -> OrderFilled {
    let (client_order_id, venue_order_id, trade_id) = identifiers;
    let mut info = IndexMap::new();
    info.insert(Ustr::from("source"), Ustr::from("execution_action"));
    info.insert(Ustr::from("action"), Ustr::from(command.action.name()));
    info.insert(
        Ustr::from("command_id"),
        Ustr::from(command.command_id.as_str()),
    );
    info.insert(Ustr::from("leg"), Ustr::from(leg));
    OrderFilled::new(
        command.trader_id,
        command.strategy_id,
        instrument_id,
        client_order_id,
        venue_order_id,
        command.account_id,
        trade_id,
        order_side,
        OrderType::Market,
        quantity,
        price,
        currency,
        LiquiditySide::NoLiquiditySide,
        UUID4::new(),
        ts_event,
        ts_event,
        false,
        None,
        Some(commission),
        Some(info),
    )
}

#[cfg(test)]
mod tests {
    use nautilus_common::messages::execution::{
        BinaryOptionAction, BinaryOptionPair, ExecuteAction, ExecutionAction,
    };
    use nautilus_core::{UUID4, UnixNanos};
    use nautilus_model::{
        accounts::{AccountAny, CashAccount},
        data::InstrumentClose,
        enums::{AccountType, InstrumentCloseType, OmsType, OrderSide},
        events::AccountState,
        identifiers::{AccountId, InstrumentId, PositionId, StrategyId, TraderId},
        instruments::{InstrumentAny, stubs::binary_option},
        position::Position,
        types::{AccountBalance, Currency, Price},
    };
    use rstest::rstest;
    use rust_decimal_macros::dec;

    use super::*;

    #[derive(Debug)]
    struct TestValidator;

    impl BinaryOptionActionValidator for TestValidator {
        fn validate(&self, action: &BinaryOptionAction, _cache: &Cache) -> anyhow::Result<()> {
            anyhow::ensure!(
                action.pair()
                    == BinaryOptionPair::new(
                        InstrumentId::from("42-YES-OUTCOME.TEST"),
                        InstrumentId::from("42-NO-OUTCOME.TEST"),
                    )?,
                "Unsupported test binary-option pair"
            );
            Ok(())
        }
    }

    fn profile() -> BinaryOptionActionProfile {
        let config = BinaryOptionActionConfig::new(Decimal::ZERO, dec!(0.0007)).unwrap();
        BinaryOptionActionProfile::new(
            config,
            BinaryOptionActionValidatorHandle::new(TestValidator),
        )
    }

    fn command(action: BinaryOptionAction) -> ExecuteAction {
        ExecuteAction::new(
            TraderId::from("TRADER-001"),
            None,
            StrategyId::from("STRATEGY-001"),
            AccountId::from("TEST-001"),
            ExecutionAction::BinaryOption(action),
            UUID4::new(),
            UnixNanos::default(),
            None,
        )
    }

    fn pair() -> BinaryOptionPair {
        BinaryOptionPair::new(
            InstrumentId::from("42-YES-OUTCOME.TEST"),
            InstrumentId::from("42-NO-OUTCOME.TEST"),
        )
        .unwrap()
    }

    fn split_command() -> ExecuteAction {
        command(BinaryOptionAction::Split {
            pair: pair(),
            quantity: Quantity::from("1.25"),
            params: None,
        })
    }

    fn merge_command() -> ExecuteAction {
        command(BinaryOptionAction::Merge {
            pair: pair(),
            quantity: Quantity::from("1.25"),
            params: None,
        })
    }

    fn cache_with_binary_option_pair(free: &str, expiration: UnixNanos) -> Cache {
        let mut cache = Cache::default();
        let mut yes = binary_option();
        yes.id = InstrumentId::from("42-YES-OUTCOME.TEST");
        yes.expiration_ns = expiration;
        let mut no = yes.clone();
        no.id = InstrumentId::from("42-NO-OUTCOME.TEST");

        cache
            .add_instrument(InstrumentAny::BinaryOption(yes))
            .unwrap();
        cache
            .add_instrument(InstrumentAny::BinaryOption(no))
            .unwrap();

        let balance = Money::from(free);
        let state = AccountState::new(
            AccountId::from("TEST-001"),
            AccountType::Cash,
            vec![AccountBalance::new(
                balance,
                Money::zero(balance.currency),
                balance,
            )],
            Vec::new(),
            true,
            UUID4::new(),
            UnixNanos::default(),
            UnixNanos::default(),
            Some(Currency::USDC()),
        );
        cache
            .add_account(AccountAny::Cash(CashAccount::new(state, true, false)))
            .unwrap();
        cache
    }

    fn cache_with_mergeable_binary_option_pair() -> Cache {
        let mut cache = cache_with_binary_option_pair("100 USDC", UnixNanos::from(2));
        let fills =
            plan_binary_option_action(&profile(), &split_command(), &cache, UnixNanos::from(1))
                .unwrap();

        for mut fill in fills {
            let instrument = cache.instrument(&fill.instrument_id).unwrap().clone();
            fill.position_id = Some(PositionId::new(format!(
                "{}-{}",
                fill.instrument_id, fill.strategy_id
            )));
            let position = Position::new(&instrument, fill);
            cache
                .add_position_without_order(&position, OmsType::Netting)
                .unwrap();
        }
        cache
    }

    #[rstest]
    fn test_config_rejects_fee_rate_above_one() {
        let err = BinaryOptionActionConfig::new(dec!(1.0001), Decimal::ZERO).unwrap_err();

        assert_eq!(
            err.to_string(),
            "Split fee rate must be within [0, 1], was 1.0001"
        );
    }

    #[rstest]
    fn test_profile_validates_binary_option_pair() {
        let action = BinaryOptionAction::Split {
            pair: pair(),
            quantity: Quantity::from("1.25"),
            params: None,
        };

        profile().validate(&action, &Cache::default()).unwrap();
    }

    #[rstest]
    fn test_profile_propagates_venue_rejection() {
        let action = BinaryOptionAction::Split {
            pair: BinaryOptionPair::new(
                InstrumentId::from("42-YES-OUTCOME.OTHER"),
                InstrumentId::from("42-NO-OUTCOME.OTHER"),
            )
            .unwrap(),
            quantity: Quantity::from("1.25"),
            params: None,
        };
        let err = profile().validate(&action, &Cache::default()).unwrap_err();

        assert_eq!(err.to_string(), "Unsupported test binary-option pair");
    }

    #[rstest]
    fn test_plan_split_creates_orderless_yes_and_no_fills() {
        let cache = cache_with_binary_option_pair("100 USDC", UnixNanos::from(2));
        let fills =
            plan_binary_option_action(&profile(), &split_command(), &cache, UnixNanos::from(1))
                .unwrap();

        assert_eq!(
            fills[0].instrument_id,
            InstrumentId::from("42-YES-OUTCOME.TEST")
        );
        assert_eq!(
            fills[1].instrument_id,
            InstrumentId::from("42-NO-OUTCOME.TEST")
        );
        assert_eq!(fills[0].order_side, OrderSide::Buy);
        assert_eq!(fills[1].order_side, OrderSide::Buy);
        assert_eq!(fills[0].last_qty.as_decimal(), dec!(1.25));
        assert_eq!(fills[0].last_px.as_decimal(), dec!(0.5));
        assert!(fills[0].client_order_id.as_str().contains("-LEG-YES"));
        assert!(fills[1].client_order_id.as_str().contains("-LEG-NO"));
    }

    #[rstest]
    fn test_plan_split_rejects_insufficient_free_quote_balance() {
        let cache = cache_with_binary_option_pair("1 USDC", UnixNanos::from(2));
        let err =
            plan_binary_option_action(&profile(), &split_command(), &cache, UnixNanos::from(1))
                .unwrap_err();

        assert_eq!(err.to_string(), "Insufficient free USDC balance for split");
    }

    #[rstest]
    fn test_plan_split_rejects_expired_pair() {
        let cache = cache_with_binary_option_pair("100 USDC", UnixNanos::from(1));
        let err =
            plan_binary_option_action(&profile(), &split_command(), &cache, UnixNanos::from(1))
                .unwrap_err();

        assert_eq!(err.to_string(), "Binary option pair expired at 1");
    }

    #[rstest]
    fn test_plan_rejects_missing_binary_option_leg() {
        let mut cache = Cache::default();
        let mut yes = binary_option();
        yes.id = InstrumentId::from("42-YES-OUTCOME.TEST");
        cache
            .add_instrument(InstrumentAny::BinaryOption(yes))
            .unwrap();

        let err =
            plan_binary_option_action(&profile(), &split_command(), &cache, UnixNanos::from(1))
                .unwrap_err();

        assert_eq!(
            err.to_string(),
            "instrument not found in cache: 42-NO-OUTCOME.TEST"
        );
    }

    #[rstest]
    fn test_plan_rejects_closed_pair() {
        let mut cache = cache_with_binary_option_pair("100 USDC", UnixNanos::from(2));
        cache
            .add_instrument_close(InstrumentClose::new(
                InstrumentId::from("42-YES-OUTCOME.TEST"),
                Price::from("0.50"),
                InstrumentCloseType::EndOfSession,
                UnixNanos::from(1),
                UnixNanos::from(1),
            ))
            .unwrap();

        let err =
            plan_binary_option_action(&profile(), &split_command(), &cache, UnixNanos::from(1))
                .unwrap_err();

        assert_eq!(err.to_string(), "Binary option pair is closed");
    }

    #[rstest]
    fn test_plan_merge_creates_orderless_yes_and_no_sell_fills() {
        let cache = cache_with_mergeable_binary_option_pair();
        let fills =
            plan_binary_option_action(&profile(), &merge_command(), &cache, UnixNanos::from(1))
                .unwrap();

        assert_eq!(fills[0].order_side, OrderSide::Sell);
        assert_eq!(fills[1].order_side, OrderSide::Sell);
        assert_eq!(fills[0].last_qty.as_decimal(), dec!(1.25));
        assert_eq!(
            fills[0].commission.unwrap().as_decimal() + fills[1].commission.unwrap().as_decimal(),
            dec!(0.000875)
        );
    }
}
