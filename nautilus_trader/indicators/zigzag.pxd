# -------------------------------------------------------------------------------------------------
#  Copyright (C) 2015-2022 Nautech Systems Pty Ltd. All rights reserved.
#  https://nautechsystems.io
#
#  Licensed under the GNU Lesser General Public License Version 3.0 (the "License");
#  You may not use this file except in compliance with the License.
#  You may obtain a copy of the License at https://www.gnu.org/licenses/lgpl-3.0.en.html
#
#  Unless required by applicable law or agreed to in writing, software
#  distributed under the License is distributed on an "AS IS" BASIS,
#  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
#  See the License for the specific language governing permissions and
#  limitations under the License.
# -------------------------------------------------------------------------------------------------

from libc.stdint cimport uint64_t

from nautilus_trader.indicators.base.indicator cimport Indicator
from nautilus_trader.model.data.bar cimport Bar


cdef class Zigzag(Indicator):
    cdef readonly object zigzags_values
    cdef object zigzags_Type
    cdef readonly list price_array
    cdef readonly list volume_array 
    cdef readonly object zigzags_datetime

    cdef readonly double sum_volume 
    """The sum volume of bars in current zigzag_line.\n\n:returns: `double`"""
    cdef readonly double sum_value
    """The sum value of bars in current zigzag_line.\n\n:returns: `double`"""
    cdef readonly double anchored_vwap  
    """The anchored vwap in current zigzag_line.\n\n:returns: `double`"""
    cdef readonly uint64_t last_ts_event
    """The last bar ts_event.\n\n:returns: `uint64`"""
    cdef readonly double poi 
    """The poi price in current zigzag line.\n\n:returns: `double`"""
    cdef readonly int num_bars 
    """The number of bars for different computing.\n\n:returns: `int`"""

    cdef readonly double change_percent
    """The zigzag change_percent .\n\n:returns: `double`"""
    cdef readonly bint full_close
    """The zigzag full_close param.\n\n:returns: `bool`"""
    cdef readonly int bins_num
    """The bins number for price array.\n\n:returns: `int`"""
    cdef readonly double threshold
    """The zigzag threshold .\n\n:returns: `double`"""
    cdef readonly double virtual_high
    """The virtual zigzag high price.\n\n:returns: `double`"""
    cdef readonly double virtual_low
    """The virtual zigzag low price.\n\n:returns: `double`"""
    cdef readonly double virtual_length 
    """The length of the virtual zigzag.\n\n:returns: `double`"""
    cdef readonly int virtual_direction 
    """The current virtual zigzag direction.\n\n:returns: `int`"""


    cpdef void handle_bar(self, Bar bar) except *
    cpdef void update_raw(self,double open, double high, double low, double close, double volume, uint64_t timestamp) except *
    cpdef double calc_change_since_pivot(self,double current_value) except *

