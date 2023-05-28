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

from collections import deque
import numpy as np

cimport numpy as np

from libc.stdint cimport uint64_t

from nautilus_trader.core.correctness cimport Condition
from nautilus_trader.indicators.base.indicator cimport Indicator
from nautilus_trader.model.data.bar cimport Bar


cdef class Zigzag(Indicator):
    """
    A zigzag indicator which calculates and stores peak and trough.

    References
    ----------
    https://github.com/jbn/ZigZag/issues/11
    """

    def __init__(
        self,
        double change_percent,
        bint full_close,
        int bins_num = 10,
    ):
        """
        Initialize a new instance of the zigzag class.

        Parameters
        ----------
        """
        params = [
            change_percent,
            full_close,
            bins_num 
        ]
        super().__init__(params=params)

        self.change_percent = change_percent
        self.full_close = full_close
        self.bins_num = bins_num 
        self.threshold = 0

        self.virtual_high = 0
        self.virtual_low = 0
        self.virtual_length = 0
        self.virtual_direction = 0

        self.zigzags_values = deque(maxlen = 3)
        self.zigzags_Type = deque(maxlen = 3)
        self.zigzags_datetime = deque(maxlen = 3)
        self.price_array = [] 
        self.volume_array = [] 
        self.last_ts_event = 0 
        self.num_bars = 0 
        self.poi = 0 
        self.sum_volume  = 0 
        self.sum_value = 0 
        self.anchored_vwap  = 0 

    cpdef void handle_bar(self, Bar bar) except *:
        """
        Update the indicator with the given bar.

        Parameters
        ----------
        bar : Bar
            The update bar.

        """
        Condition.not_none(bar, "bar")
        if self.full_close:
            self.update_raw(
                bar.open.as_double(),
                bar.close.as_double(),
                bar.close.as_double(),
                bar.close.as_double(),
                bar.volume.as_double(),
                bar.ts_event,
            )
        else:
            self.update_raw(
                bar.open.as_double(),
                bar.high.as_double(),
                bar.low.as_double(),
                bar.close.as_double(),
                bar.volume.as_double(),
                bar.ts_event,
            )

    cpdef void update_raw(
        self,
        double open,
        double high,
        double low,
        double close,
        double volume,
        uint64_t timestamp,
    ) except *:
        """
        Update the indicator with the given raw values.

        Parameters
        ----------
        open : double
            The oepn price.
        high : double
            The high price.
        low : double
            The low price.
        close : double
            The close price.
        timestamp : datetime
            The current timestamp.

        """

        self.price_array.append(close)
        self.volume_array.append(volume)
        self.sum_value += close * volume
        self.sum_volume += volume
        self.anchored_vwap = self.sum_value / self.sum_volume
        if not self.initialized:
            self._set_has_inputs(True)
            if len(self.zigzags_values) >=3:
                self._set_initialized(True)
            ##init the zigzags
            if  not self.zigzags_values:
                self.zigzags_values.append(close)
                self.zigzags_Type.append(None)
                self.zigzags_datetime.append(timestamp)
                return
            self.threshold = self.change_percent
            if len(self.zigzags_values) == 1:
                perc_change_since_pivot = self.calc_change_since_pivot(close)

                if(abs(perc_change_since_pivot) >= self.threshold):
                    if(perc_change_since_pivot > 0):
                        self.zigzags_values.append(high)
                        self.zigzags_Type.append("Peak")
                        self.zigzags_datetime.append(timestamp)
                        self.zigzags_Type[-2] = "Trough"
                        return
                    else:
                        self.zigzags_values.append(low)
                        self.zigzags_Type.append("Trough")
                        self.zigzags_datetime.append(timestamp)
                        self.zigzags_Type[-2] = "Peak"
                        return
                else:
                    return  #waiting for

        self.threshold = self.change_percent
        is_trough = self.zigzags_values[-2] > self.zigzags_values[-1]
        last_pivot =  self.zigzags_values[-1]
        if is_trough:
            perc_change_since_pivot = self.calc_change_since_pivot(high)
            is_reversing = perc_change_since_pivot >= self.threshold
            is_continuing = low <=last_pivot
            if  is_continuing:
                self.zigzags_values[-1] = low
                self.zigzags_Type[-1] = "Trough"
                self.zigzags_datetime[-1] = timestamp
            elif is_reversing:
                self.zigzags_values.append(high)
                self.zigzags_Type.append("Peak")
                self.zigzags_datetime.append(timestamp)
                if self.last_ts_event > 0 and len(self.zigzags_datetime)>=3:
                    self.num_bars = (timestamp - self.zigzags_datetime[-3]) / (timestamp - self.last_ts_event)
                    self.price_array = self.price_array[-self.num_bars:]
                    self.volume_array = self.volume_array[-self.num_bars:]
                    self.sum_value = 0 
                    self.sum_volume = 0 
                #if self.last_ts_event > 0 and len(self.zigzags_datetime)>=3:
                    #self.num_bars = (timestamp - self.zigzags_datetime[-2]) / (timestamp - self.last_ts_event)
                    #self.poi = np.dot(self.price_array[:-self.num_bars],self.volume_array[:-self.num_bars]) / np.sum(self.volume_array[:-self.num_bars])
        
        else:
            perc_change_since_pivot = self.calc_change_since_pivot(low)
            is_reversing = perc_change_since_pivot <=-self.threshold
            is_continuing = high >= last_pivot
            if is_continuing:
                self.zigzags_values[-1] = high
                self.zigzags_Type[-1] = "Peak"
                self.zigzags_datetime[-1] = timestamp
            elif is_reversing:
                self.zigzags_values.append(low)
                self.zigzags_Type.append("Trough")
                self.zigzags_datetime.append(timestamp)
                if self.last_ts_event > 0 and len(self.zigzags_datetime)>=3:
                    self.num_bars = (timestamp - self.zigzags_datetime[-3]) / (timestamp - self.last_ts_event)
                    self.price_array = self.price_array[-self.num_bars:]
                    self.volume_array = self.volume_array[-self.num_bars:]
                    self.sum_value = 0 
                    self.sum_volume = 0 
                #if self.last_ts_event > 0 and len(self.zigzags_datetime)>=3:
                    #self.num_bars = (timestamp - self.zigzags_datetime[-2]) / (timestamp - self.last_ts_event)
                    #self.poi = np.dot(self.price_array[:-self.num_bars],self.volume_array[:-self.num_bars]) / np.sum(self.volume_array[:-self.num_bars])

        ## compute previous pivot
        if self.zigzags_datetime[-1] != timestamp:
            if self.zigzags_Type[-1] == "Trough":
                self.virtual_low = self.zigzags_values[-1]
                self.virtual_high = self.zigzags_values[-2]
                self.virtual_direction = -1
            else:
                self.virtual_low = self.zigzags_values[-2]
                self.virtual_high = self.zigzags_values[-1]
                self.virtual_direction = 1

        self.virtual_length  = self.virtual_high - self.virtual_low
        self.last_ts_event = timestamp

    cpdef double calc_change_since_pivot(self,double current_value) except *:
        last_pivot = self.zigzags_values[-1]
        if(last_pivot == 0): last_pivot = 1 ** (-100) # avoid division by 0
        perc_change_since_pivot = (current_value - last_pivot) / abs(last_pivot)
        return perc_change_since_pivot


    cpdef void _reset(self) except *:
        self.zigzags_values.clear()
        self.zigzags_Type.clear()
        self.zigzags_datetime.clear()
        self.price_array[:] = [] 
        self.volume_array[:] = [] 
        self.last_ts_event = 0 
        self.num_bars = 0 
        self.poi = 0 
        self.sum_volume  = 0 
        self.sum_value = 0 
        self.anchored_vwap  = 0       
        self.threshold = 0
        self.virtual_high = 0
        self.virtual_low = 0
        self.virtual_length = 0
        self.virtual_direction = 0
