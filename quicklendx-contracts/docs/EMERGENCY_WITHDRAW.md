# Emergency Withdraw Runbook

This document describes the emergency withdrawal flow implemented in `quicklendx-contracts/src/emergency.rs`. The flow is admin-only, timelocked, and intended for last-resort recovery of non-escrow surplus tokens that are stuck in the contract.

## Entry Points

| Function | Purpose | Auth |
| --- | --- | --- |
| `EmergencyWithdraw::initiate(env, admin, token, amount, target)` | Queue a pending withdrawal and assign the next nonce. | `admin.require_auth()` plus `AdminStorage::require_admin` |
| `EmergencyWithdraw::execute(env, admin)` | Execute the current pending withdrawal after the timelock and before expiration. | `admin.require_auth()` plus `AdminStorage::require_admin` |
| `EmergencyWithdraw::cancel(env, admin)` | Permanently cancel the current pending withdrawal and mark its nonce cancelled. | `admin.require_auth()` plus `AdminStorage::require_admin` |
| `EmergencyWithdraw::get_pending(env)` | Return the current pending withdrawal, if any. | Read-only |
| `EmergencyWithdraw::get_nonce(env)` | Return the current global emergency-withdraw nonce. | Read-only |
| `EmergencyWithdraw::is_nonce_cancelled(env, nonce)` | Check whether a nonce has been cancelled. | Read-only |
| `EmergencyWithdraw::can_execute(env)` | Return whether the current pending withdrawal is executable now. | Read-only |
| `EmergencyWithdraw::time_until_unlock(env)` | Return seconds until the timelock unlocks, or `0` after unlock. | Read-only |
| `EmergencyWithdraw::time_until_expiration(env)` | Return seconds until the pending withdrawal expires, or `0` after expiration. | Read-only |

## State Machine

```text
no pending withdrawal
  |
  | initiate(admin, token, amount, target)
  v
pending and locked
  | now < unlock_at
  | execute => EmergencyWithdrawTimelockNotElapsed
  |
  | now >= unlock_at
  v
unlocked execution window
  | execute before expires_at => transfer non-escrow surplus and remove pending withdrawal
  | cancel => mark nonce cancelled and keep pending record as cancelled
  | now >= expires_at
  v
expired
  | execute => EmergencyWithdrawExpired
  | cancel => mark nonce cancelled
```

`initiate` computes:

- `initiated_at = env.ledger().timestamp()`
- `unlock_at = initiated_at + DEFAULT_EMERGENCY_TIMELOCK_SECS`
- `expires_at = unlock_at + DEFAULT_EMERGENCY_EXPIRATION_SECS`
- `nonce = increment_nonce(env)`

The current constants are:

| Constant | Value | Meaning |
| --- | ---: | --- |
| `DEFAULT_EMERGENCY_TIMELOCK_SECS` | `24 * 60 * 60` | 24-hour delay before execution can start |
| `DEFAULT_EMERGENCY_EXPIRATION_SECS` | `7 * 24 * 60 * 60` | 7-day execution window after unlock |
| `MIN_EMERGENCY_TIMELOCK_SECS` | `60 * 60` | 1-hour minimum for configurable flows |
| `MAX_EMERGENCY_TIMELOCK_SECS` | `30 * 24 * 60 * 60` | 30-day maximum for configurable flows |

## Boundary Rules

The execution window is `[unlock_at, expires_at)`:

- Execute exactly at `unlock_at`: allowed if all other checks pass.
- Execute before `unlock_at`: rejected with `EmergencyWithdrawTimelockNotElapsed`.
- Execute at `expires_at` or later: rejected with `EmergencyWithdrawExpired`.
- Cancel after unlock: allowed, as long as the pending withdrawal has not already been cancelled.
- Re-initiate: increments the global nonce and stores a new pending withdrawal record.

## Nonce Model

`get_nonce` returns the current global nonce, starting at `1` when no value is stored. `initiate` calls `increment_nonce`, stores the incremented nonce on the pending withdrawal, and writes that pending record to instance storage.

`cancel` marks the current pending withdrawal as `cancelled = true`, sets `cancelled_at`, and stores `true` under the per-nonce cancellation key. After that, `is_nonce_cancelled(env, nonce)` returns `true` for that nonce.

A cancelled withdrawal cannot be executed later, even if the timelock has elapsed, because `execute` checks `pending.cancelled` before the unlock and expiration checks.

## Execute Rejection Conditions

`execute(env, admin)` rejects when:

| Condition | Error |
| --- | --- |
| Caller is not the current admin | `NotAdmin` from `AdminStorage::require_admin` |
| No pending withdrawal exists | `EmergencyWithdrawNotFound` |
| Pending withdrawal is cancelled | `EmergencyWithdrawCancelled` |
| `now < unlock_at` | `EmergencyWithdrawTimelockNotElapsed` |
| `now >= expires_at` | `EmergencyWithdrawExpired` |
| Held escrow reserve repair is incomplete | `EmergencyWithdrawInsufficientBalance` |
| Contract balance is below the held reserve | `EmergencyWithdrawInsufficientBalance` |
| Requested amount exceeds `balance - held_reserve` | `EmergencyWithdrawInsufficientBalance` |
| Token transfer fails | Token transfer error propagated from `transfer_funds` |

`initiate` rejects `amount <= 0` with `InvalidAmount`, rejects `token == current_contract_address` or `target == current_contract_address` with `InvalidAddress`, and rejects timestamp overflow/invalid expiration with `InvalidTimestamp`.

`cancel` rejects when no pending withdrawal exists (`EmergencyWithdrawNotFound`) or when the pending withdrawal is already cancelled (`EmergencyWithdrawCancelled`).

## Operator Runbook

1. Inspect the pending withdrawal:

   ```rust
   let pending = EmergencyWithdraw::get_pending(env);
   ```

2. If there is a pending withdrawal, check the lifecycle helpers before attempting execution:

   ```rust
   let can_execute = EmergencyWithdraw::can_execute(env);
   let unlock_wait = EmergencyWithdraw::time_until_unlock(env);
   let expiration_wait = EmergencyWithdraw::time_until_expiration(env);
   ```

3. Interpret the helper values:

   - `can_execute == None`: no pending withdrawal exists.
   - `can_execute == Some(false)` and `unlock_wait > Some(0)`: wait for the timelock.
   - `can_execute == Some(false)` and `expiration_wait == Some(0)`: the request has expired; initiate a new one if recovery is still required.
   - `can_execute == Some(false)` while unlocked and unexpired: check escrow reserve repair and non-escrow surplus before retrying.
   - `can_execute == Some(true)`: execution is currently within the valid window and reserve checks pass.

4. Execute only after confirming the token, amount, target, nonce, and timing:

   ```rust
   EmergencyWithdraw::execute(env, admin)?;
   ```

5. Cancel immediately if the target, amount, token, or incident context is wrong:

   ```rust
   EmergencyWithdraw::cancel(env, admin)?;
   ```

6. After cancellation, record the nonce and verify it is cancelled:

   ```rust
   let cancelled = EmergencyWithdraw::is_nonce_cancelled(env, pending_nonce);
   ```

## Safety Notes

- Emergency withdrawal is pause-exempt so operators can abort or complete recovery during incident mode, but admin authentication and the timelock remain mandatory.
- The withdrawal can only use non-escrow surplus for the selected token. Held escrow reserves are checked before transfer.
- The pending withdrawal is a single slot. Operators should treat `initiate` as replacing the active recovery plan and should record the nonce, token, amount, target, unlock time, and expiration time in the incident log.
- The source comment currently references `docs/contracts/emergency-recovery.md`; this runbook is the contract README-linked operational document requested by issue #1725.