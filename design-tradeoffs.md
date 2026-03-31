#  Design Tradeoffs & Future Optimizations For Callipsos

> **Living document.** This doc is updated as we build. Each phase records what we chose, what we deferred, and why. When complexity grows, check here first before redesigning.

---

## Phase 1: Foundation + Policy Engine (23/02/2025)

Tradeoffs made during Phase 1 to keep scope tight. Revisit these in later phases.

### Deferred to Phase 2

| Tradeoff | What we did (Phase 1) | What to do later | Why we deferred |
|---|---|---|---|
| **`target_address` typing** | Plain `String` | Replace with alloy `Address` type with proper hex validation | alloy enters the crate in Phase 2. Hand-rolling a newtype now is throwaway work. |
| **Transaction calldata decoding** | `target_protocol` is declared intent from the agent, so we trust the request fields | Decode raw calldata with alloy `sol!` macro, verify target contract is actually the claimed protocol | Phase 2 adds alloy-rs. The policy engine doesn't change, only the validate route gets smarter about where `TransactionRequest` fields come from. |
| **`audited_protocols` as `HashSet`** | `Vec<ProtocolId>` with `.contains()` | Switch to `HashSet<ProtocolId>` for O(1) lookups | We have 3 protocols. O(n) on n=3 is not a bottleneck. Revisit when the allowlist grows past ~20. |
| **Transaction simulation** | No simulation. Policy engine approves/blocks based on rules only. | Add `eth_call` simulation via alloy provider on Base to preview transaction outcomes before execution. | Simulation requires an RPC connection and alloy. Not needed to prove the policy engine works. |
| **`ReallocationDeltaTooSmall` in `policy/rules`** | TODO: | Add as a policy rule for rate chasing logic | Will come in handy when designing the DeFi agents to prevent agent from churning.
| **`Money` arithmetic in `policy/types`** | Can add basic arithmetic ops for the engine | We'll design the tests first, then add ops when the test demands it | Currently not needed will check back.
|**Action-aware rule filtering** | All rules run for all actions. Math assumes additive (Supply)| Engine filters which rules apply by action type. Withdraw/Transfer skip exposure and spend rules | MVP only supports Supply on Aave/Moonwell. Other actions exist in the enum for forward-compatibility
|**Single-asset `TransactionRequest`** | One `asset:AssetSymbol` field. Works for Supply/Withdraw/Stake.| Add `asset_in` and `asset_out` for `Action::Swap`. `MaxPercentPerAsset` evaluates both sides.| Swaps aren't in MVP scope. Calldata decoding in Phase 2 is when swap fields become meaningful.


### Deferred to Phase 3+

| Tradeoff | What we did (Phase 1) | What to do later | Why we deferred |
|---|---|---|---|
| **`ChainId` on `TransactionRequest`** | Hardcoded to Base. No chain field. | Add `chain: ChainId(u64)` to `TransactionRequest`. Route allowlists and rule sets per chain. | Single-chain MVP. Adding a field we never read in any rule is dead code. Add when we actually support multiple chains. |
| **Time window rules** | Not implemented | Add `PolicyRule::TimeWindow { start_hour, end_hour, timezone }` — "Only allow transactions between 9am–9pm" | Low complexity, high trust value. Design is clean to add as a new enum variant. Not needed for hackathon demo. |
| **Cooldown / rate limit rules** | Not implemented | Add `PolicyRule::MaxTransactionsPerHour(u32)` — protects against compromised agent loops | Requires tracking tx count per time window in `EvaluationContext`. Easy to add, not needed for initial demo. |
| **Recipient allowlist/blocklist** | Not implemented | Add `PolicyRule::AllowedRecipients(Vec<Address>)` / `BlockedRecipients(Vec<Address>)` | Big for "agent goes rogue" narrative. Needs typed addresses (Phase 2). |
| **NLP → Policy mapping** | Policies set via presets only (safety_first, best_yields, balanced) | Claude function calling extracts structured `PolicyRule` from natural language via Rig | Phase 3 adds Rig + Claude. The policy engine and `rules_json` schema already support this, so only the input method changes. |
| **`primary_reason` on `PolicyVerdict`** | `failed_rules()` helper filters non-passing results | Add severity ranking to rules so verdict can surface the highest-priority violation | Implies a severity system between rules. Not needed when all rules are equally weighted. Add when UI needs "most important reason." |

### Decisions we're keeping

| Decision | Why it's right |
|---|---|
| **Policy engine is purely offchain** | Chain-agnostic, fast iteration, no audit/deploy/gas overhead. Signed verdicts provide on-chain verifiability without on-chain execution. Don't let a partner dictate architecture. |
| **`Money` as `rust_decimal::Decimal`, not `f64`** | Float boundary bugs in financial logic are unacceptable. `0.1 + 0.2 != 0.3` energy. Judges feel it when money logic uses floats. |
| **`BasisPoints(u32)` for percentages** | 10% = 1000 bps. Avoids float precision issues in percentage comparisons. |
| **Structured `Violation` enum over plain strings** | Machine-readable failures enable analytics, UI rendering, and signed attestations. |
| **`RuleResult` constructors enforce invariants** | Impossible to create a Pass with a Violation or a Fail without one. Type system prevents bugs. |
| **`RuleOutcome::Indeterminate` exists** | Most hackathon projects pretend uncertainty doesn't exist. We explicitly handle "can't evaluate" (e.g., portfolio total is zero) and default to blocked. |
| **`OnlyAuditedProtocols` reads from `EvaluationContext`, not hardcoded** | Keeps rules pure and testable. Allowlist can be updated without code changes in the future. |
| **Evaluate all rules, don't short-circuit** | Aggregated results show full breakdown: "Failed 2 rules: daily limit + protocol not audited." Better for trust-building and demos. |

---


## Phase 2: Validation Pipeline + Signing (13/03/2026)

Tradeoffs made during Phase 2 for Lit Protocol integration and API completion.

### Deferred to Phase 3+

| Tradeoff | What we did (Phase 2) | What to do later | Why we deferred |
|---|---|---|---|
| **Lit Action code inline vs IPFS** | Send Lit Action JS code inline with every `/core/v1/lit_action` request | Pin to IPFS and reference by CID for immutability guarantees. Register CID in Chipotle group for tighter scoping. | Inline is simpler and avoids IPFS availability dependency. For production, pinned CID proves to users the signing logic hasn't changed. |
| **Placeholder tx hash** | Generate `0x{uuid}` as stand-in tx hash for signing | Sign actual transaction calldata hash once alloy-rs calldata decoding lands | No real on-chain transactions yet. The signing flow works the same — real hash is just a different input. |
| **`signer_address` not populated** | `SigningResult.signer_address` is always `None` | Derive PKP address from public key and include in response | Address derivation requires keccak256 of the uncompressed public key. Not needed for demo — the signature itself proves the PKP signed. |
| **Signing failure is silent** | If Lit API fails, log a warning and return verdict without signature. `signing: null` in response. | Surface signing errors to caller via a `signing_error` field or separate status | For MVP, the policy decision is the priority. Signing is additive. Don't let Lit downtime break the validate endpoint. |
| **No retry on Lit API failure** | Single attempt, fail-open (verdict still returned) | Add retry with backoff for transient Lit API errors | Complexity not justified for MVP. Chipotle dev network may have occasional downtime. |
| **Risk score float precision** | `protocol_risk_score` arrives as f64, converted via `Decimal::from_f64_retain` which produces long decimals (e.g. `0.4000000000000000222044604924`) | Accept risk score as string (like money fields) or round after conversion | Display is correct (rounds to 2dp), only the raw serialized violation shows the noise. Cosmetic issue, not a correctness issue. |
| **Caller-supplied protocol metadata and market context** | `validate` trusts the request body for `audited_protocols`, risk score, utilization, TVL, and exposure/spend context | Resolve these server-side from chain state, risk oracles, internal ledgers, or signed upstream data. Caller should provide transaction intent, not the facts used to judge it | For MVP, the policy engine needed realistic inputs without requiring wallet reads, RPC infra, or market-data integrations. This keeps the validator pure and demoable while we prove the pipeline. |
| **No API authentication / authorization** | The REST API is open to any caller who can reach it. Routes accept `user_id` / `policy_id` directly and rely on the bot or local clients to behave correctly | Add auth (wallet signatures, API keys, session auth, or signed agent identity) and ownership checks on every user/policy mutation path | The current deployment target is a local/demo environment where the Telegram bot and chaos agent are the only intended clients. Auth would add substantial product and infrastructure work under hackathon time pressure. |
| **Transaction log stores verdicts, not full signing artifacts** | `transaction_log` records request JSON and rule results, but not the Lit signature, PKP address, or signing failure reason | Extend the schema with signing metadata or add a separate `attestations` table linked to `transaction_log.id` | The verdict itself is the primary artifact in MVP. Persisting signatures and failure modes matters once audits, dashboards, or on-chain verification flows need to query history directly. |
| **No dedicated attestation retrieval endpoint** | Signing result comes back inline on `POST /api/v1/validate`. `src/routes/attestation.rs` exists only as a stub | Add an attestation route that can fetch or verify historical verdict/signature pairs by transaction or log ID | The demo only needs to show "approved + signed" in the immediate response. A read API becomes important when third parties need to inspect historical attestations without replaying the validation flow. |
| **Naga → Chipotle migration** | Built directly on Chipotle (Lit v3) REST API. No Naga code exists. | Move to Chipotle production when it launches (~March 25) | Naga is sunsetting April 1. Chipotle dev is live and working. Swap `LIT_API_URL` to production endpoint when available. |
| **No IPFS CID scoping in Chipotle group** | Group has "all actions permitted" flag for simplicity | Register specific IPFS CID in group, scope usage API key to only that action | Tighter security for production. MVP uses inline code so CID scoping doesn't apply yet. |
| **Express sidecar eliminated** | Call Chipotle REST API directly from Rust via reqwest. No `lit-signer/` TS service. | N/A — this is the final architecture | Chipotle's REST API made the sidecar unnecessary. Fewer moving parts, one language, one process. |

### Decisions we're keeping

| Decision | Why it's right |
|---|---|
| **`SigningProvider` trait abstraction** | `LitSigningProvider` today, could swap to any other signing backend (ZeroDev, Ika, local HSM) without touching the validate route. Trait takes `&PolicyVerdict` + tx hash, returns `SigningResult`. |
| **Signing is optional (`Option<Arc<dyn SigningProvider>>`)** | Server starts and works without Lit configured. All Phase 1 tests pass with `signing_provider: None`. No env vars required for development. |
| **Signing only on approved verdicts** | Blocked verdicts never touch the Lit API. The PKP physically cannot sign a transaction that Callipsos rejected. This is the core security guarantee. |
| **Lit Action double-checks the verdict** | The Lit Action independently verifies `decision === 'approved'` and no failed rules before signing. Belt-and-suspenders — even if the Rust code has a bug, the TEE won't sign a bad verdict. |
| **`ValidateResponse` uses `#[serde(flatten)]` on `PolicyVerdict`** | Keeps the existing `decision`, `results`, `engine_reason` fields at the top level. Adding `signing` alongside them is non-breaking — Phase 1 consumers see the same shape plus a new nullable field. |
| **Inline Lit Action code over IPFS** | Matches how Chipotle's own SDK (`litAction` method) sends code. Avoids IPFS pinning setup, gateway availability issues, and extra dashboard config. Code is ~30 lines and deterministic. |

---

## Phase 3: AI Layer + Conversational Interface  (26/03/2026)

Tradeoffs made during Phase 3 for the Telegram bot, Rig agent integration, and conversational UX.
 
### Deferred to post-hackathon
 
| Tradeoff | What we did (Phase 3) | What to do later | Why we deferred |
|---|---|---|---|
| **Per-user API keys (`/setkey`)** | All users share the server operator's `ANTHROPIC_API_KEY`. No `/setkey` command in MVP. | Add `/setkey` command. User's key encrypted with AES-256-GCM (`src/encrypt.rs`), stored in `users.llm_api_key_encrypted`. Bot decrypts per-session and builds Rig agent with user's own key. | Requiring users to paste an Anthropic API key before chatting kills adoption for testing. Server operator eats compute costs during the testing phase. Cost per test session is ~$0.05-0.15. |
| **LLM provider selection** | Anthropic only. Dropped the `llm_provider` column from the migration entirely. | Add `llm_provider` column to users table. Support OpenAI, Anthropic, and potentially local models via Rig's provider-agnostic interface. | Scope creep for a hackathon deadline. The chaos agent hardcodes Anthropic. Adding provider switching is a post-MVP feature. One-column ALTER TABLE when needed. |
| **Anthropic client construction** | `Client::from_env()` reads `ANTHROPIC_API_KEY` from environment on every message. | Use `rig::providers::anthropic::ClientBuilder::new(&key).build()` with key from `BotState`. Enables per-user keys without re-reading env vars. | `ClientBuilder::new()` had compilation issues with rig-core 0.31's generic type parameters. `from_env()` works identically for single-key MVP. Switch when per-user keys ship. |
| **Wallet connection** | No wallet. Users state their portfolio amount in natural language ("I have $1000 USDC"). Agent uses stated amount as `portfolio_total_usd`. | Add inline keyboard button for wallet connect via Telegram web mini-app (WalletConnect). Read actual on-chain balances via alloy provider. Portfolio context comes from chain state, not user declaration. | Wallet integration requires a web mini-app, WalletConnect setup, and on-chain RPC calls. The policy engine and agent work identically regardless of where the portfolio number comes from. |
| **Preset picker buttons** | No inline keyboard for presets. The agent creates policies conversationally via `SetPolicyTool` based on natural language. | Optionally add preset buttons as a quick-start for users who don't want to describe preferences. Agent still fills in defaults for unstated rules. | The conversational flow is the product differentiator. Preset buttons short-circuit the NLP-to-policy demonstration. If user testing shows people want buttons, add them. |
| **Policy modification semantics** | `SetPolicyTool` only creates new policies. It does not replace, merge, or delete existing ones. In practice, `/reset` is the easiest way to start fresh during testing | Add explicit update/replace/delete tools, policy versioning, or a "make this the only active policy" path in the API | Append-only policy creation was the safest thing to ship quickly. Mutation semantics are easy to get wrong, especially when multiple active policies compose into one stricter rule set. |
| **Shared API types across binaries** | `ValidateRequest`, `ValidateContext`, `CreatePolicyRequest`, response types duplicated between `chaos_agent.rs` and `telegram_bot.rs`. | Extract to a shared module in the library crate (e.g. `src/api_types.rs`). Both binaries import from the library. | Duplication across two binaries is cheaper than a premature abstraction. Refactoring working code is fast. Refactoring broken abstractions is slow. |
| **Agent error type duplication** | `AgentError` in telegram_bot.rs mirrors `ChaosAgentError` in chaos_agent.rs. Same three variants. | Extract to shared module in library crate alongside API types. | Same rationale as above. Two binaries, identical error shape, extract when a third consumer appears. |
| **Conversation JSONB array unbounded growth** | `trim_to_recent()` keeps last 40 messages. Called after every assistant response. | Implement sliding window with summarization: when trimming, ask Claude to summarize the dropped messages into a system context note. Preserves long-term memory without token overflow. | 40 messages is sufficient for hackathon sessions. Summarization requires an extra LLM call per trim, which adds latency and cost. |
| **Tool-call persistence in conversation history** | Tool calls/results are preserved only in-memory during a live Rig turn. Persisted `conversations.messages_json` stores user text and final assistant text, but not the per-tool input/output chain even though the schema supports it | Persist `ToolCall { name, input, output }` on assistant messages and optionally rehydrate them into Rig-compatible history when resuming a session | Hackathon sessions are self-contained, `/reset` intentionally starts over, and authoritative policy/transaction outcomes already live in `policies` and `transaction_log`. The missing audit trail is real, but it does not change the visible demo behavior. |
| **Session-scoped memory, not identity-scoped memory** | Conversation state is keyed by the active Telegram session. `/reset` deactivates the current conversation and starts a new one. Nothing today loads durable memory by agent identity | Add a separate identity-linked memory layer keyed by ERC-8004 / wallet identity. Session chat can reset, while durable agent memory survives across chats and devices | The hackathon bot is optimized for testing loops, not long-lived autonomous identity. Durable memory becomes worth building when ERC-8004 identity is first-class and we want the agent to remember work across sessions. |
| **Telegram Markdown parsing** | Removed all `parse_mode(Markdown)` calls. All bot messages sent as plain text. | Re-enable Markdown with proper escaping (MarkdownV2 requires escaping of `_*[]()~>#+-=\|{}.!`). Or use HTML parse mode which has simpler escaping rules. | Telegram's Markdown parser rejects unescaped special characters. `!` in welcome messages caused `Bad Request: can't parse entities`. Plain text works everywhere with zero escaping issues. |
| **Agent loop approach** | Manual completion loop with `agent.completion()` + `.send()`. Execute tools manually between turns. Send Telegram progress messages after each tool call. | Use Rig's `Hook` trait for per-request tool call observation. Implement a hook that sends Telegram messages on each tool execution. Cleaner architecture, less manual loop code. | Rig's hook system requires holding a reference to `Bot` and `ChatId` inside the hook, which is non-trivial with Rig's ownership model. Manual loop gives us full control over the UX and was faster to build. |
| **Portfolio amount hardcoded in tool** | `ValidateTool.portfolio_total_usd` defaults to `"1000.00"`. Agent preamble tells Claude to use whatever amount the user states. | Parse the stated amount from conversation history and pass dynamically to the tool. Or add a `SetPortfolioTool` that the agent calls to update the amount. | The preamble handles it for demo purposes. Claude reads "$1000" from the user message and uses it in reasoning, even though the tool sends a fixed context value. Correct dynamic extraction needs NLP parsing. |
| **Daily spend tracking scope** | `daily_spend_so_far` is an `Arc<Mutex<f64>>` created fresh per message handler invocation. Resets to 0 on every new message. | Persist daily spend in the database. Query `transaction_log` for today's approved amounts. Pass actual cumulative spend to `ValidateContext`. | Per-session tracking is sufficient for the demo. A user sending multiple messages in one session sees correct cumulative tracking within that session. Cross-session tracking requires a DB query per validate call. |
| **Zero-knowledge API key storage** | Server operator holds `ENCRYPTION_KEY` and can technically decrypt user API keys by writing intentional code. Architecture prevents accidental exposure: `#[serde(skip_serializing)]`, no log/API surface, key lives in memory only during request. | Client-side encryption where the user encrypts their key with their own password before sending. Server never sees plaintext. User enters password each session to unlock. | True zero-knowledge requires per-session password entry, which is worse UX than pasting the key. The current model protects against database leaks (the likely threat). Protection against server operator is a policy commitment, not a technical impossibility. |
| **Sponsorship/donation flow** | Text note in `/help`: "If you enjoy using Callipsos, consider supporting compute costs." | Add an inline button linking to a donation address or payment page. Track contributions per user. | Not in the onboarding flow (adds friction). Placed in `/help` where users see it after getting value. Implementation is a button + address, trivial to add. |
| **Bot name vs username** | Display name: "CallipsosDev". Username: `@callipsos_agent_bot`. Username is permanent. | Create a production bot with a clean username when shipping to mainnet. Retire the dev bot or keep it for testing. | Display name is changeable via BotFather anytime. Username `callipsos_agent_bot` works for production anyway. |
| **Default policy scaling** | Tool description tells Claude to scale `max_transaction_amount` and `max_daily_spend` relative to portfolio size (~10% and ~10-20% respectively). Percentage-based rules are inherently portfolio-agnostic. | Build dynamic default calculation into the tool itself. Tool reads portfolio size and computes absolute values before calling the API. | Claude does the math from conversation context. A user with $10 gets ~$1 defaults, $1000 gets ~$100. No code change needed for scaling. The LLM handles the arithmetic. |
| **Agent identity is not wired into the conversation stack yet** | The bot operates as a Telegram-scoped assistant today. ERC-8004 identity exists conceptually in the project narrative, but not yet as a first-class key in the conversation, policy, or memory model | Introduce an agent identity model that links conversations, policies, transaction logs, and long-term memory to the agent's on-chain identity | Identity work was intentionally sequenced after the conversational MVP. The current bot proves the safety workflow; the next phase makes that workflow portable across sessions and on-chain personas. |
 
### Decisions we're keeping
 
| Decision | Why it's right |
|---|---|
| **Bot is API consumer, not embedded in server** | Telegram bot calls the same REST API as the chaos agent. No special internal access. Tests the real API surface. Bot and server can scale independently. |
| **Conversational onboarding, no menus** | The product differentiator is NLP-to-policy. Users type naturally, agent interprets and creates rules. Preset buttons would short-circuit the demo of this capability. |
| **Demo mode (no real funds)** | Zero friction to test. No wallet, no testnet faucet, no gas tokens needed. User states a portfolio amount and the agent simulates everything. Same policy engine, same signing, same safety guarantees. The wallet-less demo is a stronger hackathon play than half-built wallet integration. |
| **Server operator pays compute costs** | Removes the biggest adoption blocker for early testing. Cost per session is negligible (~$0.05-0.15 for Sonnet). 50 test sessions is under $10. Iterate on product before optimizing costs. |
| **AES-256-GCM encryption for stored API keys** | Column stores `base64(nonce || ciphertext)`. Random nonce per encryption means same key encrypted twice produces different ciphertext. Auth tag catches tampering. `#[serde(skip_serializing)]` prevents accidental exposure in API responses. Infrastructure exists and is tested even though MVP doesn't use it yet. |
| **`conversations` table with JSONB array (not per-message rows)** | Preserves tool call ordering within assistant turns. One read, one write per message. Atomic append via `messages_json || $1::jsonb`. No fragile reconstruction logic. Unique partial index enforces one active conversation per user. |
| **Manual agent loop over Rig's built-in `.chat()`** | `.chat()` runs the full multi-turn tool loop silently. User sees nothing for 30-40 seconds. Manual loop sends Telegram progress messages after each tool call ("Attempting: supply $100 USDC on aave-v3..." / "APPROVED (Signed by Lit Protocol)"). Perceived latency drops from 40s of silence to 3-5s between updates. Same total work, dramatically better UX. |
| **`encrypt.rs` not `crypto.rs`** | `crypto.rs` in a Web3 project is ambiguous. `encrypt.rs` is unambiguous about what the module does. |
| **`load_encryption_key()` returns Result, not panic** | Bot binary treats missing key as fatal. API server never calls it. The caller decides severity, not the utility function. Three distinct error variants: `Missing`, `InvalidHex`, `WrongLength`. |
| **Plain text Telegram messages over Markdown** | Telegram's MarkdownV2 parser requires escaping 12+ special characters. One unescaped `!` crashes the message send. Plain text works everywhere. The formatting loss is cosmetic; the content is what matters. |
---

## Post-MVP: Scaling & Production
 
_Tradeoffs that only matter at scale. Don't touch these until product-market fit hypothesis is validated._
 
| Tradeoff | What to revisit | Trigger |
|---|---|---|
| `Vec` → `HashSet` for protocol lookups (`audited_protocols: Vec<ProtocolId>`) | Allowlist exceeds ~20 entries | Protocol count grows |
| Single-crate Rust → workspace with sub-crates | Module boundaries get painful | Codebase exceeds ~5k lines |
| PostgreSQL → read replicas or caching layer | DB becomes bottleneck on validate endpoint | Sustained >1k req/s |
| Hardcoded yield sources → general yield aggregator | Users want protocols beyond Aave + Moonwell | User feedback demands it |
| Add `MaxPositionsExceeded` Violation in `policy/types` → A cap on simultaneous positions a user can have | Users want Vaults and LPs | User feedback demands it |
| Per-user Rig agent caching | Rebuilding the agent per message is wasteful at scale | Concurrent user count exceeds ~50 |
| Conversation summarization on trim | Long sessions lose early context silently | Users reference earlier conversation points that were trimmed |
| On-chain portfolio reading via alloy provider | Replace user-declared amounts with real balances | Wallet connection ships |
| Persistent daily spend tracking in DB | Cross-session spend limits don't reset correctly | Users report unexpected approvals after restart |
| Rate limiting on bot messages | Agent could spam Telegram with rapid tool calls | Aggressive agent preambles or adversarial users |
| API authentication / ownership checks | Prevent arbitrary callers from reading or mutating other users' state | The API is exposed beyond a trusted local/demo environment |
| Identity-linked durable memory | Keep agent context across `/reset`, new chats, devices, or clients | ERC-8004 identity becomes part of the product, not just the narrative |
| Persisted tool-call audit trail | Make conversation history replayable and auditable beyond final assistant text | Users or judges want to inspect how the agent reasoned through tool use after the fact |
| Signed attestation storage and retrieval | Query historical signatures/verdicts without replaying validation | Third parties need to verify past approvals or build dashboards around them |
