use nautilus_core::python::to_pyvalue_err;
use nautilus_model::{orderbook::book::OrderBook, enums::OrderSide};
use pyo3::prelude::*;

use crate::{book::resilience::MarketResilienceIndicator, indicator::Indicator};

#[pymethods]
impl MarketResilienceIndicator {
    #[new]
    fn py_new(
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
        Self::new(
            timeout_ms,
            spread_window_size,
            levels_to_consume,
            spread_increase_threshold,
            strong_resilience_threshold,
            weak_resilience_threshold,
            time_weight,
            depth_weight,
            spread_weight,
            same_side_bias,
            opposite_side_bias,
        )
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
    #[pyo3(name = "score")]
    fn py_score(&self) -> f64 {
        self.score
    }

    #[getter]
    #[pyo3(name = "bias_side")]
    fn py_bias_side(&self) -> OrderSide {
        self.bias_side
    }

    #[getter]
    #[pyo3(name = "depletion_side")]
    fn py_depletion_side(&self) -> OrderSide {
        self.depletion_side
    }

    #[getter]
    #[pyo3(name = "recovery_side")]
    fn py_recovery_side(&self) -> OrderSide {
        self.recovery_side
    }

    #[getter]
    #[pyo3(name = "recovery_time")]
    fn py_recovery_time(&self) -> u64 {
        self.recovery_time.as_u64()
    }

    #[getter]
    #[pyo3(name = "count")]
    fn py_count(&self) -> usize {
        self.count
    }

    #[getter]
    #[pyo3(name = "initialized")]
    fn py_initialized(&self) -> bool {
        self.initialized
    }

    #[getter]
    #[pyo3(name = "has_inputs")]
    fn py_has_inputs(&self) -> bool {
        self.has_inputs
    }

    #[pyo3(name = "handle_book")]
    fn py_handle_book(&mut self, book: &OrderBook) {
        self.handle_book(book);
    }

    #[pyo3(name = "reset")]
    fn py_reset(&mut self) {
        self.reset();
    }

    #[getter]
    #[pyo3(name = "is_spread_recovered")]
    fn py_is_spread_recovered(&self) -> bool {
        self.is_spread_recovered
    }

    #[getter]
    #[pyo3(name = "is_strong_reversal")]
    fn py_is_strong_reversal(&self) -> bool {
        self.is_strong_reversal
    }

    #[getter]
    #[pyo3(name = "is_depletion_continuing")]
    fn py_is_depletion_continuing(&self) -> bool {
        self.is_depletion_continuing
    }
} 