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

import asyncio
from typing import Any, Optional

import msgspec
import pandas as pd

from nautilus_trader.adapters.binance.common.constants import BINANCE_VENUE
from nautilus_trader.adapters.binance.common.enums import BinanceAccountType
from nautilus_trader.adapters.binance.common.functions import parse_symbol
from nautilus_trader.adapters.binance.common.parsing.data import parse_bar_http
from nautilus_trader.adapters.binance.common.parsing.data import parse_bar_ws
from nautilus_trader.adapters.binance.common.parsing.data import parse_diff_depth_stream_ws
from nautilus_trader.adapters.binance.common.parsing.data import parse_quote_tick_ws
from nautilus_trader.adapters.binance.common.parsing.data import parse_ticker_24hr_ws
from nautilus_trader.adapters.binance.common.parsing.data import parse_trade_tick_http
from nautilus_trader.adapters.binance.common.schemas import BinanceCandlestickMsg
from nautilus_trader.adapters.binance.common.schemas import BinanceDataMsgWrapper
from nautilus_trader.adapters.binance.common.schemas import BinanceOrderBookMsg
from nautilus_trader.adapters.binance.common.schemas import BinanceQuoteMsg
from nautilus_trader.adapters.binance.common.schemas import BinanceTickerMsg
from nautilus_trader.adapters.binance.common.schemas import BinanceTrade
from nautilus_trader.adapters.binance.common.types import BinanceBar
from nautilus_trader.adapters.binance.common.types import BinanceTicker
from nautilus_trader.adapters.binance.futures.http.market import BinanceFuturesMarketHttpAPI
from nautilus_trader.adapters.binance.futures.http.user import BinanceFuturesUserDataHttpAPI
from nautilus_trader.adapters.binance.futures.parsing.data import parse_futures_book_snapshot
from nautilus_trader.adapters.binance.futures.parsing.data import parse_futures_mark_price_ws
from nautilus_trader.adapters.binance.futures.parsing.data import parse_futures_trade_tick_ws
from nautilus_trader.adapters.binance.futures.schemas.market import BinanceFuturesMarkPriceMsg
from nautilus_trader.adapters.binance.futures.schemas.market import BinanceFuturesTradeMsg
from nautilus_trader.adapters.binance.futures.types import BinanceFuturesMarkPriceUpdate
from nautilus_trader.adapters.binance.http.client import BinanceHttpClient
from nautilus_trader.adapters.binance.websocket.client import BinanceWebSocketClient
from nautilus_trader.cache.cache import Cache
from nautilus_trader.common.clock import LiveClock
from nautilus_trader.common.enums import LogColor
from nautilus_trader.common.logging import Logger
from nautilus_trader.common.providers import InstrumentProvider
from nautilus_trader.core.asynchronous import sleep0
from nautilus_trader.core.correctness import PyCondition
from nautilus_trader.core.datetime import secs_to_millis
from nautilus_trader.core.uuid import UUID4
from nautilus_trader.live.data_client import LiveMarketDataClient
from nautilus_trader.model.data.bar import BarType
from nautilus_trader.model.data.base import DataType
from nautilus_trader.model.data.base import GenericData
from nautilus_trader.model.data.tick import QuoteTick
from nautilus_trader.model.data.tick import TradeTick
from nautilus_trader.model.enums import BarAggregation
from nautilus_trader.model.enums import BookType
from nautilus_trader.model.enums import PriceType
from nautilus_trader.model.enums import bar_aggregation_to_str
from nautilus_trader.model.identifiers import ClientId
from nautilus_trader.model.identifiers import InstrumentId
from nautilus_trader.model.identifiers import Symbol
from nautilus_trader.model.instruments.base import Instrument
from nautilus_trader.model.orderbook.data import OrderBookData
from nautilus_trader.model.orderbook.data import OrderBookDeltas
from nautilus_trader.model.orderbook.data import OrderBookSnapshot
from nautilus_trader.msgbus.bus import MessageBus


class BinanceFuturesDataClient(LiveMarketDataClient):
    """
    Provides a data client for the `Binance Futures` exchange.

    Parameters
    ----------
    loop : asyncio.AbstractEventLoop
        The event loop for the client.
    client : BinanceHttpClient
        The binance HTTP client.
    msgbus : MessageBus
        The message bus for the client.
    cache : Cache
        The cache for the client.
    clock : LiveClock
        The clock for the client.
    logger : Logger
        The logger for the client.
    instrument_provider : InstrumentProvider
        The instrument provider.
    account_type : BinanceAccountType
        The account type for the client.
    base_url_ws : str, optional
        The base URL for the WebSocket client.
    """

    def __init__(
        self,
        loop: asyncio.AbstractEventLoop,
        client: BinanceHttpClient,
        msgbus: MessageBus,
        cache: Cache,
        clock: LiveClock,
        logger: Logger,
        instrument_provider: InstrumentProvider,
        account_type: BinanceAccountType = BinanceAccountType.FUTURES_USDT,
        base_url_ws: Optional[str] = None,
    ):
        super().__init__(
            loop=loop,
            client_id=ClientId(BINANCE_VENUE.value),
            venue=BINANCE_VENUE,
            instrument_provider=instrument_provider,
            msgbus=msgbus,
            cache=cache,
            clock=clock,
            logger=logger,
        )

        assert account_type.is_futures, "account type is not for futures"
        self._binance_account_type = account_type
        self._log.info(f"Account type: {self._binance_account_type.value}.", LogColor.BLUE)

        self._update_instrument_interval: int = 60 * 60  # Once per hour (hardcode)
        self._update_instruments_task: Optional[asyncio.Task] = None

        # HTTP API
        self._http_client = client
        self._http_market = BinanceFuturesMarketHttpAPI(client=client, account_type=account_type)
        self._http_user = BinanceFuturesUserDataHttpAPI(client=client, account_type=account_type)

        # Listen keys
        self._ping_listen_keys_interval: int = 60 * 5  # Once every 5 mins (hardcode)
        self._ping_listen_keys_task: Optional[asyncio.Task] = None
        self._listen_key: Optional[str] = None

        # WebSocket API
        self._ws_client = BinanceWebSocketClient(
            loop=loop,
            clock=clock,
            logger=logger,
            handler=self._handle_ws_message,
            base_url=base_url_ws,
        )

        # Hot caches
        self._instrument_ids: dict[str, InstrumentId] = {}
        self._book_buffer: dict[InstrumentId, list[OrderBookData]] = {}

        self._log.info(f"Base URL HTTP {self._http_client.base_url}.", LogColor.BLUE)
        self._log.info(f"Base URL WebSocket {base_url_ws}.", LogColor.BLUE)

    async def _connect(self) -> None:
        # Connect HTTP client
        if not self._http_client.connected:
            await self._http_client.connect()

        await self._instrument_provider.initialize()

        self._send_all_instruments_to_data_engine()
        self._update_instruments_task = self._loop.create_task(self._update_instruments())

        # Connect WebSocket clients
        self._loop.create_task(self._connect_websockets())

    async def _connect_websockets(self) -> None:
        self._log.info("Awaiting subscriptions...")
        await asyncio.sleep(4)
        if self._ws_client.has_subscriptions:
            await self._ws_client.connect()

    async def _update_instruments(self) -> None:
        while True:
            self._log.debug(
                f"Scheduled `update_instruments` to run in "
                f"{self._update_instruments_interval}s.",
            )
            await asyncio.sleep(self._update_instruments_interval)
            await self._instrument_provider.load_all_async()
            self._send_all_instruments_to_data_engine()

    async def _disconnect(self) -> None:
        # Cancel tasks
        if self._update_instruments_task:
            self._log.debug("Canceling `update_instruments` task...")
            self._update_instruments_task.cancel()

        if self._ping_listen_keys_task:
            self._log.debug("Canceling `ping_listen_keys` task...")
            self._ping_listen_keys_task.cancel()

        # Disconnect WebSocket client
        if self._ws_client.is_connected:
            await self._ws_client.disconnect()

        # Disconnect HTTP client
        if self._http_client.connected:
            await self._http_client.disconnect()

    # -- SUBSCRIPTIONS ----------------------------------------------------------------------------

    def subscribe(self, data_type: DataType) -> None:
        if data_type.type == BinanceFuturesMarkPriceUpdate:
            if not self._binance_account_type.is_futures:
                self._log.error(
                    f"Cannot subscribe to `BinanceFuturesMarkPriceUpdate` "
                    f"for {self._binance_account_type.value} account types.",
                )
                return
            instrument_id: Optional[InstrumentId] = data_type.metadata.get("instrument_id")
            if instrument_id is None:
                self._log.error(
                    "Cannot subscribe to `BinanceFuturesMarkPriceUpdate` "
                    "no instrument ID in `data_type` metadata.",
                )
                return
            self._ws_client.subscribe_mark_price(instrument_id.symbol.value, speed=1000)
            self._add_subscription(data_type)
        else:
            self._log.error(
                f"Cannot subscribe to {data_type.type} (not implemented).",
            )

    def subscribe_instruments(self) -> None:
        for instrument_id in list(self._instrument_provider.get_all().keys()):
            self._add_subscription_instrument(instrument_id)

    def subscribe_instrument(self, instrument_id: InstrumentId) -> None:
        self._add_subscription_instrument(instrument_id)

    def subscribe_order_book_deltas(
        self,
        instrument_id: InstrumentId,
        book_type: BookType,
        depth: Optional[int] = None,
        kwargs: Optional[dict] = None,
    ) -> None:
        self._loop.create_task(
            self._subscribe_order_book(
                instrument_id=instrument_id,
                book_type=book_type,
                depth=depth,
            ),
        )

        self._add_subscription_order_book_deltas(instrument_id)

    def subscribe_order_book_snapshots(
        self,
        instrument_id: InstrumentId,
        book_type: BookType,
        depth: Optional[int] = None,
        kwargs: Optional[dict] = None,
    ) -> None:
        self._loop.create_task(
            self._subscribe_order_book(
                instrument_id=instrument_id,
                book_type=book_type,
                depth=depth,
            ),
        )

        self._add_subscription_order_book_snapshots(instrument_id)

    async def _subscribe_order_book(
        self,
        instrument_id: InstrumentId,
        book_type: BookType,
        depth: Optional[int] = None,
    ) -> None:
        if book_type == BookType.L3_MBO:
            self._log.error(
                "Cannot subscribe to order book deltas: "
                "L3_MBO data is not published by Binance. "
                "Valid book types are L1_TBBO, L2_MBP.",
            )
            return

        if depth is None or depth == 0:
            depth = 20

        # Add delta stream buffer
        self._book_buffer[instrument_id] = []

        if 0 < depth <= 20:
            if depth not in (5, 10, 20):
                self._log.error(
                    "Cannot subscribe to order book snapshots: "
                    f"invalid `depth`, was {depth}. "
                    "Valid depths are 5, 10 or 20.",
                )
                return
            self._ws_client.subscribe_partial_book_depth(
                symbol=instrument_id.symbol.value,
                depth=depth,
                speed=0,
            )
        else:
            self._ws_client.subscribe_diff_book_depth(
                symbol=instrument_id.symbol.value,
                speed=0,
            )

        while not self._ws_client.is_connected:
            await sleep0()

        data: dict[str, Any] = await self._http_market.depth(
            symbol=instrument_id.symbol.value,
            limit=depth,
        )

        ts_event: int = self._clock.timestamp_ns()
        last_update_id: int = data.get("lastUpdateId", 0)

        snapshot = OrderBookSnapshot(
            instrument_id=instrument_id,
            book_type=BookType.L2_MBP,
            bids=[[float(o[0]), float(o[1])] for o in data["bids"]],
            asks=[[float(o[0]), float(o[1])] for o in data["asks"]],
            ts_event=ts_event,
            ts_init=ts_event,
            update_id=last_update_id,
        )

        self._handle_data(snapshot)

        book_buffer = self._book_buffer.pop(instrument_id)
        for deltas in book_buffer:
            if deltas.update_id <= last_update_id:
                continue
            self._handle_data(deltas)

    def subscribe_ticker(self, instrument_id: InstrumentId) -> None:
        self._ws_client.subscribe_ticker(instrument_id.symbol.value)
        self._add_subscription_ticker(instrument_id)

    def subscribe_quote_ticks(self, instrument_id: InstrumentId) -> None:
        self._ws_client.subscribe_book_ticker(instrument_id.symbol.value)
        self._add_subscription_quote_ticks(instrument_id)

    def subscribe_trade_ticks(self, instrument_id: InstrumentId) -> None:
        self._ws_client.subscribe_trades(instrument_id.symbol.value)
        self._add_subscription_trade_ticks(instrument_id)

    def subscribe_bars(self, bar_type: BarType) -> None:
        PyCondition.true(bar_type.is_externally_aggregated(), "aggregation_source is not EXTERNAL")

        if not bar_type.spec.is_time_aggregated():
            self._log.error(
                f"Cannot subscribe to {bar_type}: only time bars are aggregated by Binance.",
            )
            return

        if bar_type.spec.aggregation in (BarAggregation.MILLISECOND, BarAggregation.SECOND):
            self._log.error(
                f"Cannot subscribe to {bar_type}: "
                f"{bar_aggregation_to_str(bar_type.spec.aggregation)} "
                f"bars are not aggregated by Binance.",
            )
            return

        if bar_type.spec.aggregation == BarAggregation.MINUTE:
            resolution = "m"
        elif bar_type.spec.aggregation == BarAggregation.HOUR:
            resolution = "h"
        elif bar_type.spec.aggregation == BarAggregation.DAY:
            resolution = "d"
        else:
            raise RuntimeError(  # pragma: no cover (design-time error)
                f"invalid `BarAggregation`, "  # pragma: no cover
                f"was {bar_aggregation_to_str(bar_type.spec.aggregation)}",  # pragma: no cover
            )

        self._ws_client.subscribe_bars(
            symbol=bar_type.instrument_id.symbol.value,
            interval=f"{bar_type.spec.step}{resolution}",
        )
        self._add_subscription_bars(bar_type)

    def unsubscribe(self, data_type: DataType) -> None:
        if data_type.type == BinanceFuturesMarkPriceUpdate:
            if not self._binance_account_type.is_futures:
                self._log.error(
                    "Cannot unsubscribe from `BinanceFuturesMarkPriceUpdate` "
                    f"for {self._binance_account_type.value} account types.",
                )
                return
            instrument_id: Optional[InstrumentId] = data_type.metadata.get("instrument_id")
            if instrument_id is None:
                self._log.error(
                    "Cannot subscribe to `BinanceFuturesMarkPriceUpdate` no instrument ID in `data_type` metadata.",
                )
                return
            self._remove_subscription(data_type)
        else:
            self._log.error(
                f"Cannot unsubscribe from {data_type.type} (not implemented).",
            )

    def unsubscribe_instruments(self) -> None:
        for instrument_id in list(self._instrument_provider.get_all().keys()):
            self._remove_subscription_instrument(instrument_id)

    def unsubscribe_instrument(self, instrument_id: InstrumentId) -> None:
        self._remove_subscription_instrument(instrument_id)

    def unsubscribe_order_book_deltas(self, instrument_id: InstrumentId) -> None:
        self._remove_subscription_order_book_deltas(instrument_id)

    def unsubscribe_order_book_snapshots(self, instrument_id: InstrumentId) -> None:
        self._remove_subscription_order_book_snapshots(instrument_id)

    def unsubscribe_ticker(self, instrument_id: InstrumentId) -> None:
        self._remove_subscription_ticker(instrument_id)

    def unsubscribe_quote_ticks(self, instrument_id: InstrumentId) -> None:
        self._remove_subscription_quote_ticks(instrument_id)

    def unsubscribe_trade_ticks(self, instrument_id: InstrumentId) -> None:
        self._remove_subscription_trade_ticks(instrument_id)

    def unsubscribe_bars(self, bar_type: BarType) -> None:
        self._remove_subscription_bars(bar_type)

    def unsubscribe_instrument_status_updates(self, instrument_id: InstrumentId) -> None:
        self._remove_subscription_instrument_status_updates(instrument_id)

    def unsubscribe_instrument_close_prices(self, instrument_id: InstrumentId) -> None:
        self._remove_subscription_instrument_close_prices(instrument_id)

    # -- REQUESTS ---------------------------------------------------------------------------------

    def request_instrument(self, instrument_id: InstrumentId, correlation_id: UUID4) -> None:
        instrument: Optional[Instrument] = self._instrument_provider.find(instrument_id)
        if instrument is None:
            self._log.error(f"Cannot find instrument for {instrument_id}.")
            return

        data_type = DataType(
            type=Instrument,
            metadata={"instrument_id": instrument_id},
        )

        self._handle_data_response(
            data_type=data_type,
            data=[instrument],  # Data engine handles lists of instruments
            correlation_id=correlation_id,
        )

    def request_quote_ticks(
        self,
        instrument_id: InstrumentId,
        limit: int,
        correlation_id: UUID4,
        from_datetime: Optional[pd.Timestamp] = None,
        to_datetime: Optional[pd.Timestamp] = None,
    ) -> None:
        self._log.error(
            "Cannot request historical quote ticks: not published by Binance.",
        )

    def request_trade_ticks(
        self,
        instrument_id: InstrumentId,
        limit: int,
        correlation_id: UUID4,
        from_datetime: Optional[pd.Timestamp] = None,
        to_datetime: Optional[pd.Timestamp] = None,
    ) -> None:
        if limit == 0 or limit > 1000:
            limit = 1000

        if from_datetime is not None or to_datetime is not None:
            self._log.warning(
                "Trade ticks have been requested with a from/to time range, "
                f"however the request will be for the most recent {limit}.",
            )

        self._loop.create_task(self._request_trade_ticks(instrument_id, limit, correlation_id))

    async def _request_trade_ticks(
        self,
        instrument_id: InstrumentId,
        limit: int,
        correlation_id: UUID4,
    ) -> None:
        response: list[BinanceTrade] = await self._http_market.trades(
            instrument_id.symbol.value,
            limit,
        )

        ticks: list[TradeTick] = [
            parse_trade_tick_http(
                trade=trade,
                instrument_id=instrument_id,
                ts_init=self._clock.timestamp_ns(),
            )
            for trade in response
        ]

        self._handle_trade_ticks(instrument_id, ticks, correlation_id)

    def request_bars(
        self,
        bar_type: BarType,
        limit: int,
        correlation_id: UUID4,
        from_datetime: Optional[pd.Timestamp] = None,
        to_datetime: Optional[pd.Timestamp] = None,
    ) -> None:
        if bar_type.is_internally_aggregated():
            self._log.error(
                f"Cannot request {bar_type}: "
                f"only historical bars with EXTERNAL aggregation available from Binance.",
            )
            return

        if not bar_type.spec.is_time_aggregated():
            self._log.error(
                f"Cannot request {bar_type}: only time bars are aggregated by Binance.",
            )
            return

        if bar_type.spec.aggregation in (BarAggregation.MILLISECOND, BarAggregation.SECOND):
            self._log.error(
                f"Cannot request {bar_type}: "
                f"{bar_aggregation_to_str(bar_type.spec.aggregation)} "
                f"bars are not aggregated by Binance.",
            )
            return

        if bar_type.spec.price_type != PriceType.LAST:
            self._log.error(
                f"Cannot request {bar_type}: "
                f"only historical bars for LAST price type available from Binance.",
            )
            return

        self._loop.create_task(
            self._request_bars(
                bar_type=bar_type,
                limit=limit,
                correlation_id=correlation_id,
                from_datetime=from_datetime,
                to_datetime=to_datetime,
            ),
        )

    async def _request_bars(
        self,
        bar_type: BarType,
        limit: int,
        correlation_id: UUID4,
        from_datetime: Optional[pd.Timestamp],
        to_datetime: Optional[pd.Timestamp],
    ) -> None:
        if limit == 0 or limit > 1000:
            limit = 1000

        if bar_type.spec.aggregation == BarAggregation.MINUTE:
            resolution = "m"
        elif bar_type.spec.aggregation == BarAggregation.HOUR:
            resolution = "h"
        elif bar_type.spec.aggregation == BarAggregation.DAY:
            resolution = "d"
        else:
            raise RuntimeError(  # pragma: no cover (design-time error)
                f"invalid `BarAggregation`, "  # pragma: no cover
                f"was {bar_aggregation_to_str(bar_type.spec.aggregation)}",  # pragma: no cover
            )

        start_time_ms = None
        if from_datetime is not None:
            start_time_ms = secs_to_millis(from_datetime.timestamp())

        end_time_ms = None
        if to_datetime is not None:
            end_time_ms = secs_to_millis(to_datetime.timestamp())

        data: list[list[Any]] = await self._http_market.klines(
            symbol=bar_type.instrument_id.symbol.value,
            interval=f"{bar_type.spec.step}{resolution}",
            start_time_ms=start_time_ms,
            end_time_ms=end_time_ms,
            limit=limit,
        )

        bars: list[BinanceBar] = [
            parse_bar_http(
                bar_type,
                values=b,
                ts_init=self._clock.timestamp_ns(),
            )
            for b in data
        ]
        partial: BinanceBar = bars.pop()

        self._handle_bars(bar_type, bars, partial, correlation_id)

    def _send_all_instruments_to_data_engine(self) -> None:
        for instrument in self._instrument_provider.get_all().values():
            self._handle_data(instrument)

        for currency in self._instrument_provider.currencies().values():
            self._cache.add_currency(currency)

    def _get_cached_instrument_id(self, symbol: str) -> InstrumentId:
        # Parse instrument ID
        nautilus_symbol: str = parse_symbol(symbol, account_type=self._binance_account_type)
        instrument_id: Optional[InstrumentId] = self._instrument_ids.get(nautilus_symbol)
        if not instrument_id:
            instrument_id = InstrumentId(Symbol(nautilus_symbol), BINANCE_VENUE)
            self._instrument_ids[nautilus_symbol] = instrument_id
        return instrument_id

    def _handle_ws_message(self, raw: bytes) -> None:
        # TODO(cs): Uncomment for development
        # self._log.info(str(raw), LogColor.CYAN)

        wrapper = msgspec.json.decode(raw, type=BinanceDataMsgWrapper)

        try:
            if "@depth@" in wrapper.stream:
                self._handle_book_diff_update(raw)
            elif "@depth" in wrapper.stream:
                self._handle_book_update(raw)
            elif "@bookTicker" in wrapper.stream:
                self._handle_book_ticker(raw)
            elif "@trade" in wrapper.stream:
                self._handle_trade(raw)
            elif "@ticker" in wrapper.stream:
                self._handle_ticker(raw)
            elif "@kline" in wrapper.stream:
                self._handle_kline(raw)
            elif "@markPrice" in wrapper.stream:
                self._handle_mark_price(raw)
            else:
                self._log.error(
                    f"Unrecognized websocket message type " f"{msgspec.json.decode(raw)['stream']}",
                )
        except (TypeError, ValueError) as e:
            self._log.error(f"Error handling websocket message, {e}")

    def _handle_book_diff_update(self, raw: bytes) -> None:
        msg: BinanceOrderBookMsg = msgspec.json.decode(raw, type=BinanceOrderBookMsg)
        instrument_id: InstrumentId = self._get_cached_instrument_id(msg.data.s)
        book_deltas: OrderBookDeltas = parse_diff_depth_stream_ws(
            instrument_id=instrument_id,
            data=msg.data,
            ts_init=self._clock.timestamp_ns(),
        )
        book_buffer: Optional[list[OrderBookData]] = self._book_buffer.get(instrument_id)
        if book_buffer is not None:
            book_buffer.append(book_deltas)
        else:
            self._handle_data(book_deltas)

    def _handle_book_update(self, raw: bytes) -> None:
        msg: BinanceOrderBookMsg = msgspec.json.decode(raw, type=BinanceOrderBookMsg)
        instrument_id: InstrumentId = self._get_cached_instrument_id(msg.data.s)
        book_snapshot: OrderBookSnapshot = parse_futures_book_snapshot(
            instrument_id=instrument_id,
            data=msg.data,
            ts_init=self._clock.timestamp_ns(),
        )

        # Check if book buffer active
        book_buffer: Optional[list[OrderBookData]] = self._book_buffer.get(instrument_id)
        if book_buffer is not None:
            book_buffer.append(book_snapshot)
        else:
            self._handle_data(book_snapshot)

    def _handle_book_ticker(self, raw: bytes) -> None:
        msg: BinanceQuoteMsg = msgspec.json.decode(raw, type=BinanceQuoteMsg)
        instrument_id: InstrumentId = self._get_cached_instrument_id(msg.data.s)
        quote_tick: QuoteTick = parse_quote_tick_ws(
            instrument_id=instrument_id,
            data=msg.data,
            ts_init=self._clock.timestamp_ns(),
        )
        self._handle_data(quote_tick)

    def _handle_trade(self, raw: bytes) -> None:
        msg: BinanceFuturesTradeMsg = msgspec.json.decode(raw, type=BinanceFuturesTradeMsg)
        instrument_id: InstrumentId = self._get_cached_instrument_id(msg.data.s)
        trade_tick: TradeTick = parse_futures_trade_tick_ws(
            instrument_id=instrument_id,
            data=msg.data,
            ts_init=self._clock.timestamp_ns(),
        )
        self._handle_data(trade_tick)

    def _handle_ticker(self, raw: bytes) -> None:
        msg: BinanceTickerMsg = msgspec.json.decode(raw, type=BinanceTickerMsg)
        instrument_id: InstrumentId = self._get_cached_instrument_id(msg.data.s)
        ticker: BinanceTicker = parse_ticker_24hr_ws(
            instrument_id=instrument_id,
            data=msg.data,
            ts_init=self._clock.timestamp_ns(),
        )
        self._handle_data(ticker)

    def _handle_kline(self, raw: bytes) -> None:
        msg: BinanceCandlestickMsg = msgspec.json.decode(raw, type=BinanceCandlestickMsg)
        if not msg.data.k.x:
            return  # Not closed yet

        instrument_id: InstrumentId = self._get_cached_instrument_id(msg.data.s)
        bar: BinanceBar = parse_bar_ws(
            instrument_id=instrument_id,
            data=msg.data.k,
            ts_init=self._clock.timestamp_ns(),
        )
        self._handle_data(bar)

    def _handle_mark_price(self, raw: bytes) -> None:
        msg: BinanceFuturesMarkPriceMsg = msgspec.json.decode(raw, type=BinanceFuturesMarkPriceMsg)
        instrument_id: InstrumentId = self._get_cached_instrument_id(msg.data.s)
        data: BinanceFuturesMarkPriceUpdate = parse_futures_mark_price_ws(
            instrument_id=instrument_id,
            data=msg.data,
            ts_init=self._clock.timestamp_ns(),
        )
        data_type = DataType(
            BinanceFuturesMarkPriceUpdate,
            metadata={"instrument_id": instrument_id},
        )
        generic = GenericData(data_type=data_type, data=data)
        self._handle_data(generic)
