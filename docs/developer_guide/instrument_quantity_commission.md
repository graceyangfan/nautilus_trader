# Instrument quantity commission prototype

This downstream Rust prototype accompanies [RFC #1](https://github.com/graceyangfan/nautilus_trader/issues/1).
It is not an agreed upstream interface or a production-ready fee implementation.

## Contract

`OrderFilled.last_qty` and `FillReport.last_qty` remain gross execution quantities.
An optional `quantity_commission: Decimal` expresses a positive fee in inventory units
of the existing instrument ID. It is mutually exclusive with a nonzero `Money` commission.
Existing constructors default to `None`; old JSON without the field remains readable.
Adding a public field still changes Rust struct literals and ABI.

The execution engine classifies duplicates and price restatements using existing rules
before validating support for a new position change. Order-only reconciliation projections
skip inventory preflight: their fills are already represented by the retained position,
and projecting history must not validate a historical SELL against current holdings.
Validation precedes publication and order mutation, including orderless leg fills; historical duplicates are not checked against the
inventory of a later position episode. Position applies
the gross execution and inventory deduction together, then finalizes quantity and lifecycle.
The engine publishes ordinary order and position events after Cache contains the result.

The deduction reuses the internal commission adjustment used for CurrencyPair base fees.
It is derived from the original fill, not recorded as a second replay input. There is no
BinaryOption-specific branch, adapter position ledger, synthetic SELL or recovery timer.

## Inventory cost and PnL

The existing internal Commission adjustment carries both inventory deduction and
settlement-currency `pnl_change`. For supported long inventory, removing `f` units
writes off `f * avg_px_open` after the gross fill has updated the moving average.
This preserves execution prices and the existing average-price calculation while
including the removed inventory's cost in realized PnL. Future exit fees are not
estimated or booked by Position.

For example, BUY 1 at 100 with an inventory fee of 0.01 leaves 0.99 and realized
PnL of -1. Selling 0.99 at 110 without another fee closes with realized PnL of
8.9, matching cash received minus cash spent. A quote-currency SELL fee follows
the existing Money path. Reported account balances are not charged again.

Legacy CurrencyPair base commissions remain unchanged outside the explicit
quantity-fee path. The same BUY/SELL represented as a legacy base Money fee therefore
retains 9.9 quote PnL, while the explicit inventory fee reports 8.9. This is a deliberate
prototype boundary, not equivalent PnL semantics. Upstream review must decide separately
whether to accept the inventory-fee fact and whether incurred inventory cost should also
change the legacy base-fee convention. Tests establish cash-flow correctness for the new
path; they do not settle that API policy. Fee currency and original inventory quantities remain reported
facts; the cost write-off is not a second commission asset.

## Precision and persistence

Position retains cumulative inventory fees in `quantity_commission_total` and
removed inventory cost in `quantity_commission_cost`. Inventory fees are projected
upward to instrument precision; only the projection change is deducted. Cost uses
the deducted inventory and existing moving average, accumulates in Decimal, and
books the difference between current and previous Money projections. Four costs
of 0.000000004 USDT therefore book 0.00000002, rather than four rounded zeros.
No separately stored projected balance or adapter accounting state is needed.

Canonical projected inventory prevents subsequent zero-fee or quote-fee fills
from reintroducing deducted units through floating-point round trips. Remaining
subquantum inventory is conservatively unavailable, not proof of zero holdings.
This policy does not rewrite Native moving-average pricing or Money rounding.
Upstream `Money::from_decimal_exact` supports sub-currency amounts for reported
balances and venue fees without a custom Currency or Money implementation. Position's
BUY/SELL handlers still reconstruct realized PnL at currency precision. Changing only
the cost conversion to exact Money would therefore lose cost tails on the next fill;
the cumulative cost projection retains that upstream Position convention.

Original fills are the replay inputs; internal Commission adjustments are derived
once. Ordinary Redis/SQL persistence rebuilds both totals from the original fills;
no extra snapshot is forced for inventory fees. Existing complete replay state and
explicit snapshots also retain both values, including Arrow round trips. No
PositionSnapshot summary field or Arrow column is added. Episode reset and purge
rebuild this state.
Quantity-only prototype snapshots without cost must be rebuilt from original fills
before continuation; missing historical cost is rejected before a new fill mutates
order or position state.

## Supported scope and explicit exclusions

- Reported cash accounts, long linear inventory, unit multiplier.
- Positive Decimal inventory fees, including 18-decimal tails.
- Native gross order completion, partial executions, duplicate replay and net lifecycle.
- JSON and Arrow order events retain the new field; upstream no longer has Cap'n Proto support.

Inverse products, shorts/flips, calculated accounts, price restatements and fill voids
remain unsupported by this prototype and are rejected before application. An opening net
balance below one instrument quantity quantum is rejected rather than emitting a zero-size
PositionOpened. These restrictions are implementation boundaries, not the definition of an
inventory fee. Inventory-cost accounting does not establish general account support.

Native quantity remains at instrument precision (16 in the Predict harness). PnL retains
existing moving-average floating-point and Money rounding. PyO3 constructors, getters and fill dictionaries preserve the optional Decimal fee;
Position getters and summary dictionaries expose its cumulative units and cost. Position
summary dictionaries are not full persistence snapshots. Legacy FFI surfaces and correction
semantics remain follow-ups before an upstream-ready implementation. Earlier
unpublished prototype snapshots with `quantity_commission_totals` must be rebuilt from
original fills; automatic migration of that experimental format is unsupported.

## Validation

Model tests cover mixed-price partial executions, currency/base compatibility,
sub-Money cost accumulation, JSON/snapshot restore, duplicate replay, purge and
episode reset. Invalid or overflowing costs leave fill state unchanged. Legacy
base-fee tests retain their original PnL convention.

Engine subscribers observe cost-inclusive PositionOpened and PositionClosed events
without additional synthetic fills or duplicate callbacks. The separate Predict
harness replays recorded economic facts through Rust Strategy + LiveNode, verifies
net inventory and account cash, and independently checks cash-flow PnL. Different-price
and precision stress cases are synthetic and labeled separately from venue evidence.
