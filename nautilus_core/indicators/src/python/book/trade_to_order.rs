use nautilus_model::{orderbook::book::OrderBook, data::trade::TradeTick};
use pyo3::prelude::*;

use crate::{book::trade_to_order::TradeToOrderRatio, indicator::Indicator};

#[pymethods]
impl TradeToOrderRatio {
    #[new]
    #[pyo3(signature = (depth = 20))]
    fn py_new(depth: usize) -> Self {
        Self::new(depth)
    }

    fn __repr__(&self) -> String {
        self.to_string()
    }

    #[getter]
    #[pyo3(name = "name")]
    fn py_name(&self) -> String {
        self.name()
    }

    #[getter]
    #[pyo3(name = "count")]
    fn py_count(&self) -> usize {
        self.count
    }

    #[getter]
    #[pyo3(name = "value")]
    fn py_value(&self) -> f64 {
        self.value
    }

    #[getter]
    #[pyo3(name = "has_inputs")]
    fn py_has_inputs(&self) -> bool {
        self.has_inputs()
    }

    #[getter]
    #[pyo3(name = "initialized")]
    fn py_initialized(&self) -> bool {
        self.initialized
    }

    #[pyo3(name = "handle_book")]
    fn py_handle_book(&mut self, book: &OrderBook) {
        self.handle_book(book);
    }

    #[pyo3(name = "handle_trade_tick")]
    fn py_handle_trade_tick(&mut self, trade: &TradeTick) {
        self.handle_trade_tick(trade);
    }

    #[pyo3(name = "reset")]
    fn py_reset(&mut self) {
        self.reset();
    }

    #[pyo3(name = "reset_calculation")]
    fn py_reset_calculation(&mut self) {
        self.reset_calculation();
    }
} 