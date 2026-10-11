// -------------------------------------------------------------------------------------------------
//  Copyright (C) 2015-2026 Nautech Systems Pty Ltd. All rights reserved.
//  https://nautechsystems.io
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

//! Python bindings for opaque binary-option action profiles.

use pyo3::pyclass;

use crate::models::outcome_action::BinaryOptionActionProfile;

/// An opaque, venue-provided binary-option action profile.
#[pyclass(
    frozen,
    module = "nautilus_trader.execution",
    name = "BinaryOptionActionProfile",
    unsendable,
    from_py_object
)]
#[pyo3_stub_gen::derive::gen_stub_pyclass(module = "nautilus_trader.execution")]
#[derive(Clone, Debug)]
pub struct PyBinaryOptionActionProfile {
    inner: BinaryOptionActionProfile,
}

impl PyBinaryOptionActionProfile {
    /// Wraps a venue-provided profile for Python backtest and sandbox configuration.
    #[must_use]
    pub const fn new(inner: BinaryOptionActionProfile) -> Self {
        Self { inner }
    }

    /// Returns the wrapped action profile.
    #[must_use]
    pub fn profile(&self) -> BinaryOptionActionProfile {
        self.inner.clone()
    }
}
