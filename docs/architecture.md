# Callipsos Architecture

This document is the technical and product architecture companion to the main [`README.md`](../README.md) and [`design-tradeoffs.md`](../design-tradeoffs.md).

- [`README.md`](../README.md): what exists today, how to run it, API surface, demo flow
- [`design-tradeoffs.md`](../design-tradeoffs.md): MVP compromises, production hardening path, deferred work
- [`docs/assets/Callipsos_Network_Pitch_Deck.pdf`](./assets/Callipsos_Network_Pitch_Deck.pdf): business framing, market narrative, and fundraising context

## What Callipsos Is

Callipsos is a safety and trust layer for autonomous agents that move capital.

At the MVP stage, it is a DeFi policy engine plus cryptographic signing gate:
- a user describes constraints in plain English
- an agent converts those constraints into structured policy rules
- each proposed transaction is evaluated against those rules
- approved transactions can be signed by a Lit PKP
- blocked transactions cannot reach the signing layer

That is the starting point, not the full destination.

The long-term vision is broader:
- a standard safety layer for autonomous finance
- a trust fabric for agent-to-agent commerce
- a modular control plane that can sit between any agent model and any capital-moving backend
- a cross-agent intelligence layer that learns from malicious patterns and propagates those signals across the network

## Current MVP

Today, the shipped architecture is:

1. User interface
- Telegram bot built with `teloxide`
- conversational onboarding and policy creation
- live progress updates as the agent thinks and uses tools

2. Agent layer
- Claude via Rig
- tool calling for:
  - `set_policy`
  - `validate_transaction`

3. Policy layer
- pure Rust policy engine
- deterministic rule evaluation
- fail-closed behavior when rules fail or cannot be evaluated
- reasoning audit rules alongside action and protocol rules

4. Signing layer
- Lit Protocol Chipotle integration
- approved verdicts can be signed by a PKP
- blocked verdicts never enter the signing path

5. Persistence layer
- PostgreSQL for:
  - users
  - policies
  - transaction logs
  - conversations

6. Identity layer
- ERC-8004 identity registration on Base Sepolia
- current live agent identity:
  - `agentId = 3196`
- reputation publishing is intentionally deferred until the proper FeedbackAuth flow is integrated

## Big Vision

Callipsos evolves from a gatekeeper into a full agent safety infrastructure layer.

The broad direction is:
- from "approve or block this single transaction"
- to "prove this agent is operating inside verifiable constraints"

That shift matters because it changes the trust model:
- users do not only trust the backend
- counterparties do not only trust Callipsos as a company
- agents do not need a high-latency round trip for every action forever
- the constraint system itself becomes part of the product

## Six Layers of Defense

The long-term architecture is organized into six layers. They are designed to strengthen independently, so the system becomes safer even if all layers do not ship at once.

### Layer 1: Policy Engine

Purpose:
- translate user-defined rules and limits into enforceable constraints

Status:
- live

What it does:
- evaluates transaction intent against structured policy rules
- returns detailed pass/fail results for every rule
- includes both transaction safety and reasoning quality rules

Why it matters:
- this is the judgment core of Callipsos
- it is intentionally pure Rust so it is deterministic, testable, and portable

### Layer 2: Calldata Decoding

Purpose:
- verify what the transaction actually does, not what the caller claims it does

Status:
- next major step before handling real user funds

What it does:
- decode raw calldata with alloy
- derive actual target contract, function, token flow, and amounts
- verify declared intent matches real execution intent

Why it matters:
- without calldata decoding, the system still trusts caller-declared transaction intent
- that is acceptable for an MVP demo but not acceptable for live capital

### Layer 3: Transaction Simulation

Purpose:
- dry-run higher-risk or higher-value transactions before signing

Status:
- planned

What it does:
- fork-simulate on chain state before execution
- predict token flows, reverts, liquidity issues, and position impact
- confirm that the transaction outcome matches expectations before Callipsos signs

Why it matters:
- policy compliance is necessary but not sufficient
- an approved transaction can still be strategically harmful if the state outcome is bad

### Layer 4: Cryptographic Attestation

Purpose:
- produce signed, verifiable proof that a transaction passed policy checks

Status:
- partially live

What is live:
- Lit PKP signing for approved verdicts

What remains:
- attestation retrieval
- stronger artifact persistence
- richer third-party verification paths

Why it matters:
- it turns "the backend said this was okay" into a cryptographic artifact
- this is the bridge between policy reasoning and verifiable trust

### Layer 5: Behavioural Analysis

Purpose:
- detect bad agents by pattern, not just by single-transaction rule failure

Status:
- planned

What it does:
- analyze sequences of actions across time
- detect suspicious timing, unsafe target patterns, churn, or strategy drift
- flag sabotage-like behavior that still fits inside allowed actions

Why it matters:
- a compromised or poor-quality agent can still make harmful decisions inside an allowed envelope
- this layer addresses behavior quality, not just static rule conformance

### Layer 6: Cross-Agent Intelligence

Purpose:
- promote network-wide trust and threat learning

Status:
- long-term vision

What it does:
- share threat signals across agents and deployments
- propagate knowledge of malicious protocols, unsafe patterns, compromised delegates, and emergent attack surfaces
- turn isolated safety incidents into collective network defense

Why it matters:
- the category-defining opportunity is not only local validation
- it is becoming the trust layer agents use to safely transact with each other

## Design Principles

### Pure Policy Core

The policy engine is intentionally:
- deterministic
- side-effect free
- portable

That enables several future directions:
- server-side validation
- guest execution inside a zkVM such as RISC Zero
- replayable verification for audits
- possible proof generation for third-party verification

### Modularity

Callipsos is not tied to one model vendor or one agent framework.

The intended shape is:
- user can bring their own agent
- user can bring their own model stack
- Callipsos provides the safety, validation, and trust layer underneath

That means compatibility should eventually extend across:
- Claude
- OpenAI-based agents
- Gemini-based agents
- framework-specific bots
- custom proprietary agents

The same principle applies to execution environments:
- DeFi agents
- wallet-connected assistants
- custodial or semi-custodial finance workflows
- eventually Web2 financial assistants as well

## Proof of Constraint

Today:
- the backend is a gatekeeper
- agent proposes a transaction
- Callipsos evaluates
- Lit signs only if approved

Longer term:
- Callipsos can evolve toward proof-of-constraint architectures
- the goal is not only "server approves"
- the goal is "agent acts within cryptographically bounded authority"

That future might include:
- constrained session keys
- delegated sub-keys
- attested policy envelopes
- verifiable claims that an agent could not exceed certain bounds even if compromised

This is strategically important because:
- it reduces hot-path latency for autonomous agents
- it improves compromise resistance
- it creates machine-verifiable trust for counterparties

## ZK-Verifiable Policy Execution

Because the policy engine is pure Rust, it is a strong candidate for execution inside a zkVM.

Potential future flow:
1. transaction intent is normalized
2. policy engine runs inside a zkVM guest
3. output is:
   - verdict
   - proof
4. the proof can be verified without rerunning the full engine in a trusted environment

Why that matters:
- third parties can verify the decision
- protocols can integrate with Callipsos without trusting the operator
- attestation becomes stronger than a signed statement alone

This is especially relevant for:
- inter-agent commerce
- protocol-level trust integrations
- regulated or institutional settings where independent verification matters

## Private Policies

Another long-term design goal is private policy verification.

User need:
- some users will not want to expose their full policy configuration
- the policy itself can reveal strategy, wealth assumptions, or operational preferences

Target property:
- Callipsos can verify whether a transaction satisfies a policy without exposing the raw policy itself to all observers

This could evolve through:
- encrypted policy storage with controlled execution
- trusted execution
- proof systems that show compliance without full policy disclosure

The architectural principle is:
- policy visibility should eventually be separable from policy enforceability

## Commitment-Based Execution

Single-step approval is enough for the MVP.
It is not enough for complex strategies.

As agents become more capable, Callipsos should support commitment-based execution for multi-step plans.

Concept:
- agent proposes a plan, not just one transaction
- authority is released step by step
- after each step, Callipsos checks actual on-chain state before allowing the next step

Why this matters:
- reduces blast radius for complex workflows
- allows revocation between steps
- prevents stale-context approvals in a rapidly changing market

A future execution state machine might look like:
- `Committed`
- `Active`
- `Settled`
- `Revoked`

This is the right complement to the policy engine because it turns validation from a one-time gate into a continuous control loop.

## Delegated Agents and Agent-to-Agent Commerce

The long-term Web3 opportunity is not merely "agents share yield strategies."
It is that agents hire other agents to do specialized work safely.

Example delegation model:
- user has a parent Callipsos-protected agent
- parent agent hires a specialized rebalancing, monitoring, tax, or execution agent
- delegated capability inherits the user’s constraints and adds tighter ones

Critical controls for delegated execution:
- narrower monetary cap
- shorter expiry
- fewer allowed operations
- narrower protocol set
- explicit delegate allowlists
- delegated exposure caps

Why this matters:
- the policy layer travels with the money
- every delegate acts inside inherited bounds
- payment and execution can eventually be tied together through verifiable results

This is where Callipsos starts looking like infrastructure, not just a product feature.

## Cross-Agent Threat Intelligence Standard

The category-defining ambition is for Callipsos to evolve into a trust and threat-intelligence standard for autonomous finance.

That means:
- one agent discovers malicious behavior
- the signal can become useful to other agents
- safety knowledge compounds across the network

This is the architecture shift from:
- "my backend validates my user’s transactions"
to:
- "the ecosystem shares machine-usable safety signals"

## Beyond Web3

The first distribution wedge is DeFi.
That is not the final boundary.

The same architecture can extend into Web2 financial agents:
- stock investing assistants
- forex trading assistants
- treasury management copilots
- cross-platform personal finance agents

The common pattern is the same:
- autonomous system
- user-defined risk boundaries
- capital movement
- need for verifiable safety and auditability

## Onramping and Offramping

For Web3 specifically, a future user experience should not require the user to separately manage wallet funding complexity.

Long-term product direction:
- user provides fiat directly
- agent can on-ramp capital into crypto rails
- invest according to policy
- unwind and off-ramp back to fiat when needed

That turns Callipsos from a DeFi safety tool into a broader autonomous capital allocation interface.

## Business Positioning

Callipsos is not competing only as another wallet tool or agent wrapper.

The intended wedge is:
- wallet providers give agents keys
- frameworks give agents cognition
- Callipsos gives agents judgment

Revenue can evolve in phases:
- trust-building and aligned monetization first
- developer or framework infrastructure second
- enterprise and network intelligence later

That aligns with the product strategy:
- start with a sharp safety use case
- expand into infrastructure once trust is established

## Near-Term Priorities

Before mainnet or real-funds deployment, the highest-priority architecture steps are:

1. Calldata decoding
2. Authoritative on-chain or server-side evaluation context
3. Action-aware rule semantics
4. Signing artifact persistence and attestation retrieval
5. Simulation and post-state verification
6. FeedbackAuth-based ERC-8004 reputation flow

## Summary

The MVP proves that:
- natural-language policy creation works
- pure Rust policy evaluation works
- reasoning-aware safety rules work
- cryptographic signing gates work
- on-chain agent identity works

The long-term vision is larger:
- agent safety standard
- cross-agent trust layer
- modular infrastructure for autonomous finance
- eventually a bridge across both Web3 and Web2 capital-moving agents
