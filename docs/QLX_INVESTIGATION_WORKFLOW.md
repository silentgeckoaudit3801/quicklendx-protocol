# QLX Investigation Workflow

Audience: operators and reviewers coordinating protocol investigations, invoice disputes, settlement anomalies, and backend/indexer incidents.

This workflow defines how an investigation moves from intake to closure, which evidence is required at each stage, and the target response times for different severity levels. For evidence categories, see [QLX Evidence Model](QLX_EVIDENCE_MODEL.md). For dispute states, see [Dispute Lifecycle](DISPUTE.md). For settlement accounting, see [Settlement](SETTLEMENT.md).

## Severity and SLA Targets

| Severity | Use when | Initial triage | Status updates | Target closure |
| --- | --- | --- | --- | --- |
| Critical | Funds may be at risk, unauthorized transition, freeze/pause needed, or settlement invariant appears broken | 30 minutes | Every 2 hours | Same day or documented mitigation |
| High | Invoice, bid, dispute, or settlement state is wrong for one or more users but funds are not actively draining | 4 hours | Daily | 2 business days |
| Medium | Backend/indexer discrepancy, stale dashboard state, missing evidence, or delayed admin review | 1 business day | Every 2 business days | 5 business days |
| Low | Documentation, labeling, support clarification, or non-urgent follow-up | 2 business days | Weekly | Next planned maintenance window |

If severity is unclear, start one level higher and downgrade only after evidence rules out fund movement or irreversible state change.

## Investigation Stages

### 1. Intake

Create or update the operator record with:

- reporter and contact channel
- invoice ID, bid ID, dispute ID, or transaction hash
- affected network and contract ID
- short description of the observed problem
- first observed time and current severity
- links to any screenshots, logs, explorer pages, or support tickets

Do not accept screenshots as the only evidence for on-chain state. Record the stable identifier behind the screenshot whenever possible.

### 2. Triage

Within the severity SLA, determine:

- whether the issue affects a single invoice, a tenant, or the whole protocol
- whether funds are at risk or already moved
- whether the invoice is in a terminal state (`Paid`, `Defaulted`, `Cancelled`, or `Refunded`)
- whether a platform admin action is required before the state can progress
- whether off-chain indexer/backend state disagrees with on-chain state

Triage outcome must produce one of these routes:

- dispute review
- settlement/payment reconciliation
- backend/indexer recovery
- operational freeze or pause review
- documentation/support clarification
- no-action closure with evidence

### 3. Evidence Collection

Collect the minimum evidence needed for the selected route.

For dispute review:

- dispute creation transaction or audit record
- creator address and role
- current dispute status
- evidence IDs or document digests
- admin reviewer identity, if already under review

For settlement/payment reconciliation:

- invoice status and payment amount
- settlement transaction hash
- token address and contract ID
- investor payout, protocol fee, late penalty, and total collected
- comparison against the no-dust invariant

For backend/indexer recovery:

- expected on-chain state
- observed backend/API/indexer state
- replay range or cursor position
- queue job ID or webhook delivery ID
- final state after replay or rebuild

### 4. Decision and Escalation

Escalate immediately when:

- funds moved to an unexpected recipient
- a terminal state appears inconsistent with evidence
- a pause/freeze control may be required
- a contract or backend bug is reproducible
- required evidence is missing and the current state may expire or be overwritten

Decision records should include:

- selected route and severity
- evidence reviewed
- reviewer and approver
- on-chain action to take, if any
- off-chain remediation, if any
- follow-up issue or PR links

### 5. Execution

Execute only the minimum action required by the decision.

Examples:

- resolve a dispute after evidence supports the outcome
- replay an indexer range after confirming the authoritative on-chain state
- update support/admin dashboard state after a backend correction
- open a focused bug issue with reproduction evidence
- update runbooks or docs when the investigation exposes a process gap

Avoid bundling unrelated cleanup into an incident fix. If the investigation reveals secondary issues, file them separately.

### 6. Closure

Close the investigation only after:

- the current on-chain state is known and recorded
- backend/indexer state is reconciled or intentionally marked stale
- user-facing/admin-facing status is correct
- evidence links remain accessible to reviewers
- follow-up issues are linked
- the reporter or owner has been notified when appropriate

The closure note should state whether the issue was confirmed, mitigated, fixed, non-reproducible, or informational.

## Handoff Template

```md
Severity:
Route:
Invoice / bid / dispute ID:
Network / contract ID:
Reporter:

Observed:
Expected:

Evidence reviewed:
- <tx hash / document ID / log ID>

Decision:
Action taken:
Follow-ups:
Next update due:
```

## Review Checklist

Before merging a PR or closing an operator ticket that changes investigation behavior, confirm:

- the SLA table still matches operational expectations
- evidence requirements link back to `docs/QLX_EVIDENCE_MODEL.md`
- dispute and settlement routes match `docs/DISPUTE.md` and `docs/SETTLEMENT.md`
- private data, credentials, and raw identity documents are excluded from public artifacts
- closure criteria require both on-chain and off-chain state checks where relevant