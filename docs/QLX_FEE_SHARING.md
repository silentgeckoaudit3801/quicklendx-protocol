# QLX Fee Sharing

Audience: contributors, operators, and reviewers who need to verify how invoice settlement proceeds are split between the investor and the protocol.

This guide summarizes the fee-sharing model documented in [Settlement](SETTLEMENT.md) and implemented by the settlement calculation path. It focuses on the values reviewers need to recompute when checking a settlement, dispute, or accounting report.

## Terms

| Term | Meaning |
| --- | --- |
| `face_value` | Original invoice amount in the smallest token unit. |
| `funded_amount` | Amount the investor provided to fund the invoice. It must be greater than zero and no larger than `face_value`. |
| `protocol_fee_bps` | Platform fee rate in basis points, where `10_000` is 100%. |
| `late_penalty_bps` | Late-payment penalty rate in basis points, capped separately from the protocol fee. |
| `late_penalty` | Extra amount charged to the business when a late-payment penalty applies. |
| `protocol_fee` | Amount retained by the protocol from settlement proceeds. |
| `investor_payout` | Net amount returned to the investor after protocol fees. |
| `total_collected` | Amount collected from the business, including any late penalty. |

## Base Settlement Split

When an invoice settles, the protocol compares the payment/face value against the investor's funded amount.

If there is no profit, the platform does not collect a profit fee:

```text
if face_value <= funded_amount:
  gross_profit = 0
  protocol_fee = 0
  investor_payout = face_value + late_penalty
```

If there is profit, the protocol fee is taken from gross profit, not from the full face value:

```text
gross_profit = face_value - funded_amount
protocol_fee = floor(gross_profit * protocol_fee_bps / 10_000)
investor_payout = face_value + late_penalty - protocol_fee
```

The investor keeps rounding dust because the protocol fee uses integer floor division.

## Late Penalty

The late penalty is computed from `face_value` and added to the total collected amount:

```text
late_penalty = floor(face_value * late_penalty_bps / 10_000)
total_collected = face_value + late_penalty
```

A late penalty increases what the business owes. It does not change the gross-profit basis used for the protocol fee unless contract logic is explicitly changed in a future version.

## No-Dust Invariant

Every settlement review should verify:

```text
investor_payout + protocol_fee == total_collected
```

This invariant ensures the split neither creates nor loses value during distribution.

## Worked Examples

### No Profit

```text
face_value = 10_000
funded_amount = 10_000
protocol_fee_bps = 200
late_penalty_bps = 0

gross_profit = 0
protocol_fee = 0
late_penalty = 0
investor_payout = 10_000
total_collected = 10_000
```

Check:

```text
10_000 + 0 == 10_000
```

### Profit With Fee

```text
face_value = 11_000
funded_amount = 10_000
protocol_fee_bps = 200
late_penalty_bps = 0

gross_profit = 1_000
protocol_fee = floor(1_000 * 200 / 10_000) = 20
late_penalty = 0
investor_payout = 10_980
total_collected = 11_000
```

Check:

```text
10_980 + 20 == 11_000
```

### Late Settlement

```text
face_value = 11_000
funded_amount = 10_000
protocol_fee_bps = 200
late_penalty_bps = 500

gross_profit = 1_000
protocol_fee = 20
late_penalty = floor(11_000 * 500 / 10_000) = 550
investor_payout = 11_000 + 550 - 20 = 11_530
total_collected = 11_550
```

Check:

```text
11_530 + 20 == 11_550
```

## Review Checklist

Before approving settlement or fee-sharing changes, verify:

- fee rates are expressed in basis points
- `funded_amount` is non-zero and does not exceed `face_value`
- protocol fee is computed from gross profit, not total collected
- late penalty is added to `total_collected`
- integer division rounds the protocol fee down
- the no-dust invariant holds for each example and test fixture
- docs that reference settlement math link back to this guide or `docs/SETTLEMENT.md`

## Out of Scope

This guide does not define governance for changing fee rates, tenant overrides, or treasury rotation. Those topics belong in platform-fee and admin-operation documentation.