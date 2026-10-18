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
Validation precedes order mutation; historical duplicates are not checked against the
inventory of a later position episode. Position applies
the gross execution and inventory deduction together, then finalizes quantity and lifecycle.
The engine publishes ordinary order and position events after Cache contains the result.

The deduction reuses the internal commission adjustment used for CurrencyPair base fees.
It is derived from the original fill, not recorded as a second replay input. There is no
BinaryOption-specific branch, adapter position ledger, synthetic SELL or recovery timer.

## Existing commission and PnL convention

Inventory deduction does not add a new settlement-currency realized PnL expense. Like the
existing CurrencyPair base commission path, its internal Commission adjustment has no
`pnl_change`. Quote-currency commission continues to affect realized PnL through existing
logic. The new field does not replace currency commission or migrate old base-fee events.

For example, BUY 1 at 100 with an inventory fee of 0.01 leaves 0.99 and quote realized
PnL of zero. Selling 0.99 at 110 gives quote realized PnL of 9.9, matching the existing
base-currency commission path. Cash difference is 8.9. The inventory fee remains separately
identifiable; quote realized PnL alone is not an all-in cash return. Consumers must not
interpret 9.9 as a fee-inclusive return or charge the reported cash account again.

This prototype does not introduce inventory-fee valuation, moving-average fee-cost totals,
or a second cost/PnL algorithm.

## Precision and persistence

Position retains one optional Decimal `quantity_commission_total`: actual inventory fees
in the current episode. It does not retain a separate fee-cost field or residual ledger.
Fees are cumulatively projected upward to instrument precision; only the change in that
projection is deducted on each execution. This prevents repeated subprecision fees from
being discarded. Remaining subquantum inventory is conservatively unavailable, not proof
that the venue holds exactly zero.

The canonical projected quantity remains necessary for this limited path: subsequent
zero-fee and quote-fee fills must not reintroduce inventory through floating-point round
trips. Other positions retain existing quantity handling. This is not a universal Native
rounding policy or an 18-decimal rewrite of Position, Portfolio or PnL.

Original fill events retain fee facts. Existing Position snapshot `replay_state`, including
Arrow round trips, retains the single cumulative value. No new Order aggregation method,
PositionSnapshot summary field or Arrow snapshot column is required. Episode reset and
purge/replay rebuild Native state. Test diagnostics derive residuals with Decimal.

## Supported scope and explicit exclusions

- Reported cash accounts, long linear inventory, unit multiplier.
- Positive Decimal inventory fees, including 18-decimal tails.
- Native gross order completion, partial executions, duplicate replay and net lifecycle.
- JSON, Arrow order events and Cap'n Proto order events retain the new field.

Inverse products, shorts/flips, calculated accounts, price restatements and fill voids
remain unsupported by this prototype and are rejected before application. An opening net
balance below one instrument quantity quantum is rejected rather than emitting a zero-size
PositionOpened. These restrictions are implementation boundaries, not the definition of an
inventory fee. Removing fee-cost bookkeeping does not establish general account support.

Native quantity remains at instrument precision (16 in the Predict harness). PnL retains
existing moving-average floating-point and Money rounding. Python/PyO3/FFI access and
correction semantics remain follow-ups before an upstream-ready implementation. Earlier
unpublished prototype snapshots with `quantity_commission_totals` must be rebuilt from
original fills; automatic migration of that experimental format is unsupported.

## Validation

A differential regression compares the new inventory fee with the existing CurrencyPair
base commission path, including net quantity, quote realized PnL and closure. The previous
implementation fails this regression by adding an extra realized PnL expense.

Model tests cover 100 subprecision fees, JSON restore, snapshot state, purge/replay, episode
reset, conflicting fees, unrepresentable net inventory and late historical duplicates.
Engine tests cover duplicate ordering, atomic oversell rejection and subsequent closure.
The separate Predict harness checks report ingestion, immediate net Cache observations,
partial executions, quote-fee SELL and Rust Strategy + LiveNode maker-TP/cancel/taker exits.
Offline JSON distinguishes Native quote realized PnL from cash difference and does not
claim that separately denominated fees have already been valued in the former.
