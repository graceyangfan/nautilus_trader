# Instrument quantity commission prototype

This downstream Rust prototype accompanies [RFC #1](https://github.com/graceyangfan/nautilus_trader/issues/1).
It is not an agreed upstream interface or a production-ready fee implementation.

## Contract

`OrderFilled.last_qty` and `FillReport.last_qty` remain gross execution quantities.
An optional `quantity_commission: Decimal` expresses a positive fee in inventory units
of the existing instrument ID. It is mutually exclusive with a nonzero `Money` commission.
Existing constructors default to `None`; old JSON without the field remains readable.
Adding a public field still changes Rust struct literals and ABI.

The execution engine classifies duplicate fills and price restatements using its existing
rules before validating account and inventory support for a new change. Validation still
precedes order mutation. A historical duplicate is not checked against the inventory of a
later position episode. The position applies gross execution and the inventory deduction together, then
finalizes quantity and lifecycle state. The engine publishes the ordinary order and
position events only after these changes are visible in Cache.

The deduction reuses the internal commission adjustment used for CurrencyPair base
fees. It is derived from the original fill, not recorded as a second replay input.
Existing currency commission behavior is unchanged. No BinaryOption-specific branch,
adapter position ledger, synthetic SELL, or recovery timer is introduced.

## Cost convention

The new path removes fee inventory at the position's moving-average acquisition cost:

```text
quantity change = -fee quantity
realized PnL change = -fee quantity * current average acquisition price
```

The position retains cumulative raw quantity and cost in one optional
`QuantityCommissionTotals`. Quantity fees are projected upward to instrument precision;
only the change in that cumulative projection is deducted on each fill. Fee cost likewise
uses the change in cumulative `Money` projection. This preserves fee tails across executions
instead of rounding each fee independently. Quantity and cost residuals are derived from
the totals, with no separately mutable carry fields.

Remaining inventory retains its moving-average price. A BUY of 1 at 100 with a
0.01-inventory fee leaves 0.99 and realizes -1 in settlement currency. Selling 0.99
at 110 with no further fee closes with 8.9 realized PnL, matching the cash difference.
This convention does not debit reported cash a second time or redefine existing
multi-currency base commission PnL.

## Accounting compatibility

This new cost convention is not a reinterpretation of existing `Money` commissions.
A CurrencyPair BUY of 1 at 100 with a 0.01 base-currency `Money` fee and SELL of 0.99 at
110 retains the existing quote realized PnL of 9.9 and separately reports the base fee.
The inventory-unit prototype reports 8.9 in quote realized PnL by expensing the fee's
acquisition cost. These metrics are not interchangeable all-in reports. Existing Money
fees must not be migrated to the new field just to make their quote PnL equal.

The prototype's conservative quantity projection remains limited to inventory-fee
positions; it does not establish a universal precision contract for all adjustments or
positions. Generalizing the cost presentation, calculated accounts, and numeric ownership
requires separate agreement and validation. Duplicate classification does not widen any
of those supported scopes.

## Supported scope and explicit exclusions

- Reported cash accounts, long linear inventory, unit multiplier.
- Positive Decimal fees, including 18-decimal tails, with conservative cumulative quantity projection.
- Native gross order completion, partial executions, duplicate replay, and net position lifecycle.
- JSON, Arrow order events, and Cap'n Proto order events retain the new field.

Inverse products, shorts/flips, calculated accounts, price restatements, fill voids,
are rejected before application. An opening net balance below one native quantity quantum
is also rejected rather than publishing a zero-size PositionOpened. Fee totals are checked
before order/position mutation.
Native quantity remains at the instrument precision (16 for the Predict execution harness).
This is exact 18-decimal evidence with cumulative projection, not an 18-decimal rewrite of
Position, Portfolio, or PnL. Gross projection stays with the adapter's existing protocol
normalization. A remaining fraction below the native quantum is visible in the residual;
it is not proof that a venue inventory balance is exactly zero. Its marked value must be
included when comparing raw venue equity with projected native inventory.

PnL retains existing moving-average floating-point and per-fill Money rounding. The
mixed-price BUY/SELL/re-entry probe differs from cash by one settlement-currency quantum;
this change does not introduce a second PnL algorithm to eliminate that existing boundary.

Rust order `quantity_commissions()` reports inventory-unit fees separately from the
existing Money map. Position and PositionSnapshot include the cumulative quantity and cost;
JSON and Arrow snapshots retain these totals. Residual methods take instrument/currency
precision explicitly. Episode reset and purge/replay rebuild the same native state.

Python/PyO3/FFI access and correction semantics remain follow-ups before an upstream-ready
implementation. Existing persisted currency-valued fee history must not be mixed with the
new representation. An earlier quantity-fee prototype position without totals must be
rebuilt from its original fill events before further fills. Migrating that unpublished
prototype cache is unsupported; normal fill application does not scan history to detect it.

## Validation

Model tests cover inventory, disposal cost, closure metadata, JSON reconstruction,
duplicate application, and unchanged state on conflicting fee input. Additional tests
accumulate 100 subprecision fees, restore mid-stream, retain snapshot totals, purge fills,
and reset episodes. Historical duplicates after reopening a smaller position are ignored,
including after JSON reconstruction and with a different event ID. Engine tests also retain
atomic oversell rejection, subsequent valid closure, and unsupported price restatement rejection.
Cap'n Proto tests retain an exact 18-decimal fee value; this tests serialization rather than inventory precision.

The downstream Predict harness additionally tests multiple execution prices,
maker/taker mixtures, gross order completion with immediate net Cache observations,
quote-fee SELL exits, reconciliation, and a Rust Strategy + LiveNode wallet reconnect.
The harness uses retained exchange JSON and synthetic loopback services; no live order
is required and no new live acceptance is claimed.
