# QLX Evidence Model

Audience: operators, reviewers, and contributors who need a consistent way to evaluate evidence during invoice disputes.

QuickLendX disputes are opened by the business owner or funding investor and resolved by a platform administrator. This guide describes what evidence should be attached to a dispute, which evidence classes are useful for each dispute type, and how reviewers should preserve enough context to make a repeatable decision.

For the dispute state machine, authorization rules, and terminal-state behavior, see [Dispute Lifecycle](DISPUTE.md). For settlement accounting, see [Settlement](SETTLEMENT.md). For invoice states, see [Invoice Lifecycle](INVOICE_LIFECYCLE.md).

## Evidence Principles

Evidence should be:

- **Relevant**: tied to a specific invoice, bid, settlement, payment, or dispute transition.
- **Attributable**: identifies who supplied it and which account or service produced it.
- **Timestamped**: includes ledger sequence, transaction hash, off-chain timestamp, or both.
- **Preserved**: copied or referenced in a way that a reviewer can inspect later.
- **Minimal**: excludes private documents, credentials, unrelated customer data, and secrets.

Do not put sensitive personal data directly on-chain. Store private files in an approved off-chain evidence store and reference only the stable identifier or digest required for review.

## Evidence Types

| Type | Examples | Best used for | Notes |
| --- | --- | --- | --- |
| On-chain transaction evidence | Transaction hash, ledger sequence, event topics, contract ID | Funding, settlement, default, refund, state transition disputes | Prefer explorer links plus raw IDs so the evidence survives UI changes. |
| Contract state snapshot | Invoice status, bid status, escrow balance, settlement result | Showing current protocol state before review or resolution | Capture the query time and network. |
| Payment evidence | Token transfer hash, payment amount, payer/payee addresses | Paid-vs-unpaid disputes, partial payment reconciliation | Match token, amount, and invoice ID; do not rely on screenshots alone. |
| Business document evidence | Invoice PDF ID, purchase order ID, delivery receipt reference | Invoice validity or goods/services disputes | Store documents off-chain; reference digest or document ID. |
| KYC/admin evidence | Verification decision ID, reviewer ID, policy version | Eligibility, compliance, or admin-action disputes | Keep private identity data out of public PRs and on-chain notes. |
| Operational evidence | Indexer replay logs, webhook delivery ID, queue job ID | Backend/indexer disagreement and recovery investigations | Include replay range and final observed state. |
| User-provided narrative | Short reason string, structured dispute note | Initial dispute context | Treat as a pointer to evidence, not proof by itself. |

## Evidence by Dispute Class

### Invoice Authenticity

Use when a party disputes whether an invoice should have been verified or funded.

Accepted evidence:

- invoice document ID or digest
- business owner address
- admin verification transaction or audit record
- matching invoice amount, currency, due date, and debtor reference

Reviewer checks:

- invoice status was eligible when the dispute opened
- business owner or investor opened the dispute
- supporting document matches the invoice fields used on-chain

### Funding and Bid Acceptance

Use when the accepted bid, escrowed amount, or investor identity is disputed.

Accepted evidence:

- accepted bid ID and investor address
- bid acceptance transaction hash
- escrow/funding transfer evidence
- invoice status before and after acceptance

Reviewer checks:

- bid was accepted for the disputed invoice
- accepted amount matches the escrowed or funded amount
- non-winning bids were not used as settlement evidence

### Payment and Settlement

Use when the business claims it paid, the investor claims payment is missing, or settlement accounting is disputed.

Accepted evidence:

- settlement transaction hash
- payment token and payment amount
- computed settlement breakdown: investor payout, protocol fee, late penalty, total collected
- invoice status transition to `Paid` or other terminal state

Reviewer checks:

- `investor_payout + protocol_fee == payment_amount` when applying the no-dust invariant
- late penalty and fee basis points match the configured policy at settlement time
- payment token and contract IDs match the deployment record

### Default and Late Payment

Use when a party disputes whether an invoice should be defaulted or late penalties should apply.

Accepted evidence:

- invoice due date or repayment window
- current ledger/time evidence used by the default check
- payment history and partial payment records
- late-payment policy version if maintained off-chain

Reviewer checks:

- default trigger happened after the allowed repayment window
- partial payments were counted exactly once
- late penalty evidence references the same face value and funded amount as settlement evidence

### Refund or Dispute Resolution

Use when the outcome of an admin dispute resolution is challenged.

Accepted evidence:

- dispute creation and resolution transaction hashes
- original creator address
- admin reviewer address
- final resolution status and reason
- refund or release transaction evidence, if funds moved

Reviewer checks:

- dispute reached `UnderReview` before resolution when required by the state machine
- resolver had platform-admin authority
- final invoice state is terminal only when the resolution requires it

## Reviewer Workflow

1. Identify the invoice ID, dispute ID, network, and contract ID.
2. Classify the dispute using the categories above.
3. Collect the minimum evidence set for that class.
4. Verify on-chain evidence independently from user-provided screenshots.
5. Compare current contract state with the expected transition in `docs/DISPUTE.md` and `docs/INVOICE_LIFECYCLE.md`.
6. Record the decision, evidence IDs, reviewer, and timestamp in the operator system.
7. Resolve the dispute only after evidence supports the chosen outcome.

## Evidence Rejection Rules

Reject or request replacement evidence when it is:

- unrelated to the disputed invoice or bid
- missing network, contract ID, transaction hash, or stable document reference
- a screenshot with no verifiable underlying identifier
- contradictory without an explanation of which source is authoritative
- exposing secrets, private keys, raw credentials, or unnecessary personal data
- produced after the decision without explaining why it is still relevant

## PR and Test Guidance

Changes that alter dispute evidence handling should update this guide and the relevant contract/backend tests in the same PR. Documentation-only changes should stay focused on reviewer/operator behavior and should not change contract semantics.