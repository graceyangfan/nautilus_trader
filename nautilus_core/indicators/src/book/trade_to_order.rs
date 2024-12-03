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

use std::fmt::Display;

use nautilus_model::{
    orderbook::book::OrderBook, 
    data::trade::TradeTick, 
    types::quantity::Quantity,
    enums::AggressorSide,
};

use crate::indicator::Indicator;

/// An indicator which calculates the ratio of trade volume to order volume in the order book.
/// 
/// The T2O ratio measures the proportion of executed trades to placed orders in the market.
/// It's calculated by dividing the volume of executed trades by the total volume of orders 
/// (including unexecuted ones) in a given time frame.
/// 
/// A high T2O ratio may indicate strong demand or supply at certain price levels, while a 
/// low ratio may suggest indecision or lack of conviction in the market.
#[repr(C)]
#[derive(Debug)]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "nautilus_trader.core.nautilus_pyo3.indicators")
)]
pub struct TradeToOrderRatio {
    pub value: f64,
    pub count: usize,
    pub initialized: bool,
    has_inputs: bool,
    depth: usize,
    trade_volume: f64,
    order_volume: f64,
    initial_order_volume: f64,
}

impl Display for TradeToOrderRatio {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}({})", self.name(), self.depth)
    }
}

impl Indicator for TradeToOrderRatio {
    fn name(&self) -> String {
        stringify!(TradeToOrderRatio).to_string()
    }

    fn has_inputs(&self) -> bool {
        self.has_inputs
    }

    fn initialized(&self) -> bool {
        self.initialized
    }

    fn handle_book(&mut self, book: &OrderBook) {
        // Calculate total volume from order book up to specified depth
        let mut total_volume = 0.0;
        
        // Process bids
        for (i, level) in book.bids().iter().enumerate() {
            if i >= self.depth {
                break;
            }
            total_volume += level.size();
        }

        // Process asks
        for (i, level) in book.asks().iter().enumerate() {
            if i >= self.depth {
                break;
            }
            total_volume += level.size();
        }

        self.order_volume = total_volume;
        if self.initial_order_volume == 0.0 {
            self.initial_order_volume = total_volume;
        }
        self.update();
    }

    fn handle_trade_tick(&mut self, trade: &TradeTick) {
        let volume = trade.size.as_f64();
        match trade.aggressor_side {
            AggressorSide::Buyer => self.trade_volume += volume,
            AggressorSide::Seller => self.trade_volume -= volume,
        }
        self.update();
    }

    fn reset(&mut self) {
        self.value = 0.0;
        self.count = 0;
        self.has_inputs = false;
        self.initialized = false;
        self.trade_volume = 0.0;
        self.order_volume = 0.0;
        self.initial_order_volume = 0.0;
    }
}

impl TradeToOrderRatio {
    /// Creates a new [`TradeToOrderRatio`] instance.
    ///
    /// # Arguments
    ///
    /// * `depth` - The maximum number of price levels to consider from the order book.
    ///            If the actual order book has fewer levels, all available levels will be used.
    #[must_use]
    pub fn new(depth: usize) -> Self {
        Self {
            value: 0.0,
            count: 0,
            has_inputs: false,
            initialized: false,
            depth,
            trade_volume: 0.0,
            order_volume: 0.0,
            initial_order_volume: 0.0,
        }
    }

    /// Resets calculation for a new time window while maintaining the depth setting.
    pub fn reset_calculation(&mut self) {
        self.trade_volume = 0.0;
        self.initial_order_volume = self.order_volume;
        self.value = 0.0;
        self.count = 0;
    }

    fn update(&mut self) {
        self.has_inputs = true;
        self.count += 1;

        let order_volume_delta = self.order_volume - self.initial_order_volume;
        if order_volume_delta != 0.0 {
            self.value = self.trade_volume / order_volume_delta;
            self.initialized = true;
        }
    }
}

////////////////////////////////////////////////////////////////////////////////
// Tests
////////////////////////////////////////////////////////////////////////////////
#[cfg(test)]
mod tests {
    use nautilus_model::{
        identifiers::InstrumentId,
        stubs::{stub_order_book_mbp, stub_trade_tick_eth_usdt},
    };
    use rstest::rstest;

    use super::*;

    #[rstest]
    fn test_initialized() {
        let ratio = TradeToOrderRatio::new(20);
        let display_str = format!("{ratio}");
        assert_eq!(display_str, "TradeToOrderRatio(20)");
        assert_eq!(ratio.value, 0.0);
        assert_eq!(ratio.count, 0);
        assert!(!ratio.has_inputs);
        assert!(!ratio.initialized);
    }

    #[rstest]
    fn test_handle_book() {
        let mut ratio = TradeToOrderRatio::new(2);
        let book = stub_order_book_mbp(
            InstrumentId::from("ETH/USDT.BINANCE"),
            101.0,
            100.0,
            100.0,
            100.0,
            2,
            0.01,
            0,
            100.0,
            10,
        );
        ratio.handle_book(&book);

        assert_eq!(ratio.count, 1);
        assert_eq!(ratio.order_volume, 200.0);
        assert_eq!(ratio.initial_order_volume, 200.0);
        assert_eq!(ratio.value, 0.0);
        assert!(ratio.has_inputs);
    }

    #[rstest]
    fn test_handle_trade_with_buyer_aggressor() {
        let mut ratio = TradeToOrderRatio::new(2);
        let book = stub_order_book_mbp(
            InstrumentId::from("ETH/USDT.BINANCE"),
            101.0,
            100.0,
            100.0,
            150.0, // Changed ask size
            2,
            0.01,
            0,
            100.0,
            10,
        );
        let mut trade = stub_trade_tick_eth_usdt();
        trade.aggressor_side = AggressorSide::Buyer;

        ratio.handle_book(&book);
        ratio.handle_trade_tick(&trade);

        assert_eq!(ratio.count, 2);
        assert_eq!(ratio.trade_volume, 1.0);
        assert_eq!(ratio.order_volume, 250.0);
        assert_eq!(ratio.value, 0.02); // 1.0 / (250.0 - 250.0)
        assert!(ratio.initialized);
    }

    #[rstest]
    fn test_handle_trade_with_seller_aggressor() {
        let mut ratio = TradeToOrderRatio::new(2);
        let book = stub_order_book_mbp(
            InstrumentId::from("ETH/USDT.BINANCE"),
            101.0,
            100.0,
            150.0, // Changed bid size
            100.0,
            2,
            0.01,
            0,
            100.0,
            10,
        );
        let mut trade = stub_trade_tick_eth_usdt();
        trade.aggressor_side = AggressorSide::Seller;

        ratio.handle_book(&book);
        ratio.handle_trade_tick(&trade);

        assert_eq!(ratio.count, 2);
        assert_eq!(ratio.trade_volume, -1.0);
        assert_eq!(ratio.order_volume, 250.0);
        assert_eq!(ratio.value, -0.02); // -1.0 / (250.0 - 250.0)
        assert!(ratio.initialized);
    }

    #[rstest]
    fn test_reset_calculation() {
        let mut ratio = TradeToOrderRatio::new(2);
        let book = stub_order_book_mbp(
            InstrumentId::from("ETH/USDT.BINANCE"),
            101.0,
            100.0,
            100.0,
            100.0,
            2,
            0.01,
            0,
            100.0,
            10,
        );
        let trade = stub_trade_tick_eth_usdt();

        ratio.handle_book(&book);
        ratio.handle_trade_tick(&trade);
        ratio.reset_calculation();

        assert_eq!(ratio.count, 0);
        assert_eq!(ratio.value, 0.0);
        assert_eq!(ratio.trade_volume, 0.0);
        assert_eq!(ratio.initial_order_volume, 200.0);
        assert!(ratio.initialized);
    }

    #[rstest]
    fn test_full_reset() {
        let mut ratio = TradeToOrderRatio::new(2);
        let book = stub_order_book_mbp(
            InstrumentId::from("ETH/USDT.BINANCE"),
            101.0,
            100.0,
            100.0,
            100.0,
            2,
            0.01,
            0,
            100.0,
            10,
        );
        let trade = stub_trade_tick_eth_usdt();

        ratio.handle_book(&book);
        ratio.handle_trade_tick(&trade);
        ratio.reset();

        assert_eq!(ratio.count, 0);
        assert_eq!(ratio.value, 0.0);
        assert_eq!(ratio.trade_volume, 0.0);
        assert_eq!(ratio.order_volume, 0.0);
        assert_eq!(ratio.initial_order_volume, 0.0);
        assert!(!ratio.initialized);
        assert!(!ratio.has_inputs);
    }
} 