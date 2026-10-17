# Instrument quantity commission prototype

This downstream Rust prototype accompanies [RFC #1](https://github.com/graceyangfan/nautilus_trader/issues/1).
It is not an agreed upstream interface or a production-ready fee implementation.

## Contract

`OrderFilled.last_qty` and `FillReport.last_qty` remain gross execution quantities.
An optional `quantity_commission: Decimal` expresses a positive fee in inventory units
of the existing instrument ID. It is mutually exclusive with a nonzero `Money` commission.
Existing constructors default to `None`; old JSON without the field remains readable.
Adding a public field still changes Rust struct literals and ABI.

The execution engine validates the supported account and inventory before changing the
order. The position applies gross execution and the inventory deduction together, then
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

Remaining inventory retains its moving-average price. A BUY of 1 at 100 with a
0.01-inventory fee leaves 0.99 and realizes -1 in settlement currency. Selling 0.99
at 110 with no further fee closes with 8.9 realized PnL, matching the cash difference.
This convention does not debit reported cash a second time or redefine existing
multi-currency base commission PnL.

## Supported scope and explicit exclusions

- Reported cash accounts, long linear inventory, unit multiplier.
- Positive fees exactly representable at instrument quantity precision.
- Native gross order completion, partial executions, duplicate replay, and net position lifecycle.
- JSON, Arrow order events, and Cap'n Proto order events retain the new field.

Inverse products, shorts/flips, calculated accounts, price restatements, fill voids,
and fees exceeding supported quantity precision are rejected before application.
Decimal retention alone does not implement 18-decimal inventory or residual allocation.

Python/PyO3/FFI access, complete fee summaries and position snapshot reporting, and
correction semantics remain follow-ups before any upstream-ready implementation.
The existing `commissions()` Money map is not a complete summary of quantity fees.
Existing persisted currency-valued fee history must not be mixed with the new representation.

## Validation

Model tests cover inventory, disposal cost, closure metadata, JSON reconstruction,
duplicate application, and unchanged state on conflicting fee input. Cap'n Proto tests
retain an exact 18-decimal fee value; this tests serialization rather than inventory precision.

The downstream Predict harness additionally tests multiple execution prices,
maker/taker mixtures, gross order completion with immediate net Cache observations,
quote-fee SELL exits, reconciliation, and a Rust Strategy + LiveNode wallet reconnect.
The harness uses retained exchange JSON and synthetic loopback services; no live order
is required and no new live acceptance is claimed.
