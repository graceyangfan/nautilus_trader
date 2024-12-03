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

use std::collections::VecDeque;
use std::fmt::Display;

use nautilus_core::nanos::UnixNanos;
use nautilus_model::{
    enums::OrderSide,
    orderbook::book::OrderBook,
    types::price::Price,
};

use crate::indicator::Indicator;

/// Represents the state of market depletion monitoring.
#[derive(Debug)]
struct DepletionState {
    initial_book: Option<OrderBook>,
    end_book: Option<OrderBook>,
    depletion_side: OrderSide,
    recovery_side: OrderSide,
    initial_price: Price,
    start_time: Option<UnixNanos>,
    end_time: Option<UnixNanos>,
    timeout: UnixNanos,
}

impl DepletionState {
    fn new(timeout: UnixNanos) -> Self {
        Self {
            initial_book: None,
            end_book: None,
            depletion_side: OrderSide::NoOrderSide,
            recovery_side: OrderSide::NoOrderSide,
            initial_price: Price::from("0.0"),
            start_time: None,
            end_time: None,
            timeout,
        }
    }

    fn set_initial(&mut self, book: OrderBook, side: OrderSide, price: Price) {
        self.initial_book = Some(book.clone());
        self.depletion_side = side;
        self.initial_price = price;
        self.start_time = Some(book.ts_last);
        self.end_time = None;
    }

    fn set_end(&mut self, book: OrderBook, side: OrderSide) {
        self.end_book = Some(book.clone());
        self.recovery_side = side;
        self.end_time = Some(book.ts_last);
    }

    fn elapsed(&self) -> UnixNanos {
        match (self.start_time, self.end_time) {
            (Some(start), Some(end)) => end - start,
            _ => self.timeout,
        }
    }

    fn is_timeout(&self, current_time: UnixNanos) -> bool {
        self.start_time
            .map(|start| (current_time - start) > self.timeout)
            .unwrap_or(false)
    }

    fn is_running(&self) -> bool {
        self.start_time.is_some() && self.end_time.is_none()
    }

    fn reset(&mut self) {
        self.initial_book = None;
        self.end_book = None;
        self.depletion_side = OrderSide::NoOrderSide;
        self.recovery_side = OrderSide::NoOrderSide;
        self.initial_price = Price::from("0.0");
        self.start_time = None;
        self.end_time = None;
    }
}

/// Market resilience indicator that analyzes order book changes to detect market depletion and recovery.
#[repr(C)]
#[derive(Debug)]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "nautilus_trader.core.nautilus_pyo3.indicators")
)]
pub struct MarketResilienceIndicator {
    /// The current resilience score
    pub score: f64,
    /// The bias side of the market
    pub bias_side: OrderSide,
    /// The side that experienced depletion
    pub depletion_side: OrderSide,
    /// The side that showed recovery
    pub recovery_side: OrderSide,
    /// The time taken for recovery
    pub recovery_time: UnixNanos,
    /// The number of updates processed
    pub count: usize,
    /// Whether the indicator is initialized
    pub initialized: bool,
    /// Whether the indicator has inputs
    pub has_inputs: bool,
    /// Configuration parameters
    pub timeout: UnixNanos,
    pub spread_window_size: usize,
    pub levels_to_consume: usize,
    pub spread_increase_threshold: f64,
    pub strong_resilience_threshold: f64,
    pub weak_resilience_threshold: f64,
    pub time_weight: f64,
    pub depth_weight: f64,
    pub spread_weight: f64,
    pub same_side_bias: f64,
    pub opposite_side_bias: f64,
    
    recent_spreads: VecDeque<f64>,
    depletion_state: DepletionState,
    previous_book: Option<OrderBook>,
    pub is_spread_recovered: bool,
    pub is_strong_reversal: bool,
    pub is_depletion_continuing: bool,
    pub bias_side: OrderSide,
}

impl Display for MarketResilienceIndicator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}(score={}, bias_side={:?}, depletion_side={:?}, recovery_side={:?}, recovery_time={:?})",
            self.name(),
            self.score,
            self.bias_side,
            self.depletion_side,
            self.recovery_side,
            self.recovery_time,
        )
    }
}

impl MarketResilienceIndicator {
    /// Creates a new MarketResilienceIndicator with custom parameters.
    ///
    /// # Parameters
    /// - `timeout_ms`: The timeout duration in milliseconds for recovery monitoring.
    /// - `spread_window_size`: The window size for averaging the spread.
    /// - `levels_to_consume`: The number of levels to monitor for depletion.
    /// - `spread_increase_threshold`: The threshold for detecting a significant spread increase.
    /// - `strong_resilience_threshold`: The threshold for classifying strong resilience.
    /// - `weak_resilience_threshold`: The threshold for classifying weak resilience.
    /// - `time_weight`: The weight for time recovery in the resilience score.
    /// - `depth_weight`: The weight for depth recovery in the resilience score.
    /// - `spread_weight`: The weight for spread recovery in the resilience score.
    /// - `same_side_bias`: The bias adjustment when recovery is on the same side as depletion.
    /// - `opposite_side_bias`: The bias adjustment when recovery is on the opposite side of depletion.
    #[must_use]
    pub fn new(
        timeout_ms: Option<u64>,
        spread_window_size: Option<usize>,
        levels_to_consume: Option<usize>,
        spread_increase_threshold: Option<f64>,
        strong_resilience_threshold: Option<f64>,
        weak_resilience_threshold: Option<f64>,
        time_weight: Option<f64>,
        depth_weight: Option<f64>,
        spread_weight: Option<f64>,
        same_side_bias: Option<f64>,
        opposite_side_bias: Option<f64>,
    ) -> Self {
        Self {
            score: 0.0,
            bias_side: OrderSide::NoOrderSide,
            depletion_side: OrderSide::NoOrderSide,
            recovery_side: OrderSide::NoOrderSide,
            recovery_time: UnixNanos::from(timeout_ms.unwrap_or(500) * 1_000_000), // Default 500ms
            count: 0,
            initialized: false,
            has_inputs: false,
            timeout: UnixNanos::from(timeout_ms.unwrap_or(500) * 1_000_000), // Default 500ms
            spread_window_size: spread_window_size.unwrap_or(50),
            levels_to_consume: levels_to_consume.unwrap_or(3),
            spread_increase_threshold: spread_increase_threshold.unwrap_or(1.0),
            strong_resilience_threshold: strong_resilience_threshold.unwrap_or(0.7),
            weak_resilience_threshold: weak_resilience_threshold.unwrap_or(0.3),
            time_weight: time_weight.unwrap_or(0.5),
            depth_weight: depth_weight.unwrap_or(0.0),
            spread_weight: spread_weight.unwrap_or(0.5),
            same_side_bias: same_side_bias.unwrap_or(0.5),
            opposite_side_bias: opposite_side_bias.unwrap_or(-0.5),
            recent_spreads: VecDeque::with_capacity(spread_window_size.unwrap_or(50)),
            depletion_state: DepletionState::new(UnixNanos::from(timeout_ms.unwrap_or(500) * 1_000_000)),
            previous_book: None,
            is_spread_recovered: false,
            is_strong_reversal: false,
            is_depletion_continuing: false,
            bias_side: OrderSide::NoOrderSide,
        }
    }

    fn detect_depletion(&self, previous: &OrderBook, current: &OrderBook) -> Option<(OrderSide, Price)> {
        // Check for bid side depletion
        if let Some(prev_bid_threshold) = previous.bids().nth(self.levels_to_consume - 1) {
            let threshold_price = prev_bid_threshold.price.value;
            if let Some(current_best_bid) = current.best_bid_price() {
                if current_best_bid < threshold_price {
                    return Some((OrderSide::Buy, previous.best_bid_price().unwrap()));
                }
            }
        }

        // Check for ask side depletion
        if let Some(prev_ask_threshold) = previous.asks().nth(self.levels_to_consume - 1) {
            let threshold_price = prev_ask_threshold.price.value;
            if let Some(current_best_ask) = current.best_ask_price() {
                if current_best_ask > threshold_price {
                    return Some((OrderSide::Sell, previous.best_ask_price().unwrap()));
                }
            }
        }

        None
    }

    fn is_spread_increased(&self, book: &OrderBook) -> bool {
        if self.recent_spreads.is_empty() {
            return false;
        }

        let avg_spread = self.recent_spreads.iter().sum::<f64>() / self.recent_spreads.len() as f64;
        book.spread()
            .map(|spread| spread > avg_spread * (1.0 + self.spread_increase_threshold))
            .unwrap_or(false)
    }

    fn is_spread_back_to_average(&self, book: &OrderBook) -> bool {
        if self.recent_spreads.is_empty() {
            return false;
        }

        let avg_spread = self.recent_spreads.iter().sum::<f64>() / self.recent_spreads.len() as f64;
        book.spread()
            .map(|spread| spread <= avg_spread)
            .unwrap_or(false)
    }

    fn get_book_recovery_side(&self, current: &OrderBook) -> OrderSide {
        let initial_price = self.depletion_state.initial_price;
        let depletion_side = self.depletion_state.depletion_side;

        match depletion_side {
            OrderSide::Buy => {
                if let Some(best_bid) = current.best_bid_price() {
                    if best_bid >= initial_price {
                        OrderSide::Buy
                    } else {
                        OrderSide::Sell
                    }
                } else {
                    OrderSide::NoOrderSide
                }
            }
            OrderSide::Sell => {
                if let Some(best_ask) = current.best_ask_price() {
                    if best_ask <= initial_price {
                        OrderSide::Sell
                    } else {
                        OrderSide::Buy
                    }
                } else {
                    OrderSide::NoOrderSide
                }
            }
            OrderSide::NoOrderSide => OrderSide::NoOrderSide,
        }
    }

    fn calculate_resilience_metrics(&mut self) {
        let depletion_side = self.depletion_state.depletion_side;
        let recovery_side = self.depletion_state.recovery_side;
        let recovery_time = self.depletion_state.elapsed();
        
        let initial_book = self.depletion_state.initial_book.as_ref().unwrap();
        let end_book = self.depletion_state.end_book.as_ref().unwrap();

        let (normalized_time, spread_recovery, depth_recovery) = 
            self.calculate_normalized_metrics(initial_book, end_book, recovery_time);

        let base_score = self.time_weight * normalized_time
            + self.depth_weight * depth_recovery
            + self.spread_weight * spread_recovery;

        let has_recovered_same_side = depletion_side == recovery_side;
        let bias_score = if has_recovered_same_side { self.same_side_bias_side } else { self.opposite_side_bias_side };
        self.score = (base_score + bias_score).max(0.0).min(1.0);

        self.is_spread_recovered = true;
        self.bias_side = recovery_side;
        
        self.is_strong_reversal = self.score >= self.strong_resilience_threshold && has_recovered_same_side;
        self.is_depletion_continuing = self.score < self.weak_resilience_threshold && !has_recovered_same_side;

        self.depletion_side = depletion_side;
        self.recovery_side = recovery_side;
        self.recovery_time = recovery_time;
    }

    fn calculate_normalized_metrics(
        &self,
        initial_book: &OrderBook,
        end_book: &OrderBook,
        recovery_time: UnixNanos,
    ) -> (f64, f64, f64) {
        // Normalized time recovery
        let normalized_time = 1.0 - (recovery_time.as_f64() / self.timeout.as_f64());
        let normalized_time = normalized_time.max(0.0);

        // Normalized spread recovery
        let spread_recovery = initial_book.spread()
            .zip(end_book.spread())
            .map(|(initial, end)| {
                if initial > 0.0 {
                    ((initial - end) / initial).max(0.0).min(1.0)
                } else {
                    0.0
                }
            })
            .unwrap_or(0.0);

        // Normalized depth recovery
        let initial_depth = (initial_book.bids().count() + initial_book.asks().count()) as f64;
        let end_depth = (end_book.bids().count() + end_book.asks().count()) as f64;
        let depth_recovery = if initial_depth > 0.0 {
            (end_depth / initial_depth).max(0.0).min(1.0)
        } else {
            0.0
        };

        (normalized_time, spread_recovery, depth_recovery)
    }

    fn reset_monitoring(&mut self) {
        self.depletion_state.reset();
        self.previous_book = None;
    }
}

impl Indicator for MarketResilienceIndicator {
    fn name(&self) -> String {
        stringify!(MarketResilienceIndicator).to_string()
    }

    fn has_inputs(&self) -> bool {
        self.has_inputs
    }

    fn initialized(&self) -> bool {
        self.initialized
    }

    fn handle_book(&mut self, book: &OrderBook) {
        self.is_spread_recovered = false;
        self.is_strong_reversal = false;
        self.is_depletion_continuing = false;

        self.has_inputs = true;
        self.count += 1;

        if self.depletion_state.is_running() {
            if self.is_spread_back_to_average(book) {
                let recovery_side = self.get_book_recovery_side(book);
                if recovery_side != OrderSide::NoOrderSide {
                    self.depletion_state.set_end(book.clone(), recovery_side);
                    self.calculate_resilience_metrics();
                    self.reset_monitoring();
                }
            } else if self.depletion_state.is_timeout(book.ts_last) {
                self.score = 0.0;
                self.bias_side = OrderSide::NoOrderSide;
                self.depletion_side = self.depletion_state.depletion_side;
                self.recovery_side = OrderSide::NoOrderSide;
                self.recovery_time = self.timeout;
                self.reset_monitoring();
            }
        } else {
            // Update average spread
            if let Some(spread) = book.spread() {
                self.recent_spreads.push_back(spread);
                if self.recent_spreads.len() > self.spread_window_size {
                    self.recent_spreads.pop_front();
                }
            }

            // Check for depletion
            if let Some(previous) = &self.previous_book {
                if let Some((side, price)) = self.detect_depletion(previous, book) {
                    if self.is_spread_increased(book) {
                        self.depletion_state.set_initial(book.clone(), side, price);
                    }
                }
            }
            
            self.previous_book = Some(book.clone());
        }

        self.initialized = true;
    }

    fn reset(&mut self) {
        self.score = 0.0;
        self.is_spread_recovered = false;
        self.is_strong_reversal = false;
        self.is_depletion_continuing = false;
        self.bias_side = OrderSide::NoOrderSide;
        self.depletion_side = OrderSide::NoOrderSide;
        self.recovery_side = OrderSide::NoOrderSide;
        self.recovery_time = self.timeout;
        self.count = 0;
        self.initialized = false;
        self.has_inputs = false;
        self.recent_spreads.clear();
        self.depletion_state.reset();
        self.previous_book = None;
    }
}

////////////////////////////////////////////////////////////////////////////////
// Tests
////////////////////////////////////////////////////////////////////////////////
#[cfg(test)]
mod tests {
    use super::*;
    use nautilus_model::{
        identifiers::InstrumentId,
        stubs::stub_order_book_mbp,
    };
    use rstest::rstest;

    fn create_test_book() -> OrderBook {
        stub_order_book_mbp(
            InstrumentId::from("BTCUSDT.BINANCE"),
            101.0,
            100.0,
            100.0,
            100.0,
            2,
            0.01,
            0,
            100.0,
            10,
        )
    }

    fn add_test_orders(
        book: &mut OrderBook,
        top_bid_price: f64,
        top_ask_price: f64,
        top_bid_size: f64,
        top_ask_size: f64,
        price_increment: f64,
        size_increment: f64,
        num_levels: usize,
        price_precision: u8,
        size_precision: u8,
    ) {
        // Add bid orders
        for i in 0..num_levels {
            let price = Price::new(
                price_increment.mul_add(-(i as f64), top_bid_price),
                price_precision,
            );
            let size = Quantity::new(
                size_increment.mul_add(i as f64, top_bid_size),
                size_precision,
            );
            let order = BookOrder::new(
                OrderSide::Buy,
                price,
                size,
                i as u64,
            );
            book.add(order, 0, i as u64, UnixNanos::from(i as u64));
        }

        // Add ask orders
        for i in 0..num_levels {
            let price = Price::new(
                price_increment.mul_add(i as f64, top_ask_price),
                price_precision,
            );
            let size = Quantity::new(
                size_increment.mul_add(i as f64, top_ask_size),
                size_precision,
            );
            let order = BookOrder::new(
                OrderSide::Sell,
                price,
                size,
                (i + num_levels) as u64,
            );
            book.add(order, 0, i as u64, UnixNanos::from(i as u64));
        }
    }

    #[rstest]
    fn test_indicator_initialized() {
        let indicator = MarketResilienceIndicator::new(
            Some(500),
            Some(50),
            Some(3),
            Some(1.0),
            Some(0.7),
            Some(0.3),
            Some(0.5),
            Some(0.0),
            Some(0.5),
            Some(0.5),
            Some(-0.5),
        );
        
        assert_eq!(indicator.name(), "MarketResilienceIndicator");
        assert!(!indicator.has_inputs());
        assert!(!indicator.initialized());
        assert_eq!(indicator.count, 0);
        assert_eq!(indicator.score, 0.0);
        assert_eq!(indicator.bias_side, OrderSide::NoOrderSide);
    }

    #[rstest]
    fn test_detect_depletion_bid_side() {
        let mut indicator = MarketResilienceIndicator::new(
            Some(500), // 500ms
            Some(50),  // spread_window_size
            Some(3),   // levels_to_consume
            Some(1.0), // spread_increase_threshold
            Some(0.7), // strong_resilience_threshold
            Some(0.3), // weak_resilience_threshold
            Some(0.5), // time_weight
            Some(0.0), // depth_weight
            Some(0.5), // spread_weight
            Some(0.5), // same_side_bias
            Some(-0.5), // opposite_side_bias
        );

        let mut book1 = create_test_book();
        add_test_orders(&mut book1, 10.0, 10.1, 10.0, 10.0, 0.1, 0.0, 3, 5, 0);

        let mut book2 = create_test_book();
        add_test_orders(&mut book2, 9.7, 10.1, 9.7, 10.0, 0.1, 0.0, 3, 5, 0);

        if let Some((side, price)) = indicator.detect_depletion(&book1, &book2) {
            assert_eq!(side, OrderSide::Buy);
            assert_eq!(price, Price::from("10.0"));
        } else {
            panic!("Should detect bid side depletion");
        }
    }

    #[rstest]
    fn test_detect_depletion_ask_side() {
        let mut indicator = MarketResilienceIndicator::new(
            Some(500), // 500ms
            Some(50),  // spread_window_size
            Some(3),   // levels_to_consume
            Some(1.0), // spread_increase_threshold
            Some(0.7), // strong_resilience_threshold
            Some(0.3), // weak_resilience_threshold
            Some(0.5), // time_weight
            Some(0.0), // depth_weight
            Some(0.5), // spread_weight
            Some(0.5), // same_side_bias
            Some(-0.5), // opposite_side_bias
        );

        let mut book1 = create_test_book();
        add_test_orders(&mut book1, 9.8, 10.0, 10.0, 10.0, 0.1, 0.0, 3, 5, 0);

        let mut book2 = create_test_book();
        add_test_orders(&mut book2, 9.8, 10.3, 10.0, 10.0, 0.1, 0.0, 3, 5, 0);

        if let Some((side, price)) = indicator.detect_depletion(&book1, &book2) {
            assert_eq!(side, OrderSide::Sell);
            assert_eq!(price, Price::from("10.0"));
        } else {
            panic!("Should detect ask side depletion");
        }
    }

    #[rstest]
    fn test_handle_book_updates() {
        let mut indicator = MarketResilienceIndicator::new(
            Some(1000), // 1 second
            Some(10),  // spread_window_size
            Some(3),   // levels_to_consume
            Some(1.0), // spread_increase_threshold
            Some(0.7), // strong_resilience_threshold
            Some(0.3), // weak_resilience_threshold
            Some(0.5), // time_weight
            Some(0.0), // depth_weight
            Some(0.5), // spread_weight
            Some(0.5), // same_side_bias
            Some(-0.5), // opposite_side_bias
        );

        // Initial book state
        let mut book1 = create_test_book();
        add_test_orders(&mut book1, 10.0, 10.1, 10.0, 10.0, 0.1, 0.0, 3, 5, 0);
        indicator.handle_book(&book1);

        assert!(indicator.has_inputs());
        assert!(indicator.initialized());
        assert_eq!(indicator.count, 1);
        assert_eq!(indicator.score, 0.0);

        // Depleted state
        let mut book2 = create_test_book();
        add_test_orders(&mut book2, 9.7, 10.1, 9.7, 10.0, 0.1, 0.0, 3, 5, 0);
        book2.ts_last = UnixNanos::from(100_000_000);
        indicator.handle_book(&book2);

        // Recovery state
        let mut book3 = create_test_book();
        add_test_orders(&mut book3, 10.0, 10.1, 10.0, 10.0, 0.1, 0.0, 3, 5, 0);
        book3.ts_last = UnixNanos::from(200_000_000);
        indicator.handle_book(&book3);

        assert!(indicator.score > 0.0);
        assert_eq!(indicator.depletion_side, OrderSide::Buy);
        assert!(matches!(indicator.bias_side, OrderSide::Sell | OrderSide::Buy));
    }

    #[rstest]
    fn test_reset() {
        let mut indicator = MarketResilienceIndicator::new(
            Some(500), // 500ms
            Some(50),  // spread_window_size
            Some(3),   // levels_to_consume
            Some(1.0), // spread_increase_threshold
            Some(0.7), // strong_resilience_threshold
            Some(0.3), // weak_resilience_threshold
            Some(0.5), // time_weight
            Some(0.0), // depth_weight
            Some(0.5), // spread_weight
            Some(0.5), // same_side_bias
            Some(-0.5), // opposite_side_bias
        );
        
        let mut book = create_test_book();
        add_test_orders(&mut book, 10.0, 10.1, 10.0, 10.0, 0.1, 0.0, 3, 5, 0);
        indicator.handle_book(&book);
        
        indicator.reset();
        
        assert!(!indicator.has_inputs());
        assert!(!indicator.initialized());
        assert_eq!(indicator.count, 0);
        assert_eq!(indicator.score, 0.0);
        assert_eq!(indicator.bias_side, OrderSide::NoOrderSide);
        assert!(indicator.recent_spreads.is_empty());
    }

    #[rstest]
    fn test_empty_order_book() {
        let mut indicator = MarketResilienceIndicator::new(
            Some(500), // 500ms
            Some(50),  // spread_window_size
            Some(3),   // levels_to_consume
            Some(1.0), // spread_increase_threshold
            Some(0.7), // strong_resilience_threshold
            Some(0.3), // weak_resilience_threshold
            Some(0.5), // time_weight
            Some(0.0), // depth_weight
            Some(0.5), // spread_weight
            Some(0.5), // same_side_bias
            Some(-0.5), // opposite_side_bias
        );

        let book = OrderBook::new(
            InstrumentId::from("BTCUSDT.BINANCE"),
            BookType::L2_MBP,
        );

        indicator.handle_book(&book);
        assert_eq!(indicator.score, 0.0);
        assert_eq!(indicator.bias_side, OrderSide::NoOrderSide);
    }

    #[rstest]
    fn test_invalid_input() {
        let mut indicator = MarketResilienceIndicator::new(
            Some(500), // 500ms
            Some(50),  // spread_window_size
            Some(3),   // levels_to_consume
            Some(1.0), // spread_increase_threshold
            Some(0.7), // strong_resilience_threshold
            Some(0.3), // weak_resilience_threshold
            Some(0.5), // time_weight
            Some(0.0), // depth_weight
            Some(0.5), // spread_weight
            Some(0.5), // same_side_bias
            Some(-0.5), // opposite_side_bias
        );

        let mut book = create_test_book();
        add_test_orders(&mut book, -10.0, -10.1, -10.0, -10.0, 0.1, 0.0, 3, 5, 0);

        indicator.handle_book(&book);
        assert_eq!(indicator.score, 0.0);
        assert_eq!(indicator.bias_side, OrderSide::NoOrderSide);
    }
} 