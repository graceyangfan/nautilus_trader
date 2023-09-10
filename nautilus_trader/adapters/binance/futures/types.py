# -------------------------------------------------------------------------------------------------
#  Copyright (C) 2015-2023 Nautech Systems Pty Ltd. All rights reserved.
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


class BinanceFuturesMarkPriceUpdate(Data):
    """
    Represents a `Binance Futures` mark price and funding rate update.

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
        The UNIX timestamp (nanoseconds) when next funding will occur.
    ts_event : uint64_t
        The UNIX timestamp (nanoseconds) when the data event occurred.
    ts_init : uint64_t
        The UNIX timestamp (nanoseconds) when the data object was initialized.

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
        super().__init__(ts_event=ts_event, ts_init=ts_init)

        self.instrument_id = instrument_id
        self.mark = mark
        self.index = index
        self.estimated_settle = estimated_settle
        self.funding_rate = funding_rate
        self.ts_next_funding = ts_next_funding

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

    @staticmethod
    def from_dict(values: dict[str, Any]) -> "BinanceFuturesMarkPriceUpdate":
        """
        Return a `Binance Futures` mark price update parsed from the given values.

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


class OpenInterestHist(Data):
    """
    Represents an open interest statistical update.

    Parameters
    ----------
    instrument_id : InstrumentId
        The instrument ID for the update.
    sumOpenInterest : Decimal
        The sum of open interest.
    sumOpenInterestValue : Decimal
        The sum of open interest value.
    ts_event : uint64_t
        The UNIX timestamp (nanoseconds) when the data object event occurred.
    ts_init : uint64_t
        The UNIX timestamp (nanoseconds) when the data object was initialized.
    """
    def __init__(
        self,
        instrument_id: InstrumentId,
        sumOpenInterest: Decimal,
        sumOpenInterestValue: Decimal,
        ts_event: int,
        ts_init: int,
    ):
        self.instrument_id = instrument_id
        self.sumOpenInterest = sumOpenInterest
        self.sumOpenInterestValue = sumOpenInterestValue
        self._ts_event = ts_event
        self._ts_init = ts_init

    def __repr__(self) -> str:
        return (
            f"{type(self).__name__}("
            f"instrument_id={self.instrument_id}, "
            f"sumOpenInterest={self.sumOpenInterest}, "
            f"sumOpenInterestValue={self.sumOpenInterestValue}, "
            f"ts_event={self._ts_event}, "
            f"ts_init={self._ts_init})"
        )
    @staticmethod
    def from_dict(values: dict[str, Any]) -> "OpenInterestHist":
        """
        Return an open interest update parsed from the given values.

        Parameters
        ----------
        values : dict[str, Any]
            The values for initialization.

        Returns
        -------
        OpenInterestHist

        """
        return OpenInterestHist(
            instrument_id=InstrumentId.from_str(values["instrument_id"]),
            sumOpenInterest=Decimal(values["sumOpenInterest"]),
            sumOpenInterestValue=Decimal(values["sumOpenInterestValue"]),
            ts_event=values["ts_event"],
            ts_init=values["ts_init"],
        )
    @staticmethod
    def to_dict(obj: "OpenInterestHist") -> dict[str, Any]:
        """
        Return a dictionary representation of this object.

        Returns
        -------
        dict[str, Any]

        """
        return {
            "type": type(obj).__name__,
            "instrument_id": str(obj.instrument_id),
            "sumOpenInterest": str(obj.sumOpenInterest),
            "sumOpenInterestValue": str(obj.sumOpenInterestValue),
            "ts_event": obj._ts_event,
            "ts_init": obj._ts_init,
        }


class TopLongShortAccountRatio(Data):
    """
    Represents a top long/short account ratio update.

    Parameters
    ----------
    instrument_id : InstrumentId
        The instrument ID for the update.
    longShortRatio :  Decimal
        The long/short ratio.
    longAccount: Decimal
        The long account.
    shortAccount: Decimal
        The short account.
    ts_event : uint64_t
        The UNIX timestamp (nanoseconds) when the data event occurred.
    ts_init : uint64_t
        The UNIX timestamp (nanoseconds) when the data object was initialized.
    """
    def __init__(
        self,
        instrument_id: InstrumentId,
        longShortRatio: Decimal,
        longAccount: Decimal,
        shortAccount: Decimal,
        ts_event: int,
        ts_init: int,
    ):
        

        self.instrument_id = instrument_id
        self.longShortRatio = longShortRatio
        self.longAccount = longAccount
        self.shortAccount = shortAccount
        self._ts_event = ts_event
        self._ts_init = ts_init
    
    def __repr__(self) -> str:
        return (
            f"{type(self).__name__}("
            f"instrument_id={self.instrument_id}, "
            f"longShortRatio={self.longShortRatio}, "
            f"longAccount={self.longAccount}, "
            f"shortAccount={self.shortAccount}, "
            f"ts_event={self._ts_event}, "
            f"ts_init={self._ts_init})"
        )
    
    @staticmethod
    def from_dict(values: dict[str, Any]) -> "TopLongShortAccountRatio":
        """
        Return a top long/short account ratio update parsed from the given values.

        Parameters
        ----------
        values : dict[str, Any]
            The values for initialization.

        Returns
        -------
        TopLongShortAccountRatio

        """
        return TopLongShortAccountRatio(
            instrument_id=InstrumentId.from_str(values["instrument_id"]),
            longShortRatio=Decimal(values["longShortRatio"]),
            longAccount=Decimal(values["longAccount"]),
            shortAccount=Decimal(values["shortAccount"]),
            ts_event=values["ts_event"],
            ts_init=values["ts_init"],
        )
    
    @staticmethod
    def to_dict(obj: "TopLongShortAccountRatio") -> dict[str, Any]:
        """
        Return a dictionary representation of this object.

        Returns
        -------
        dict[str, Any]

        """
        return {
            "type": type(obj).__name__,
            "instrument_id": str(obj.instrument_id),
            "longShortRatio": str(obj.longShortRatio),
            "longAccount": str(obj.longAccount),
            "shortAccount": str(obj.shortAccount),
            "ts_event": obj._ts_event,
            "ts_init": obj._ts_init,
        }

class TopLongShortPositionRatio(Data):
    """
    Represents a top long/short position ratio update.

    Parameters
    ----------
    instrument_id : InstrumentId
        The instrument ID for the update.
    longShortRatio :  Decimal
        The long/short ratio.
    longAccount: Decimal
        The long account.
    shortAccount: Decimal
        The short account.
    ts_event : uint64_t
        The UNIX timestamp (nanoseconds) when the data event occurred.
    ts_init : uint64_t
        The UNIX timestamp (nanoseconds) when the data object was initialized.
    """
    def __init__(
        self,
        instrument_id: InstrumentId,
        longShortRatio: Decimal,
        longAccount: Decimal,
        shortAccount: Decimal,
        ts_event: int,
        ts_init: int,
    ):
        

        self.instrument_id = instrument_id
        self.longShortRatio = longShortRatio
        self.longAccount = longAccount
        self.shortAccount = shortAccount
        self._ts_event = ts_event
        self._ts_init = ts_init
    
    def __repr__(self) -> str:
        return (
            f"{type(self).__name__}("
            f"instrument_id={self.instrument_id}, "
            f"longShortRatio={self.longShortRatio}, "
            f"longAccount={self.longAccount}, "
            f"shortAccount={self.shortAccount}, "
            f"ts_event={self._ts_event}, "
            f"ts_init={self._ts_init})"
        )
    
    @staticmethod
    def from_dict(values: dict[str, Any]) -> "TopLongShortPositionRatio":
        """
        Return a top long/short position ratio update parsed from the given values.

        Parameters
        ----------
        values : dict[str, Any]
            The values for initialization.

        Returns
        -------
        TopLongShortPositionRatio

        """
        return TopLongShortPositionRatio(
            instrument_id=InstrumentId.from_str(values["instrument_id"]),
            longShortRatio=Decimal(values["longShortRatio"]),
            longAccount=Decimal(values["longAccount"]),
            shortAccount=Decimal(values["shortAccount"]),
            ts_event=values["ts_event"],
            ts_init=values["ts_init"],
        )
    
    @staticmethod
    def to_dict(obj: "TopLongShortPositionRatio") -> dict[str, Any]:
        """
        Return a dictionary representation of this object.

        Returns
        -------
        dict[str, Any]

        """
        return {
            "type": type(obj).__name__,
            "instrument_id": str(obj.instrument_id),
            "longShortRatio": str(obj.longShortRatio),
            "longAccount": str(obj.longAccount),
            "shortAccount": str(obj.shortAccount),
            "ts_event": obj._ts_event,
            "ts_init": obj._ts_init,
        }

class GlobalLongShortAccountRatio(Data):
    """
    Represents a global long/short account ratio update.

    Parameters
    ----------
    instrument_id : InstrumentId
        The instrument ID for the update.
    longShortRatio :  Decimal
        The long/short ratio.
    longAccount: Decimal
        The long account.
    shortAccount: Decimal
        The short account.
    ts_event : uint64_t
        The UNIX timestamp (nanoseconds) when the data event occurred.
    ts_init : uint64_t
        The UNIX timestamp (nanoseconds) when the data object was initialized.
    """
    def __init__(
        self,
        instrument_id: InstrumentId,
        longShortRatio: Decimal,
        longAccount: Decimal,
        shortAccount: Decimal,
        ts_event: int,
        ts_init: int,
    ):
        
        self.instrument_id = instrument_id
        self.longShortRatio = longShortRatio
        self.longAccount = longAccount
        self.shortAccount = shortAccount
        self._ts_event = ts_event
        self._ts_init = ts_init
    
    def __repr__(self) -> str:
        return (
            f"{type(self).__name__}("
            f"instrument_id={self.instrument_id}, "
            f"longShortRatio={self.longShortRatio}, "
            f"longAccount={self.longAccount}, "
            f"shortAccount={self.shortAccount}, "
            f"ts_event={self._ts_event}, "
            f"ts_init={self._ts_init})"
        )
    
    @staticmethod
    def from_dict(values: dict[str, Any]) -> "GlobalLongShortAccountRatio":
        """
        Return a global long/short account ratio update parsed from the given values.

        Parameters
        ----------
        values : dict[str, Any]
            The values for initialization.

        Returns
        -------
        GlobalLongShortAccountRatio

        """
        return GlobalLongShortAccountRatio(
            instrument_id=InstrumentId.from_str(values["instrument_id"]),
            longShortRatio=Decimal(values["longShortRatio"]),
            longAccount=Decimal(values["longAccount"]),
            shortAccount=Decimal(values["shortAccount"]),
            ts_event=values["ts_event"],
            ts_init=values["ts_init"],
        )
    
    @staticmethod
    def to_dict(obj: "GlobalLongShortAccountRatio") -> dict[str, Any]:
        """
        Return a dictionary representation of this object.

        Returns
        -------
        dict[str, Any]

        """
        return {
            "type": type(obj).__name__,
            "instrument_id": str(obj.instrument_id),
            "longShortRatio": str(obj.longShortRatio),
            "longAccount": str(obj.longAccount),
            "shortAccount": str(obj.shortAccount),
            "ts_event": obj._ts_event,
            "ts_init": obj._ts_init,
        }

class TakerLongShortRatio(Data):
    """
    Represents a taker long/short ratio update.

    Parameters
    ----------
    instrument_id : InstrumentId
        The instrument ID for the update.
    buySellRatio : Decimal
        The buy/sell ratio.
    buyVol: Decimal
        The buy volume.
    sellVol: Decimal
        The sell volume.
    ts_event : uint64_t
        The UNIX timestamp (nanoseconds) when the data event occurred.
    ts_init : uint64_t
        The UNIX timestamp (nanoseconds) when the data object was initialized.
    """
    def __init__(
        self,
        instrument_id: InstrumentId,
        buySellRatio: Decimal,
        buyVol: Decimal,
        sellVol: Decimal,
        ts_event: int,
        ts_init: int,
    ):
        

        self.instrument_id = instrument_id
        self.buySellRatio = buySellRatio
        self.buyVol = buyVol
        self.sellVol = sellVol
        self._ts_event = ts_event
        self._ts_init = ts_init
    
    def __repr__(self) -> str:
        return (
            f"{type(self).__name__}("
            f"instrument_id={self.instrument_id}, "
            f"buySellRatio={self.buySellRatio}, "
            f"buyVol={self.buyVol}, "
            f"sellVol={self.sellVol}, "
            f"ts_event={self._ts_event}, "
            f"ts_init={self._ts_init})"
        )
    
    @staticmethod
    def from_dict(values: dict[str, Any]) -> "TakerLongShortRatio":
        """
        Return a taker long/short ratio update parsed from the given values.

        Parameters
        ----------
        values : dict[str, Any]
            The values for initialization.

        Returns
        -------
        TakerLongShortRatio

        """
        return TakerLongShortRatio(
            instrument_id=InstrumentId.from_str(values["instrument_id"]),
            buySellRatio=Decimal(values["buySellRatio"]),
            buyVol=Decimal(values["buyVol"]),
            sellVol=Decimal(values["sellVol"]),
            ts_event=values["ts_event"],
            ts_init=values["ts_init"],
        )
    
    @staticmethod
    def to_dict(obj: "TakerLongShortRatio") -> dict[str, Any]:
        """
        Return a dictionary representation of this object.

        Returns
        -------
        dict[str, Any]

        """
        return {
            "type": type(obj).__name__,
            "instrument_id": str(obj.instrument_id),
            "buySellRatio": str(obj.buySellRatio),
            "buyVol": str(obj.buyVol),
            "sellVol": str(obj.sellVol),
            "ts_event": obj._ts_event,
            "ts_init": obj._ts_init,
        }

    
    