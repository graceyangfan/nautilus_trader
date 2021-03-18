# -------------------------------------------------------------------------------------------------
#  Copyright (C) 2015-2021 Nautech Systems Pty Ltd. All rights reserved.
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

from nautilus_trader.model.c_enums.order_side cimport OrderSide
from nautilus_trader.model.orderbook.ladder cimport Ladder
from nautilus_trader.model.orderbook.level cimport Level
from nautilus_trader.model.orderbook.order cimport Order
from nautilus_trader.model.orderbook.util import pprint_ob


cdef class OrderBook:
    """
    Provides a L1/L2/L3 order book.

    A L3 order book that can be proxied to L2 or L1 `OrderBook` classes.
    """

    def __init__(self):
        """
        Initialize a new instance of the `OrderBook` class.

        """
        self.bids = Ladder(reverse=True)
        self.asks = Ladder(reverse=False)

    cpdef void add(self, Order order) except *:
        """
        Add the given order to the book.

        Parameters
        ----------
        order : Order
            The order to add.

        """
        self._add(order=order)

    cpdef void update(self, Order order) except *:
        """
        Update the given order in the book.

        Parameters
        ----------
        order : Order
            The order to update.

        """
        self._update(order=order)

    cpdef void delete(self, Order order) except *:
        """
        Delete the given order in the book.

        Parameters
        ----------
        order : Order
            The order to delete.

        """
        self._delete(order=order)

    cpdef bint check_integrity(self, bint deep=True) except *:
        """
        Return a value indicating whether the order book integrity test passes.

        Returns
        -------
        bool
            True if check passes, else False.

        """
        return self._check_integrity(deep=deep)

    cpdef void clear_bids(self) except *:
        """
        Clear the bids from the book.
        """
        self.bids = Ladder(reverse=True)

    cpdef void clear_asks(self) except *:
        """
        Clear the asks from the book.
        """
        self.asks = Ladder(reverse=False)

    cpdef void clear(self) except *:
        """
        Clear the entire orderbook.
        """
        self.clear_bids()
        self.clear_asks()

    cdef inline void _add(self, Order order) except *:
        """
        Add the given order to the book.

        Parameters
        ----------
        order : Order
            The order to add.

        """
        if order.side == OrderSide.BUY:
            self.bids.add(order=order)
        elif order.side == OrderSide.SELL:
            self.asks.add(order=order)

    cdef inline void _update(self, Order order) except *:
        """
        Update the given order in the book.

        Parameters
        ----------
        order : Order
            The order to update.

        """
        if order.side == OrderSide.BUY:
            self.bids.update(order=order)
        elif order.side == OrderSide.SELL:
            self.asks.update(order=order)

    cdef inline void _delete(self, Order order) except *:
        """
        Delete the given order in the book.

        Parameters
        ----------
        order : Order
            The order to delete.

        """
        if order.side == OrderSide.BUY:
            self.bids.delete(order=order)
        elif order.side == OrderSide.SELL:
            self.asks.delete(order=order)

    cdef inline bint _check_integrity(self, bint deep=True) except *:
        """
        Return a value indicating whether the order book integrity test passes.

        Returns
        -------
        bool
            True if check passes, else False.

        """
        cdef Level top_bid_level = self.bids.top()
        cdef Level top_ask_level = self.asks.top()
        if top_bid_level is None or top_ask_level is None:
            return True
        if not top_bid_level.price() < top_ask_level.price():
            # TODO: logging.warning("Price in cross")
            return False
        if deep:
            if not [lvl.price() for lvl in self.bids.price_levels] == sorted(
                    [lvl.price() for lvl in self.bids.price_levels]
            ):
                return False
            if not [lvl.price() for lvl in self.asks.price_levels] == sorted(
                    [lvl.price() for lvl in self.asks.price_levels], reverse=True
            ):
                return False
        return True

    cpdef Level best_bid(self):
        """
        Return the best bid level.

        Returns
        -------
        Level

        """
        return self.bids.top()

    cpdef Level best_ask(self):
        """
        Return the best ask level.

        Returns
        -------
        Level

        """
        return self.asks.top()

    cpdef double spread(self) except *:
        """
        Return the top of book spread (if no bids or asks then returns zero).

        Returns
        -------
        double

        """
        cdef Level top_bid_level = self.bids.top()
        cdef Level top_ask_level = self.asks.top()
        if top_bid_level and top_ask_level:
            return top_ask_level.price() - top_bid_level.price()
        else:
            # TODO: What is the correct behaviour here?
            return 0

    cpdef double best_bid_price(self) except *:
        """
        Return the best bid price in the book (if no bids then returns zero).

        Returns
        -------
        double

        """
        cdef Level top_bid_level = self.bids.top()
        if top_bid_level:
            return top_bid_level.price()
        else:
            # TODO: What is the correct behaviour here?
            return 0

    cpdef double best_ask_price(self) except *:
        """
        Return the best ask price in the book (if no asks then returns zero).

        Returns
        -------
        double

        """
        cdef Level top_ask_level = self.asks.top()
        if top_ask_level:
            return top_ask_level.price()
        else:
            # TODO: What is the correct behaviour here?
            return 0

    cpdef double best_bid_qty(self) except *:
        """
        Return the best bid quantity in the book (if no bids then returns zero).

        Returns
        -------
        double

        """
        cdef Level top_bid_level = self.bids.top()
        if top_bid_level:
            return top_bid_level.volume()
        else:
            # TODO: What is the correct behaviour here?
            return 0

    cpdef double best_ask_qty(self) except *:
        """
        Return the best ask quantity in the book (if no asks then returns zero).

        Returns
        -------
        double

        """
        cdef Level top_ask_level = self.asks.top()
        if top_ask_level:
            return top_ask_level.volume()
        else:
            # TODO: What is the correct behaviour here?
            return 0

    def __repr__(self):
        return pprint_ob(self)


cdef class L3OrderBook(OrderBook):
    """
    A L3 OrderBook.

    Should map directly to functionality of the OrderBook base class.
    """

    def __init__(self):
        super().__init__()


cdef class L2OrderBook(OrderBook):
    """ A L2 Orderbook. An Orderbook where price `Levels` are only made up of a single order """

    def __init__(self):
        super().__init__()

    cpdef void add(self, Order order) except *:
        """
        Add the given order to the book.

        Parameters
        ----------
        order : Order
            The order to add.

        """
        self._process_order(order=order)
        self._add(order=order)

    cpdef void update(self, Order order) except *:
        """
        Update the given order in the book.

        Parameters
        ----------
        order : Order
            The order to update.

        """
        self._process_order(order=order)
        self._remove_if_exists(order)
        self._update(order=order)

    cpdef void delete(self, Order order) except *:
        """
        Delete the given order in the book.

        Parameters
        ----------
        order : Order
            The order to delete.

        """
        self._process_order(order=order)
        self._delete(order=order)

    cpdef bint check_integrity(self, bint deep=True) except *:
        """
        Return a value indicating whether the order book integrity test passes.

        Returns
        -------
        bool
            True if check passes, else False.

        """
        # For L2 Orderbook, ensure only one order per level in addition to
        # normal orderbook checks.
        if not self._check_integrity(deep=deep):
            return False
        for level in self.bids.levels + self.asks.levels:
            assert len(level.orders) == 1
        return True

    cdef inline Order _process_order(self, Order order):
        # Because L2 Orderbook only has one order per level, we replace the
        # order.id with a price level, which will let us easily process the
        # order in the proxy orderbook.
        order.id = str(order.price)
        return order

    cdef inline void _remove_if_exists(self, Order order) except *:
        # For a L2 orderbook, an order update means a whole level update. If
        # this level exists, remove it so we can insert the new level.

        if order.side == OrderSide.BUY and order.price in self.bids.prices():
            self.delete(order)
        elif order.side == OrderSide.SELL and order.price in self.asks.prices():
            self.delete(order)


cdef class L1OrderBook(OrderBook):
    """ A L1 Orderbook that has only has a single (top) level """

    def __init__(self):
        super().__init__()

    cpdef void add(self, Order order) except *:
        """
        NotImplemented (Use `update(order)` for L1Orderbook).
        """
        raise NotImplementedError("Use `update(order)` for L1Orderbook")

    cpdef void update(self, Order order) except *:
        """
        Update the given order in the book.

        Parameters
        ----------
        order : Order
            The order to update.

        """
        # Because of the way we typically get updates from a L1 orderbook (bid
        # and ask updates at the same time), its quite probable that the last
        # bid is now the ask price we are trying to insert (or vice versa). We
        # just need to add some extra protection against this if we are calling
        # `check_integrity` on each individual update .
        if (
            order.side == OrderSide.BUY
            and self.best_ask()
            and order.price >= self.best_ask_price()
        ):
            self.clear_asks()
        elif (
            order.side == OrderSide.SELL
            and self.best_bid()
            and order.price <= self.best_bid_price()
        ):
            self.clear_bids()
        self._update(order=self._process_order(order=order))

    cpdef void delete(self, Order order) except *:
        """
        Delete the given order in the book.

        Parameters
        ----------
        order : Order
            The order to delete.

        """
        self._delete(order=self._process_order(order=order))

    cpdef bint check_integrity(self, bint deep=True) except *:
        """
        Return a value indicating whether the order book integrity test passes.

        Returns
        -------
        bool
            True if check passes, else False.

        """
        # For L1 Orderbook, ensure only one level per side in addition to normal
        # orderbook checks.
        if not self._check_integrity(deep=deep):
            return False
        assert len(self.bids.levels) <= 1
        assert len(self.asks.levels) <= 1
        return True

    cdef inline Order _process_order(self, Order order):
        # Because L1 Orderbook only has one level per side, we replace the
        # order.id with the name of the side, which will let us easily process
        # the order in the proxy orderbook.
        order.id = str(order.side)
        return order
