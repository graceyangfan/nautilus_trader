# -------------------------------------------------------------------------------------------------
#  Copyright (C) 2015-2025 Nautech Systems Pty Ltd. All rights reserved.
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

from decimal import Decimal
from typing import Any

from nautilus_trader.core.data import Data
from nautilus_trader.model.identifiers import InstrumentId
from nautilus_trader.model.objects import Price
from nautilus_trader.model.functions import order_side_to_str
from nautilus_trader.model.functions import order_side_from_str
from nautilus_trader.model.functions import order_type_to_str
from nautilus_trader.model.functions import order_type_from_str
from nautilus_trader.model.functions import time_in_force_to_str
from nautilus_trader.model.functions import time_in_force_from_str
from nautilus_trader.model.enums import OrderSide
from nautilus_trader.model.enums import OrderType
from nautilus_trader.model.enums import TimeInForce
from nautilus_trader.model.enums import OrderStatus


class BinanceFuturesMarkPriceUpdate(Data):
    """
    Represents a Binance Futures mark price and funding rate update.

    Parameters
    ----------
    instrument_id : InstrumentId
        The instrument ID for the update.
    mark : Price
        The mark price for the instrument.
    index : Price
        The index price for the instrument.
    estimated_settle : Price
        The estimated settle price for the instrument
        (only useful in the last hour before the settlement starts).
    funding_rate : Decimal
        The current funding rate for the instrument.
    ts_next_funding : uint64_t
        UNIX timestamp (nanoseconds) when next funding will occur.
    ts_event : uint64_t
        UNIX timestamp (nanoseconds) when the data event occurred.
    ts_init : uint64_t
        UNIX timestamp (nanoseconds) when the data object was initialized.

    References
    ----------
    https://binance-docs.github.io/apidocs/futures/en/#mark-price-stream

    """

    def __init__(
        self,
        instrument_id: InstrumentId,
        mark: Price,
        index: Price,
        estimated_settle: Price,
        funding_rate: Decimal,
        ts_next_funding: int,
        ts_event: int,
        ts_init: int,
    ):
        self.instrument_id = instrument_id
        self.mark = mark
        self.index = index
        self.estimated_settle = estimated_settle
        self.funding_rate = funding_rate
        self.ts_next_funding = ts_next_funding
        self._ts_event = ts_event
        self._ts_init = ts_init

    def __repr__(self) -> str:
        return (
            f"{type(self).__name__}("
            f"instrument_id={self.instrument_id}, "
            f"mark={self.mark}, "
            f"index={self.index}, "
            f"estimated_settle={self.estimated_settle}, "
            f"funding_rate={self.funding_rate}, "
            f"ts_next_funding={self.ts_next_funding}, "
            f"ts_event={self.ts_event}, "
            f"ts_init={self.ts_init})"
        )

    @property
    def ts_event(self) -> int:
        """
        UNIX timestamp (nanoseconds) when the data event occurred.

        Returns
        -------
        int

        """
        return self._ts_event

    @property
    def ts_init(self) -> int:
        """
        UNIX timestamp (nanoseconds) when the object was initialized.

        Returns
        -------
        int

        """
        return self._ts_init

    @staticmethod
    def from_dict(values: dict[str, Any]) -> "BinanceFuturesMarkPriceUpdate":
        """
        Return a Binance Futures mark price update parsed from the given values.

        Parameters
        ----------
        values : dict[str, Any]
            The values for initialization.

        Returns
        -------
        BinanceFuturesMarkPriceUpdate

        """
        return BinanceFuturesMarkPriceUpdate(
            instrument_id=InstrumentId.from_str(values["instrument_id"]),
            mark=Price.from_str(values["mark"]),
            index=Price.from_str(values["index"]),
            estimated_settle=Price.from_str(values["estimated_settle"]),
            funding_rate=Decimal(values["funding_rate"]),
            ts_next_funding=values["ts_next_funding"],
            ts_event=values["ts_event"],
            ts_init=values["ts_init"],
        )

    @staticmethod
    def to_dict(obj: "BinanceFuturesMarkPriceUpdate") -> dict[str, Any]:
        """
        Return a dictionary representation of this object.

        Returns
        -------
        dict[str, Any]

        """
        return {
            "type": type(obj).__name__,
            "instrument_id": str(obj.instrument_id),
            "mark": str(obj.mark),
            "index": str(obj.index),
            "estimated_settle": str(obj.estimated_settle),
            "funding_rate": str(obj.funding_rate),
            "ts_next_funding": obj.ts_next_funding,
            "ts_event": obj.ts_event,
            "ts_init": obj.ts_init,
        }


class BinanceFuturesLiquidationOrder(Data):
    """
    Represents a Binance Futures liquidation order.

    Parameters
    ----------
    instrument_id : InstrumentId
        The instrument ID for the liquidation order.
    order_side : OrderSide
        The side of the liquidation order (BUY/SELL).
    order_type : OrderType 
        The type of the liquidation order (usually LIMIT).
    time_in_force : TimeInForce
        The time in force of the liquidation order (usually IOC).
    original_quantity : Quantity
        The original quantity of the liquidation order.
    price : Price
        The price of the liquidation order.
    avg_price : Price
        The average fill price of the liquidation order.
    order_status : OrderStatus
        The status of the liquidation order.
    last_filled_quantity : Quantity
        The last filled quantity.
    accumulated_filled_quantity : Quantity
        The accumulated filled quantity.
    ts_event : uint64_t
        The timestamp when the event occurred.
    ts_init : uint64_t
        The timestamp when the object was initialized.

    References
    ----------
    https://developers.binance.com/docs/derivatives/coin-margined-futures/websocket-market-streams/Liquidation-Order-Streams
    """

    def __init__(
        self,
        instrument_id: InstrumentId,
        order_side: OrderSide,
        order_type: OrderType,
        time_in_force: TimeInForce,
        original_quantity: Quantity,
        price: Price,
        avg_price: Price,
        order_status: OrderStatus,
        last_filled_quantity: Quantity,
        accumulated_filled_quantity: Quantity,
        ts_event: int,
        ts_init: int,
    ):
        self.instrument_id = instrument_id
        self.order_side = order_side
        self.order_type = order_type
        self.time_in_force = time_in_force
        self.original_quantity = original_quantity
        self.price = price
        self.avg_price = avg_price
        self.order_status = order_status
        self.last_filled_quantity = last_filled_quantity
        self.accumulated_filled_quantity = accumulated_filled_quantity
        self._ts_event = ts_event
        self._ts_init = ts_init

    def __repr__(self) -> str:
        return (
            f"{type(self).__name__}("
            f"instrument_id={self.instrument_id}, "
            f"order_side={order_side_to_str(self.order_side)}, "
            f"order_type={order_type_to_str(self.order_type)}, "
            f"time_in_force={time_in_force_to_str(self.time_in_force)}, "
            f"original_quantity={self.original_quantity}, "
            f"price={self.price}, "
            f"avg_price={self.avg_price}, "
            f"order_status={self.order_status}, "
            f"last_filled_quantity={self.last_filled_quantity}, "
            f"accumulated_filled_quantity={self.accumulated_filled_quantity}, "
            f"ts_event={self.ts_event}, "
            f"ts_init={self.ts_init})"
        )

    @property
    def ts_event(self) -> int:
        return self._ts_event

    @property
    def ts_init(self) -> int:
        return self._ts_init

    @staticmethod
    def from_dict(values: dict[str, Any]) -> "BinanceFuturesLiquidationOrder":
        """
        Return a Binance Futures liquidation order parsed from the given values.
        """
        return BinanceFuturesLiquidationOrder(
            instrument_id=InstrumentId.from_str(values["instrument_id"]),
            order_side=order_side_from_str(values["order_side"]),
            order_type=order_type_from_str(values["order_type"]),
            time_in_force=time_in_force_from_str(values["time_in_force"]),
            original_quantity=Quantity.from_str(values["original_quantity"]),
            price=Price.from_str(values["price"]),
            avg_price=Price.from_str(values["avg_price"]),
            order_status=values["order_status"],
            last_filled_quantity=Quantity.from_str(values["last_filled_quantity"]),
            accumulated_filled_quantity=Quantity.from_str(values["accumulated_filled_quantity"]),
            ts_event=values["ts_event"],
            ts_init=values["ts_init"],
        )

    @staticmethod
    def to_dict(obj: "BinanceFuturesLiquidationOrder") -> dict[str, Any]:
        """
        Return a dictionary representation of this object.
        """
        return {
            "type": type(obj).__name__,
            "instrument_id": str(obj.instrument_id),
            "order_side": order_side_to_str(obj.order_side),
            "order_type": order_type_to_str(obj.order_type),
            "time_in_force": time_in_force_to_str(obj.time_in_force),
            "original_quantity": str(obj.original_quantity),
            "price": str(obj.price),
            "avg_price": str(obj.avg_price),
            "order_status": obj.order_status,
            "last_filled_quantity": str(obj.last_filled_quantity),
            "accumulated_filled_quantity": str(obj.accumulated_filled_quantity),
            "ts_event": obj.ts_event,
            "ts_init": obj.ts_init,
        }
