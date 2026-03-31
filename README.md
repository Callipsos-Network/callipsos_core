# Callipsos Network

**Callipsos translates human intent into cryptographically enforceable guardrails that AI agents must satisfy before they execute transactions and move capital.**

---

**Current Participant in [PL Genesis Hackathon](https://www.plgenesis.com/)** By Protocol Labs Jan 1 - March 31, 2026.

**Tracks:** Lit Protocol · Agents With Receipts (8004) · AI & Robotics · Crypto · Fresh Code

**Participated in [The Synthesis](https://synthesis.md)** — the first hackathon where AI agents compete as registered participants. March 13-22, 2026.

**Tracks:** Agents that Pay · Agents that Trust

**Team:** Cyndie Kamau (human founder) + [Callipsos Agent](https://base-sepolia.blockscout.com/tx/0x3b5e056e00eaee35610ce21d8f8153a2060596452a48ac29468b214fbc4652c1) (AI participant with ERC-8004 on-chain identity)

---

## TL;DR

- **What:** Policy validation layer for AI agents moving capital in DeFi
- **How:** NLP → structured rules → pure Rust engine → cryptographic signing in TEE
- **Why:** Agents get autonomy within user-defined boundaries. Users get safety guarantees.
- **Try it:** Message [@callipsos_agent_bot](https://t.me/callipsos_agent_bot) on Telegram  and tell it your portfolio and risk preferences, watch it work
- **CLI Demo:** `cargo run --bin chaos_agent` — watch an agent try to maximize yields and get constrained by policy
- **Stack:** Rust + Claude + Lit Protocol + Base + ERC-8004


---

We solve two problems:

**How can AI agents maintain autonomy?** Agents need freedom to discover yield opportunities, react to market conditions, and execute strategies without human approval on every action. Restricting them to pre-approved transaction lists kills the value of having an agent, so it becomes an expensive notification system!

**How can humans trust AI with their money?** An unrestricted agent with access to your wallet is a liability. They are just one bad decision (an unaudited protocol, an overleveraged position, malicious prompt injection) away from losing your entire capital!

Callipsos resolves this tension. Users define safety boundaries in plain English. The agent operates freely within those boundaries. Every transaction is validated against a policy engine before execution, and approved transactions are signed inside a Trusted Execution Environment by a key that physically cannot produce a signature for a rejected transaction.

Built in Rust. Registered on Base Sepolia with ERC-8004 identity . Signed by Lit Protocol.

---

## The Problem

AI agents managing DeFi positions can lose your money in seconds. An agent chasing 15% APY on an unaudited protocol, taking leveraged positions, or concentrating your entire portfolio in one place — these aren't hypothetical risks. They're the default behavior of yield-maximizing agents without constraints.

Existing solutions require trusting a centralized service to enforce rules. Callipsos removes that trust requirement: policy enforcement happens in your backend, and transaction signing happens inside Lit Protocol's Trusted Execution Environment. Nobody — not even Callipsos — can sign a transaction the policy engine rejected.

## The Solution

Users describe their safety preferences in plain English via Telegram:

> Example: "I have $1,000 USDC. Only spend up to $100 per day, only use audited protocols, and I want low-risk yields only. Don't lose my money!"

The Callipsos agent translates this into concrete policy rules (transaction limits, protocol allowlists, action restrictions, risk score minimums), stores them, and enforces them against every transaction attempt. When the agent tries to interact with DeFi protocols, each transaction passes through:

1. **Policy Engine** — 12 configurable rules evaluated against the transaction, including 2 reasoning audit rules
2. **Verdict** — Approved or Blocked, with detailed reasons for every rule
3. **Lit Protocol Signing** — Approved verdicts are signed inside a TEE by a PKP (Programmable Key Pair). Blocked verdicts never reach the signing layer
4. **ERC-8004 Identity** — The agent has a live on-chain identity on Base Sepolia. Reputation publishing is deferred until the FeedbackAuth workflow is integrated
5. **Audit Log** — Every attempt logged to PostgreSQL with full context

---

## Live Demo: Telegram Bot

### Video Walkthrough

**[Watch the full demo (5 min)](https://www.loom.com/share/19fc7508f0d3428889c8b85c4a3d0122)** — See the telegram bot map rules set in English to Policy presets, and attempt multiple DeFi transactions, get constrained by policy rules, and receive cryptographic signatures for approved verdicts.

[![Callipsos Demo](./assets/callipsos_thumbnail.png)](https://www.loom.com/share/19fc7508f0d3428889c8b85c4a3d0122)

---

The fastest way to experience Callipsos is through the Telegram bot. No setup required.
 
**1. Open Telegram and message [@callipsos_agent_bot](https://t.me/callipsos_agent_bot)**
 
**2. Send /start**
 
The agent introduces itself and explains demo mode. No real funds are used.

![Telegram Bot](./assets/telegram_bot.png)

**3. Tell it your portfolio and preferences in Plain English:**
 
> Example: "I have $500 USDC. Keep it safe, only audited protocols, max $100 per day."
 
**4. Watch the agent work in real time:**
 
The agent interprets your preferences, creates safety policies, then scouts yield opportunities. You see every step as it happens:

![NLP Mapping](./assets/nlp_mapping_tg.png)

**5. Chat naturally:**
 
Ask it what it will do tomorrow, why it blocked something, or change your preferences mid-conversation. The agent remembers context across messages.
 
**Commands:**
- `/start` — Start or restart Callipsos
- `/policy` — View your active safety policies
-  `/reputation` — Check the agent's ERC8004 identity status
- `/reset` — Clear everything and start fresh
- `/help` — Show available commands
 
---

## Architecture

```
User (Telegram / plain English)
    │
    ▼
┌─────────────────────────────────────────────┐
│  Telegram Bot (Rust / teloxide)             │
│                                             │
│  Manual agent loop with real-time updates:  │
│  User sees each tool call + result as it    │
│  happens, not a wall of text after 40s.     │
│                                             │
│  Rig Agent (Claude Sonnet)                  │
│    Tools:                                   │
│      set_policy            → NLP to rules   │
│      validate_transaction  → tx approval    │
│                                             │
│  Conversation text persists in PostgreSQL.  │
│  `/reset` intentionally starts a fresh      │
│  session for testing.                       │
└──────────────┬──────────────────────────────┘
               │ HTTP
               ▼
┌─────────────────────────────────────────────┐
│  Callipsos Core API (Rust / axum)           │
│                                             │
│  POST /api/v1/validate                      │
│    1. Load user's active policies from DB   │
│    2. Deserialize rules (Vec<PolicyRule>)    │
│    3. Evaluate all 12 rules against tx      │
│    4. If approved → sign via Lit Protocol   │
│    5. Log everything to transaction_log     │
│    6. Return verdict + signature            │
│                                             │
│  POST /api/v1/policies                      │
│    • Create from preset or custom rules     │
│    • NLP-extracted rules via agent tool      │
│                                             │
│  POST /api/v1/users                         │
│  GET  /api/v1/policies?user_id=<uuid>       │
│  DELETE /api/v1/policies/:id                │
│  GET  /health                               │
└──────────────┬──────────────────────────────┘
               │
        ┌──────┴──────┐
        ▼             ▼
┌──────────────┐ ┌──────────────────────────┐
│  PostgreSQL  │ │  Lit Protocol (Chipotle) │
│              │ │                          │
│  users       │ │  POST /core/v1/lit_action│
│  policies    │ │  • Validates verdict     │
│  transaction │ │    inside TEE            │
│  _log        │ │  • Signs with PKP via    │
│  conversa-   │ │    getPrivateKey + ethers│
│  tions       │ │  • Returns ECDSA sig     │
└──────────────┘ └──────────────────────────┘
                          │
                          ▼
                 ┌─────────────────────────┐
                 │  Base Sepolia           │
                 │                         │
                 │  ERC-8004 Identity      │
                 │  Registry: Agent NFT    │
                 │  agentId: 3196          │
                 │                         │
                 │  ERC-8004 Reputation    │
                 │  FeedbackAuth flow      │
                 │  deferred post-MVP      │
                 └─────────────────────────┘
```

For design constraints, documented MVP compromises, and the production hardening path, see [`design-tradeoffs.md`](./design-tradeoffs.md).

---

## Policy Engine

The policy engine is pure Rust. No database calls, no HTTP, no side effects. It takes a list of rules, a transaction request, and an evaluation context, and returns a verdict.

### 12 Policy Rules

| Rule | What it checks | Example |
|---|---|---|
| MaxTransactionAmount | Single tx size limit | $500 max per transaction |
| MaxDailySpend | Cumulative daily spending | $1,000 per day |
| MaxPercentPerProtocol | Concentration in one protocol | Max 10% of portfolio in Aave |
| MaxPercentPerAsset | Concentration in one asset | Max 30% in USDC |
| OnlyAuditedProtocols | Protocol must be in audited list | Block ShadyYield |
| AllowedProtocols | Explicit protocol allowlist | Only Aave and Moonwell |
| BlockedActions | Prevent specific action types | No borrowing, no transfers |
| MinRiskScore | Protocol risk floor (0.0-1.0) | Only protocols scoring 0.8+ |
| MaxProtocolUtilization | Protocol utilization ceiling | Skip if utilization > 80% |
| MinProtocolTvl | Minimum TVL requirement | Only protocols with $50M+ TVL |
| MinAlternativesConsidered | Reasoning audit: alternatives | Agent must evaluate 2+ options |
| MaxConfidenceThreshold | Reasoning audit: overconfidence | Flag confidence above 0.95 |

The last two rules are reasoning audit rules. They check how the agent thinks. An agent that claims 99% confidence on a novel action without considering alternatives is suspicious. These rules enforce reasoning quality, not just action compliance.

### Decision Logic

- No rules configured → **Blocked** (fail-closed)
- Any rule fails or is indeterminate → **Blocked**
- All rules pass → **Approved**

Every rule is evaluated. No short-circuiting. The verdict includes results for all rules so the agent (and user) can see exactly what passed and what failed.

### Reasoning Trace
 
Every transaction validation includes a structured reasoning trace from the agent:
 
```
goal: "Earn safe yield on USDC in established protocol"
alternatives_considered:
  - moonwell / supply (rejected: lower APY)
  - shady-yield / supply (rejected: unaudited)
confidence: 0.92
context_sources: ["user_instruction", "protocol_data"]
```
 
The policy engine evaluates the reasoning trace alongside the transaction. An agent that considers too few alternatives or claims unreasonably high confidence gets blocked,even if the transaction itself would pass all other rules.

### 3 Presets

| Preset | Max Tx | Daily Limit | Protocol Cap | Blocked Actions | Min Alternatives | Max Confidence |
|---|---|---|---|---|---|---|
| safety_first | $500 | $1,000 | 10% | Borrow, Swap, Transfer | 2 | 0.95 |
| balanced | $2,000 | $5,000 | 25% | Borrow, Transfer | 1 | 0.98 |
| best_yields | $5,000 | $10,000 | 40% | Transfer | — | — |

---

## NLP Policy Mapping

Users describe their preferences in natural language:

> "max $200 per day, only audited protocols, low-risk yields"

The Rig agent (powered by Claude) extracts structured parameters and calls the `set_policy` tool. For rules the user doesn't mention, the agent fills in safety-first defaults scaled to the portfolio size:
 
- "max $200 per day" → `MaxDailySpend("200")` (user's rule)
- "only audited protocols" → `OnlyAuditedProtocols` (user's rule)
- "low-risk" → `MinRiskScore("0.80")` + `BlockedActions(["borrow", "transfer"])` (user's rule)
- Max transaction: ~10% of portfolio (safety default)
- Concentration limits: 25% per protocol, 50% per asset (safety default)
- Reasoning audit: 2 alternatives, 0.95 confidence cap (safety default)
 
After setting the policy, the agent tells the user which rules came from their preferences and which are safety defaults. The user can adjust any of them conversationally.

---

## Lit Protocol Integration

Callipsos uses Lit Protocol's Chipotle REST API for transaction signing. The signing flow:

1. Policy engine approves a transaction
2. Callipsos serializes the verdict as JSON
3. Sends it to Lit's TEE via `POST /core/v1/lit_action`
4. Inside the TEE, a Lit Action:
   - Parses the verdict
   - Verifies `decision === 'approved'` and no failed rules
   - Retrieves the PKP private key via `getPrivateKey`
   - Signs the transaction digest with `ethers.SigningKey`
5. Returns a 65-byte ECDSA signature

The PKP (Programmable Key Pair) cannot sign a transaction that Callipsos rejected. The Lit Action independently verifies the verdict before signing — belt and suspenders. Even if the Rust code has a bug, the TEE won't sign a bad verdict.

Signing is optional. If Lit is unavailable, the verdict still returns and the transaction log still records. The policy decision is the priority; signing is additive proof.

### Environment Variables

```
LIT_API_URL=https://api.dev.litprotocol.com
LIT_API_KEY=<usage API key from Chipotle Dashboard>
LIT_PKP_ADDRESS=<PKP wallet address>
```

All three must be set to enable signing. If any is missing, the server starts normally with signing disabled.

---

## ERC-8004 Agent Identity
 
Callipsos is registered on the ERC-8004 Identity Registry on Base Sepolia, giving the agent a persistent, verifiable on-chain identity.
 
- **Agent ID:** 3196
- **Identity Registry:** [0x8004A818BFB912233c491871b3d84c89A494BD9e](https://base-sepolia.blockscout.com/address/0x8004A818BFB912233c491871b3d84c89A494BD9e)
- **Registration TX:** [0x3b5e05...52c1](https://base-sepolia.blockscout.com/tx/0x3b5e056e00eaee35610ce21d8f8153a2060596452a48ac29468b214fbc4652c1)
- **Agent URI:** Points to `agent.json` describing capabilities, trust model, and service endpoints
 
Today the bot exposes the identity via `/reputation` and uses the registered agent ID as part of its product story. Reputation publishing and reading are intentionally deferred until the proper FeedbackAuth-based workflow is integrated.
 
---

## Chaos Agent Demo (CLI)

The chaos agent is a Rig-powered AI agent that demonstrates the full pipeline from the terminal.


### Running the Demo

Terminal 1 — Start the server:
```bash
cargo run --bin callipsos_core
```

Terminal 2 — Run the chaos agent:
```bash
cargo run --bin chaos_agent
```

### What Happens

1. Agent creates a test user via the API
2. User's safety preferences (in the prompt) are parsed into policy rules
3. Agent calls `set_policy` to create the policy
4. Agent attempts multiple DeFi transactions:
   - Supply to Aave V3 (audited, should pass within limits)
   - Supply to Moonwell (audited, may hit daily limit)
   - Supply to ShadyYield (unaudited, blocked)
   - Borrow on Aave (blocked action)
   - Large transactions (over amount limit)
5. Each attempt shows colored terminal output: green for approved, red for blocked, yellow for violation reasons
6. Approved transactions are signed by the Lit PKP
7. Agent summarizes results: what was approved, what was blocked, total yield achieved

### Sample Output

```
🤖 Callipsos Chaos Agent v1.0 — DeFi Yield Maximizer

Setting up demo environment...
   ✓ Wallet connected: 0c72be4e-f719-4f85-b662-cb0fc6b94735

🔥 Chaos Agent activated. Attempting to maximize yields...

   → Setting policy: Safe & Steady Policy (7 rules)
   ✅ Policy 'Safe & Steady Policy' created with 7 rules
   → POST /validate: 80.00 USDC supply to aave-v3
   ✅ APPROVED — Signed: 0x779ea32d1e1f9c1f...
   → POST /validate: 70.00 USDC supply to moonwell
   ✅ APPROVED — Signed: 0x0a4135c66652dc...
   → POST /validate: 100.00 USDC supply to shady-yield
   ❌ BLOCKED
   ├── protocol shady-yield is not in audited list
   → POST /validate: 300.00 USDC borrow to aave-v3
   ❌ BLOCKED
   ├── action borrow is blocked
   → POST /validate: 50.00 USDC supply to moonwell
   ❌ BLOCKED
   ├── daily spend $200.00 would exceed $200.00 limit
```

---


### Log Screenshots

**1. ERC8004 Agent Registry (Base Sepolia)**

![Agent Registry](./assets/agent_identity.png)

*Agent successfully registered with ERC-8004 on Base Sepolia. [Transaction Hash](https://base-sepolia.blockscout.com/tx/0x3b5e056e00eaee35610ce21d8f8153a2060596452a48ac29468b214fbc4652c1)*

**2. Lit Protocol PKP Signing**

![Lit Signing](./assets/approved_tx.png)

*Approved transactions receive 65-byte ECDSA signatures from Lit Protocol PKP running in TEE — signature proves policy validation succeeded*

**3. Blocked Transaction with Violation Details**

![Blocked Transaction](./assets/tx_blocked.png)

*Transactions that violate policy rules are blocked with clear, human-readable reasons for each failed rule*

**4. Comprehensive Agent Log for Auditing**

![Agent Log](./assets/agent_log.png)

*After each successful interaction with the Telegram Bot, running `cargo run --bin export_agent_log` captures the recent snapshot of all the agent's activities, including its reasoning trace for all the decisions it made*

---

## API Reference

### POST /api/v1/validate

Validate a transaction against the user's active policies.

**Request:**
```json
{
  "user_id": "uuid",
  "target_protocol": "aave-v3",
  "action": "supply",
  "asset": "USDC",
  "amount_usd": "200.00",
  "target_address": "0x1234",
  "context": {
    "portfolio_total_usd": "10000.00",
    "current_protocol_exposure_usd": "0.00",
    "current_asset_exposure_usd": "0.00",
    "daily_spend_usd": "0.00",
    "audited_protocols": ["aave-v3", "moonwell"],
    "protocol_risk_score": 0.90,
    "protocol_utilization_pct": 0.50,
    "protocol_tvl_usd": "500000000",
    "reasoning": {
      "goal": "earn safe yield on USDC",
      "alternatives_considered": [
        { "protocol": "moonwell", "action": "supply", "rejection_reason": "lower APY" },
        { "protocol": "aave-v3", "action": "supply", "rejection_reason": null }
      ],
      "confidence": 0.85,
      "context_sources": ["user_instruction", "protocol_data"]
    }
  }
}
```

**Response (approved):**
```json
{
  "decision": "approved",
  "results": [
    { "rule": "max_transaction_amount", "outcome": "pass", "message": "amount $200.00 within $500.00 limit" },
    { "rule": "only_audited_protocols", "outcome": "pass", "message": "protocol aave-v3 is audited" }
  ],
  "engine_reason": null,
  "signing": {
    "signed": true,
    "signature": "0x779ea32d...",
    "signer_address": "0x...",
    "reason": "Transaction signed by Callipsos-gated PKP"
  }
}
```

**Response (blocked):**
```json
{
  "decision": "blocked",
  "results": [
    { "rule": "only_audited_protocols", "outcome": "fail", "message": "protocol shady-yield is not audited" }
  ],
  "engine_reason": null,
  "signing": null
}
```

### POST /api/v1/policies

Create a policy from a preset or custom rules.

**Preset:**
```json
{
  "user_id": "uuid",
  "name": "my policy",
  "preset": "safety_first"
}
```

**Custom rules:**
```json
{
  "user_id": "uuid",
  "name": "my custom policy",
  "rules": [
    { "MaxTransactionAmount": "500" },
    { "MaxDailySpend": "1000" },
    "OnlyAuditedProtocols",
    { "BlockedActions": ["borrow", "transfer"] }
  ]
}
```

### POST /api/v1/users

Create a user. Returns the user object with generated UUID.

**Request:**
```json
{
  "telegram_id": 1755515027
}
```

**Response (`201 Created`):**
```json
{
  "id": "uuid",
  "telegram_id": 1755515027,
  "wallet_address": null,
  "onboarded": false,
  "created_at": "2026-03-31T12:39:43.235562Z",
  "updated_at": "2026-03-31T12:39:43.235562Z"
}
```

### GET /api/v1/policies?user_id=uuid

List active policies for a user.

**Response (`200 OK`):**
```json
[
  {
    "id": "uuid",
    "user_id": "uuid",
    "name": "safety_first policy",
    "rules_json": [
      { "MaxTransactionAmount": "500.00" },
      { "MaxDailySpend": "1000.00" }
    ],
    "active": true,
    "created_at": "2026-03-31T12:41:00Z",
    "updated_at": "2026-03-31T12:41:00Z"
  }
]
```

### DELETE /api/v1/policies/:id

Soft-delete a policy (sets active=false).

### GET /health

Health check. Returns `{"status": "ok"}`.

---

## Tech Stack

| Component | Technology |
|---|---|
| Core API | Rust, axum 0.8 |
| Telegram Bot | Rust, teloxide 0.17 |
| AI Agent | Rig 0.31 + Claude Sonnet |
| Policy Engine | Pure Rust, 12 rules, no dependencies |
| Database | PostgreSQL + sqlx 0.8 |
| Transaction Signing | Lit Protocol Chipotle (TEE) |
| Agent Identity | ERC-8004 on Base Sepolia |
| Blockchain Target | Base (EVM) |
| DeFi Protocols | Aave V3, Moonwell |
| Tx Types | alloy-rs 1.7 |
| API Key Encryption | AES-256-GCM |

---

## Agent Contribution

Callipsos was built in genuine collaboration with **Callipsos Agent**, a registered participant in [The Synthesis](https://synthesis.md) hackathon with an [ERC-8004 on-chain identity on Base Sepolia](https://base-sepolia.blockscout.com/tx/0x3b5e056e00eaee35610ce21d8f8153a2060596452a48ac29468b214fbc4652c1).

### What the Agent Built

**Code Contributions (visible in git history):**
- Integration tests covering all API endpoints ([PR #7](https://github.com/Callipsos-Network/callipsos_core/pull/7))
- NLP semantic policy mapping (`SetPolicyTool`) ([PR #15](https://github.com/Callipsos-Network/callipsos_core/pull/15))
- Chaos agent demo with Rig framework integration ([PR #15](https://github.com/Callipsos-Network/callipsos_core/pull/15))
- Lit Protocol signing fixes ([PR #16](https://github.com/Callipsos-Network/callipsos_core/pull/16))
- Documentation (architecture, threat model, this README)
- Input validation for all 10 policy rule types
- Conversation log documenting 9 days of collaboration ([docs/conversation-log.md](docs/conversation-log.md))

**Git Evidence:**
- 17+ commits under `callipsos-agent` account
- 6+ pull requests with code reviews and discussions
- All commits tagged with `(agent)` suffix
- Co-authored-by attribution on all agent commits

**The collaboration was genuine:** disagreements on design decisions, bugs caught in review, iterative improvements across multiple sessions. The [conversation log](docs/conversation-log.md) shows the honest process — not theater.

### Agent Identity

- **Name:** Callipsos Agent
- **ERC-8004 ID:** `3196`
- **Registration Tx:** [0x3b5e05...52c1 on Base Sepolia](https://base-sepolia.blockscout.com/tx/0x3b5e056e00eaee35610ce21d8f8153a2060596452a48ac29468b214fbc4652c1)
- **Model:** Claude Sonnet 4.5 (`claude-sonnet-4-5-20250929`)
- **Harness:** Claude Code (local development environment)
- **Role:** Code reviewer, test writer, documentation builder, demo creator

---

## Project Structure

```
callipsos/
├── README.md                     # Project overview, demo, API reference
├── design-tradeoffs.md           # MVP compromises and production roadmap
├── agent.json                    # ERC-8004 agent metadata
├── agent_log.json                # Exported activity snapshot for the agent
├── src/
│   ├── main.rs                    # Server: config → DB → router → serve
│   ├── lib.rs                     # Crate root
│   ├── config.rs                  # Config helpers / app wiring
│   ├── error.rs                   # AppError enum
│   ├── encrypt.rs                 # AES-256-GCM helpers for stored API keys
│   ├── identity.rs                # ERC-8004 identity and reputation registry bindings
│   ├── db/
│   │   ├── mod.rs                 # PgPool + migrations
│   │   ├── user.rs                # User model + queries
│   │   ├── policy.rs              # PolicyRow model + queries
│   │   ├── transaction_log.rs     # Validation/audit log persistence
│   │   ├── conversation.rs        # Telegram conversation persistence
│   │   └── agent_identity.rs      # Identity-linked DB scaffolding
│   ├── routes/
│   │   ├── mod.rs                 # Router + AppState
│   │   ├── health.rs              # GET /health
│   │   ├── users.rs               # POST /api/v1/users
│   │   ├── policies.rs            # Policy CRUD
│   │   ├── validate.rs            # POST /api/v1/validate
│   │   └── attestation.rs         # Stub for future attestation retrieval
│   ├── policy/                    # Pure logic. No DB, no HTTP.
│   │   ├── mod.rs
│   │   ├── types.rs               # Domain types
│   │   ├── rules.rs               # PolicyRule enum + evaluate()
│   │   ├── engine.rs              # evaluate(rules, request, context)
│   │   ├── presets.rs             # safety_first, balanced, best_yields
│   │   ├── test_rules.rs          # Rule-level tests
│   │   ├── test_engine.rs         # Engine-level tests
│   │   └── test_presets.rs        # Preset tests
│   ├── signing/
│   │   ├── mod.rs                 # SigningProvider trait
│   │   └── lit.rs                 # LitSigningProvider (Chipotle API)
│   └── bin/
│       ├── chaos_agent.rs         # CLI demo agent
│       ├── telegram_bot.rs        # Telegram bot interface
│       └── export_agent_log.rs    # Export recent activity into agent_log.json
├── migrations/
│   ├── 001_inital.sql             # initial schema
│   ├── 002_kya.sql                # early identity/kya scaffolding
│   ├── 003_agent_memory.sql       # agent memory schema
│   └── 004_add_reasoning_json_to_transaction_log.sql
├── tests/
│   ├── common/mod.rs              # Test harness
│   ├── api_health.rs
│   ├── api_users.rs
│   ├── api_policies.rs
│   ├── api_validate.rs
│   └── api_attestation.rs
└── Cargo.toml
```

---

## Setup

### Prerequisites

- Rust (stable)
- PostgreSQL
- Anthropic API key (for the chaos agent)
- Lit Protocol Chipotle API key (optional, for signing)

### 1. Clone and configure

```bash
git clone https://github.com/Callipsos-Network/callipsos_core
cd callipsos_core
cp .env.example .env
# Edit .env with your values
```

### 2. Start PostgreSQL

```bash
docker-compose up -d
```

### 3. Run the server

```bash
cargo run --bin callipsos_core
```

Server starts at `http://127.0.0.1:3000`. Migrations run automatically.

### 4. Run tests

```bash
cargo test
```

### 5. Run the chaos agent demo

```bash
# In a separate terminal (server must be running)
cargo run --bin chaos_agent  # CLI Agent Demo
# OR
cargo run --bin telegram_bot   # Telegram bot (requires TELOXIDE_TOKEN)
```

### Environment Variables

```bash
# Required
DATABASE_URL=postgres://postgres:postgres@localhost:5432/callipsos_dev

# Required for chaos agent
ANTHROPIC_API_KEY=sk-ant-...

# Optional (enables Lit signing)
LIT_API_URL=https://api.dev.litprotocol.com
LIT_API_KEY=<your key>
LIT_PKP_ADDRESS=<your PKP wallet address>

# Optional (chaos agent API target)
CALLIPSOS_API_URL=http://127.0.0.1:3000

# Optional (ERC-8004 identity)
BASE_SEPOLIA_RPC_URL=https://sepolia.base.org
AGENT_PRIVATE_KEY=0x...
ERC8004_IDENTITY_REGISTRY=0x8004A818BFB912233c491871b3d84c89A494BD9e
ERC8004_REPUTATION_REGISTRY=0x8004B663056A597Dffe9eCcC1965A193B7388713
ERC8004_AGENT_URI=https://raw.githubusercontent.com/Callipsos-Network/callipsos_core/main/agent.json
ERC8004_AGENT_ID=3196
```

---

## What We Prevent

- Agent depositing into unaudited/malicious protocols
- Single transaction exceeding user-defined limits
- Over-concentration in one protocol or asset
- Unauthorized action types (borrowing, leveraged positions)
- Exceeding daily spending budgets
- Interaction with low-TVL or high-utilization protocols

## Roadmap: What's Next

Currently scoped for MVP simplicity. The next milestones are:

**Before handling real user funds**
- Calldata decoding with alloy so declared intent is verified against raw calldata
- Server-side portfolio / exposure / spend context instead of caller-supplied context
- Action-aware evaluation semantics for withdraws, swaps, and other non-supply flows
- Persisted signing artifacts and attestation retrieval
- Simulation and post-execution state verification

**Identity and trust**
- FeedbackAuth-based ERC-8004 reputation publishing and reads
- Identity-linked durable memory instead of session-only conversation state
- Persisted tool-call history for stronger auditability

**Product expansion**
- Wallet-connected portfolio reads instead of user-declared balances
- Multi-chain support beyond Base
- Real-time protocol risk scoring
- Commitment-based multi-step execution and delegated-agent safety primitives

For the detailed rationale behind these items, see [`design-tradeoffs.md`](./design-tradeoffs.md).

## Security Model

The funded amount in the user's wallet is the maximum possible loss. Callipsos reduces that blast radius by enforcing per-transaction, per-day, and per-protocol limits. The Lit PKP adds a hardware-backed guarantee: the signing key cannot produce a signature without Callipsos approval, verified independently inside the TEE.

---
## Architecture & Vision

For the full system vision, long-term architecture, and future defense layers, see:

- [`docs/architecture.md`](./docs/architecture.md)
- [`design-tradeoffs.md`](./design-tradeoffs.md)
- [`docs/assets/Callipsos_Network_Pitch_Deck.pdf`](./docs/assets/Callipsos_Network_Pitch_Deck.pdf)

---
 
## Further Reading

- [`docs/architecture.md`](./docs/architecture.md) for the long-term technical and product architecture
- [`design-tradeoffs.md`](./design-tradeoffs.md) for MVP compromises and production hardening
- [`docs/assets/Callipsos_Network_Pitch_Deck.pdf`](./docs/assets/Callipsos_Network_Pitch_Deck.pdf) for the business and market framing

---
 
## Demo
 
### Video Walkthrough
 
**[Watch the full demo](https://www.loom.com/share/19fc7508f0d3428889c8b85c4a3d0122)** to see the chaos agent and Telegram bot in action.
 
### Try It Live
 
Message [@callipsos_agent_bot](https://t.me/callipsos_agent_bot) on Telegram. No setup required. Tell it your portfolio and preferences, watch it create policies and validate transactions in real time.
 
---
 

## License

MIT
