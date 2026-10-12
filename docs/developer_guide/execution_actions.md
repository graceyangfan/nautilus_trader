# Execution actions

## Purpose

An execution action is a strategy-initiated venue write which is not an order. It has a correlated
delivery result, but it does not invent an order state machine, an order ID, or a synthetic
settlement callback. The first standard action family is a binary-option inventory conversion:

| Action                      | Economic effect                                       |
| --------------------------- | ----------------------------------------------------- |
| `BinaryOptionAction::Split` | Convert quote collateral `A` into `A` Yes and `A` No. |
| `BinaryOptionAction::Merge` | Convert `A` matched Yes/No into quote collateral.     |

The canonical command and result are `ExecuteAction` and `ExecutionActionResult`.

```text
Strategy
  -> ExecuteAction { BinaryOptionAction { pair, quantity, params } }
  -> ExecutionEngine (client/account routing only)
  -> execution client
  -> ExecutionActionResult { command_id, Accepted | Rejected | Ambiguous }

Venue account/fill/position state
  -> existing reconciliation, account, and position paths
```

`ExecutionAction` is a closed common type family. It currently contains only
`ExecutionAction::BinaryOption(BinaryOptionAction)`. There is deliberately no `Adapter { name,
params }` escape hatch: accepting an opaque string would move command identity and validation back
into every adapter, while falsely presenting one common API. A future action becomes a new common
variant only after its semantic identity and simulator behavior are understood.

## Public strategy API

Rust accepts any currently supported standard action through the single generic method:

```rust
self.execute_action(
    self.account_id,
    Some(self.client_id),
    BinaryOptionAction::Split {
        pair: BinaryOptionPair::new(self.yes_id, self.no_id)?,
        quantity: Quantity::from("1.25"),
        params: None,
    },
)?;
```

The `From<BinaryOptionAction> for ExecutionAction` conversion happens at the strategy boundary.
Strategies do not know a venue action name or reconstruct a venue market identifier.

Python uses the equivalent immutable thin object. It has no routing or Hyperliquid-specific logic:

```python
action = BinaryOptionAction.split(
    yes_instrument_id=self.yes_id,
    no_instrument_id=self.no_id,
    quantity=Quantity.from_str("1.25"),
)
self.execute_action(
    account_id=self.account_id,
    action=action,
    client_id=self.client_id,
)
```

`quantity` is required and must be positive for both operations. There is no strategy-level
"merge maximum" shorthand: a concrete amount makes live and simulated scope identical and avoids
silently choosing between account-wide and strategy-owned inventory. `params` belongs to the typed
action, not the command envelope. It is reserved for venue-specific options and every adapter must
either validate it completely or reject it; silently ignoring a supplied option is invalid.

`BinaryOptionPair` is the action's complete economic identity: canonical Yes instrument ID plus
canonical No instrument ID. The constructor rejects duplicate IDs. The common layer does not use a
cross-venue `OutcomeId`, a numeric `MarketId`, or a condition ID.

## Delivery result semantics

`ExecutionActionResult` is an ACK for the observed delivery outcome, not evidence that account or
inventory convergence has completed.

| Result      | Evidence                                                                                             | Strategy behavior                                     |
| ----------- | ---------------------------------------------------------------------------------------------------- | ----------------------------------------------------- |
| `Accepted`  | The active client received an explicit successful venue response.                                    | Await normal account/position/reconciliation updates. |
| `Rejected`  | The request was not sent, or the venue explicitly rejected it.                                       | Treat as not applied under available evidence.        |
| `Ambiguous` | The request may have reached the venue but the active client cannot establish a definitive response. | Do not blindly retry; reconcile ordinary venue state. |

For Hyperliquid, the successful `userOutcome` post response is `status: "ok"` with
`response.type: "default"`. It carries neither action ID nor resulting balances, fees, fills, or
positions. An `Accepted` result therefore must not emit `OrderAccepted`, `OrderFilled`,
`FillReport`, `AccountState`, or a position event. Those state observations continue through the
existing private-stream and reconciliation paths and are uncorrelated aggregate state.

The action is `Rejected` if encoding or routing fails before sending, the client cannot perform the
action, the task cannot be admitted, the WebSocket post fails before writing, or Hyperliquid
explicitly returns `err`/`error`. It is `Ambiguous` after an attempted send when timeout,
disconnect, 429/5xx, response-channel closure, or response decoding prevents a definitive result.
If a client generation is stopped, disconnected, reset, or disposed, its cancelled task emits no
late result; after process failure no result can be delivered.

This follows the relevant `QueryAccount` / `QueryOrder` boundary without copying their lack of an
ACK: queries are observations, while an action is a venue write that needs a correlated delivery
notification.

## Hyperliquid HIP-4 live mapping

Hyperliquid owns the protocol mapping. The common command supplies the pair; the adapter verifies
the two `InstrumentId`s are its canonical HIP-4 Yes/No IDs for exactly one outcome, then encodes
that outcome for `userOutcome`:

| Standard action            | Hyperliquid request                          |
| -------------------------- | -------------------------------------------- |
| `Split { pair, quantity }` | `splitOutcome { outcome, amount: quantity }` |
| `Merge { pair, quantity }` | `mergeOutcome { outcome, amount: quantity }` |

This is deliberately a pair-to-outcome conversion only. The live action path does not consult
Cache, build a pair from an arbitrary `u32`, or assume another venue uses Hyperliquid's side
encoding. A Polymarket adapter can implement its own pair-to-condition/token conversion behind the
same common action.

Hyperliquid currently accepts no action `params`; a non-empty map is rejected before the request is
sent. The pair is also checked at encoding time, so malformed or cross-outcome IDs cannot be sent
as a valid `userOutcome` request.

## Sandbox and backtest behavior

Simulation is enabled only by an explicit `BinaryOptionActionProfile`; its default is `None`.
Unsupported or unconfigured venues return a correlated `Rejected` without mutation. The profile
holds fee configuration and a venue-owned `BinaryOptionActionValidator`, not user-provided pair
registrations or an adapter string resolver. Python receives an opaque standard profile from the
venue adapter and passes it to the normal venue configuration:

```python
from decimal import Decimal

from nautilus_trader.adapters.hyperliquid import hyperliquid_outcome_action_profile

usdc = Currency.from_str("USDC")
engine.add_venue(
    venue=Venue("HYPERLIQUID"),
    oms_type=OmsType.NETTING,
    account_type=AccountType.CASH,
    starting_balances=[Money(1_000, usdc)],
    base_currency=usdc,
    fee_model=MakerTakerFeeModel(maker_rate=Decimal(0), taker_rate=Decimal(0)),
    binary_option_action_profile=hyperliquid_outcome_action_profile(
        split_fee_rate=Decimal(0),
        merge_fee_rate=Decimal("0.0007"),
    ),
)
```

Configuration/registration checks occur once:

- fee rates are representable and in their allowed range;
- a validator exists whenever the profile is enabled;
- the simulator has a compatible, unfrozen cash account and netting OMS.

When an action arrives, the simulator checks current, mutable facts before changing state:

1. the pair is present in Cache and both instruments are `BinaryOption`;
2. the legs are canonical for the configured venue action and their supplied venue options are
   valid;
3. the instruments are still tradable for conversion (not expired or closed), with
   compatible currency, precision, and increment;
4. quantities, fees, cash, and resulting position changes are exactly representable;
5. split has available quote `>= A + F_split`; merge has available Yes and No `>= A`;
6. both leg changes and the account update can be committed atomically.

The fee model is generic even when a venue currently charges zero:

```text
F_split = fee(Split, A, profile)
F_merge = fee(Merge, A, profile)

split: quote -= A + F_split; Yes += A; No += A
merge: Yes -= A; No -= A; quote += A - F_merge
```

On success the engine plans and applies two ordinary orderless `OrderFilled` records, one per
binary leg, through the existing simulated exchange/account/position pipeline. They do not create
orders or synthetic `OrderSubmitted`/`OrderAccepted` events. Reusing the fill/account machinery preserves the
normal `AccountState`, `PositionOpened`/`PositionChanged`/`PositionClosed`, portfolio, and strategy
event ordering. Once those mutations are visible, the simulator sends `Accepted` for the command.

This reuse is intentionally narrow: an order fill expresses an execution, while a conversion fill
is an internal accounting record with action provenance. The latter is never routed through order
management, risk order checks, execution algorithms, or the order emulator.

A simulation rejection is deterministic and pre-mutation. A simulator has no transport uncertainty,
so it returns only `Accepted` or `Rejected`; `Ambiguous` is a live delivery state.

## Extension boundary

Do not add `OutcomeActionLeg`, `ExecutionActionSettlement`, a parallel position API, or synthetic
orders. They duplicate the already authoritative cache/account/position flows and make the action
result compete with normal state events.

To add another standard action family, define its complete common payload and invariants, add one
`ExecutionAction` variant, expose the same thin Rust/Python action object, define its live adapter
encoding and its explicit simulator profile/validator, then add observable tests for its ACK and
state transitions. An opaque adapter-defined command is not a substitute for that work.

## Evidence limits

Unit and integration tests prove structured request construction, result classification, simulator
account/position propagation, and local routing. The Hyperliquid adapter also tests REST and
WebSocket parsing against a quantity-1 testnet split/merge recording, with transaction hashes
anonymized. Venue-confirmed conversion fills retain their actual quantities, prices, fees, and
identifiers and enter the existing orderless leg-fill path. Reconciliation uses external instrument
claims to route these fills; it does not invent an association with a local action command.

Successful position-report collection also refreshes authoritative account state using the existing
account query. Live reported accounts do not derive cash from conversion fills. The account and
position reads are separate snapshots, and an account-refresh failure does not discard valid
position reports. Short testnet validation does not establish long-term rate-limit behavior or
ordinary-order latency.
