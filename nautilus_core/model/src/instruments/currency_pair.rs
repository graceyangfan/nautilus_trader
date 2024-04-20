// -------------------------------------------------------------------------------------------------
//  Copyright (C) 2015-2024 Nautech Systems Pty Ltd. All rights reserved.
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

use std::{
    hash::{Hash, Hasher},
    str::FromStr,
};

use nautilus_core::{
    correctness::{check_equal_u8, check_positive_i64, check_positive_u64},
    nanos::UnixNanos,
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgRow, FromRow, Row};
use ustr::Ustr;

use super::{Instrument, InstrumentAny};
use crate::{
    enums::{AssetClass, InstrumentClass, OptionKind},
    identifiers::{instrument_id::InstrumentId, symbol::Symbol},
    types::{currency::Currency, money::Money, price::Price, quantity::Quantity},
};

#[repr(C)]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "nautilus_trader.core.nautilus_pyo3.model")
)]
#[cfg_attr(feature = "trivial_copy", derive(Copy))]
pub struct CurrencyPair {
    pub id: InstrumentId,
    pub raw_symbol: Symbol,
    pub base_currency: Currency,
    pub quote_currency: Currency,
    pub price_precision: u8,
    pub size_precision: u8,
    pub price_increment: Price,
    pub size_increment: Quantity,
    pub maker_fee: Decimal,
    pub taker_fee: Decimal,
    pub margin_init: Decimal,
    pub margin_maint: Decimal,
    pub lot_size: Option<Quantity>,
    pub max_quantity: Option<Quantity>,
    pub min_quantity: Option<Quantity>,
    pub max_notional: Option<Money>,
    pub min_notional: Option<Money>,
    pub max_price: Option<Price>,
    pub min_price: Option<Price>,
    pub ts_event: UnixNanos,
    pub ts_init: UnixNanos,
}

impl CurrencyPair {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: InstrumentId,
        raw_symbol: Symbol,
        base_currency: Currency,
        quote_currency: Currency,
        price_precision: u8,
        size_precision: u8,
        price_increment: Price,
        size_increment: Quantity,
        taker_fee: Decimal,
        maker_fee: Decimal,
        margin_init: Decimal,
        margin_maint: Decimal,
        lot_size: Option<Quantity>,
        max_quantity: Option<Quantity>,
        min_quantity: Option<Quantity>,
        max_notional: Option<Money>,
        min_notional: Option<Money>,
        max_price: Option<Price>,
        min_price: Option<Price>,
        ts_event: UnixNanos,
        ts_init: UnixNanos,
    ) -> anyhow::Result<Self> {
        check_equal_u8(
            price_precision,
            price_increment.precision,
            stringify!(price_precision),
            stringify!(price_increment.precision),
        )?;
        check_equal_u8(
            size_precision,
            size_increment.precision,
            stringify!(size_precision),
            stringify!(size_increment.precision),
        )?;
        check_positive_i64(price_increment.raw, stringify!(price_increment.raw))?;
        check_positive_u64(size_increment.raw, stringify!(size_increment.raw))?;

        Ok(Self {
            id,
            raw_symbol,
            base_currency,
            quote_currency,
            price_precision,
            size_precision,
            price_increment,
            size_increment,
            maker_fee,
            taker_fee,
            margin_init,
            margin_maint,
            lot_size,
            max_quantity,
            min_quantity,
            max_notional,
            min_notional,
            max_price,
            min_price,
            ts_event,
            ts_init,
        })
    }
}

impl PartialEq<Self> for CurrencyPair {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for CurrencyPair {}

impl Hash for CurrencyPair {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl Instrument for CurrencyPair {
    fn into_any(self) -> InstrumentAny {
        InstrumentAny::CurrencyPair(self)
    }

    fn id(&self) -> InstrumentId {
        self.id
    }

    fn raw_symbol(&self) -> Symbol {
        self.raw_symbol
    }

    fn asset_class(&self) -> AssetClass {
        AssetClass::FX
    }

    fn instrument_class(&self) -> InstrumentClass {
        InstrumentClass::Spot
    }
    fn underlying(&self) -> Option<Ustr> {
        None
    }

    fn quote_currency(&self) -> Currency {
        self.quote_currency
    }

    fn base_currency(&self) -> Option<Currency> {
        Some(self.base_currency)
    }

    fn settlement_currency(&self) -> Currency {
        self.quote_currency
    }
    fn isin(&self) -> Option<Ustr> {
        None
    }

    fn is_inverse(&self) -> bool {
        false
    }

    fn price_precision(&self) -> u8 {
        self.price_precision
    }

    fn size_precision(&self) -> u8 {
        self.size_precision
    }

    fn price_increment(&self) -> Price {
        self.price_increment
    }

    fn size_increment(&self) -> Quantity {
        self.size_increment
    }

    fn multiplier(&self) -> Quantity {
        // SAFETY: Unwrap safe as using known values
        Quantity::new(1.0, 0).unwrap()
    }

    fn lot_size(&self) -> Option<Quantity> {
        self.lot_size
    }

    fn max_quantity(&self) -> Option<Quantity> {
        self.max_quantity
    }

    fn min_quantity(&self) -> Option<Quantity> {
        self.min_quantity
    }

    fn max_price(&self) -> Option<Price> {
        self.max_price
    }

    fn min_price(&self) -> Option<Price> {
        self.min_price
    }

    fn ts_event(&self) -> UnixNanos {
        self.ts_event
    }

    fn ts_init(&self) -> UnixNanos {
        self.ts_init
    }

    fn margin_init(&self) -> Decimal {
        self.margin_init
    }

    fn margin_maint(&self) -> Decimal {
        self.margin_maint
    }

    fn taker_fee(&self) -> Decimal {
        self.taker_fee
    }

    fn maker_fee(&self) -> Decimal {
        self.maker_fee
    }

    fn option_kind(&self) -> Option<OptionKind> {
        None
    }

    fn exchange(&self) -> Option<Ustr> {
        None
    }

    fn strike_price(&self) -> Option<Price> {
        None
    }

    fn activation_ns(&self) -> Option<UnixNanos> {
        None
    }

    fn expiration_ns(&self) -> Option<UnixNanos> {
        None
    }

    fn max_notional(&self) -> Option<Money> {
        self.max_notional
    }

    fn min_notional(&self) -> Option<Money> {
        self.min_notional
    }
}

impl<'r> FromRow<'r, PgRow> for CurrencyPair {
    fn from_row(row: &'r PgRow) -> Result<Self, sqlx::Error> {
        let id = row
            .try_get::<String, _>("id")
            .map(|res| InstrumentId::from(res.as_str()))?;
        let raw_symbol = row
            .try_get::<String, _>("raw_symbol")
            .map(|res| Symbol::from(res.as_str()))?;
        let base_currency = row
            .try_get::<String, _>("base_currency")
            .map(|res| Currency::from(res.as_str()))?;
        let quote_currency = row
            .try_get::<String, _>("quote_currency")
            .map(|res| Currency::from(res.as_str()))?;
        let price_precision = row.try_get::<i32, _>("price_precision")?;
        let size_precision = row.try_get::<i32, _>("size_precision")?;
        let price_increment = row
            .try_get::<String, _>("price_increment")
            .map(|res| Price::from(res.as_str()))?;
        let size_increment = row
            .try_get::<String, _>("size_increment")
            .map(|res| Quantity::from(res.as_str()))?;
        let maker_fee = row
            .try_get::<String, _>("maker_fee")
            .map(|res| Decimal::from_str(res.as_str()).unwrap())?;
        let taker_fee = row
            .try_get::<String, _>("taker_fee")
            .map(|res| Decimal::from_str(res.as_str()).unwrap())?;
        let margin_init = row
            .try_get::<String, _>("margin_init")
            .map(|res| Decimal::from_str(res.as_str()).unwrap())?;
        let margin_maint = row
            .try_get::<String, _>("margin_maint")
            .map(|res| Decimal::from_str(res.as_str()).unwrap())?;
        let lot_size = row
            .try_get::<Option<String>, _>("lot_size")
            .ok()
            .and_then(|res| res.map(|res| Quantity::from(res.as_str())));
        let max_quantity = row
            .try_get::<Option<String>, _>("max_quantity")
            .ok()
            .and_then(|res| res.map(|res| Quantity::from(res.as_str())));
        let min_quantity = row
            .try_get::<Option<String>, _>("min_quantity")
            .ok()
            .and_then(|res| res.map(|res| Quantity::from(res.as_str())));
        let max_notional = row
            .try_get::<Option<String>, _>("max_notional")
            .ok()
            .and_then(|res| res.map(|res| Money::from(res.as_str())));
        let min_notional = row
            .try_get::<Option<String>, _>("min_notional")
            .ok()
            .and_then(|res| res.map(|res| Money::from(res.as_str())));
        let max_price = row
            .try_get::<Option<String>, _>("max_price")
            .ok()
            .and_then(|res| res.map(|res| Price::from(res.as_str())));
        let min_price = row
            .try_get::<Option<String>, _>("min_price")
            .ok()
            .and_then(|res| res.map(|res| Price::from(res.as_str())));
        let ts_event = row
            .try_get::<String, _>("ts_event")
            .map(|res| UnixNanos::from(res.as_str()))?;
        let ts_init = row
            .try_get::<String, _>("ts_init")
            .map(|res| UnixNanos::from(res.as_str()))?;
        Ok(Self::new(
            id,
            raw_symbol,
            base_currency,
            quote_currency,
            price_precision as u8,
            size_precision as u8,
            price_increment,
            size_increment,
            taker_fee,
            maker_fee,
            margin_init,
            margin_maint,
            lot_size,
            max_quantity,
            min_quantity,
            max_notional,
            min_notional,
            max_price,
            min_price,
            ts_event,
            ts_init,
        )
        .unwrap())
    }
}

////////////////////////////////////////////////////////////////////////////////
// Tests
///////////////////////////////////////////////////////////////////////////////
#[cfg(test)]
mod tests {
    use rstest::rstest;

    use crate::instruments::{currency_pair::CurrencyPair, stubs::*};

    #[rstest]
    fn test_equality(currency_pair_btcusdt: CurrencyPair) {
        let cloned = currency_pair_btcusdt;
        assert_eq!(currency_pair_btcusdt, cloned);
    }
}
