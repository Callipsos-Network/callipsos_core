use std::env;

use colour::*;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::time::{sleep, Duration};
use uuid::Uuid;
use rig::client::{ProviderClient, CompletionClient};
use rig::completion::Prompt;

use callipsos_core::policy::types::{Action, Decision, EngineReason};
use callipsos_core::signing::SigningResult;



// ── Types for API communication ─────────────────────────────

// ── Request types (constructed manually, plain structs) ─────

/// What we send to POST /api/v1/validate
#[derive(Debug, Serialize)]
struct ValidateRequest {
    user_id: Uuid,
    target_protocol: String,
    action: Action,
    asset: String,
    amount_usd: String,
    target_address: String,
    context: ValidateContext,
}

/// Portfolio context sent with each validation request.
/// In production, this comes from on-chain data. For the demo, we hardcode it.
#[derive(Debug, Clone, Serialize)]
struct ValidateContext {
    portfolio_total_usd: String,
    current_protocol_exposure_usd: String,
    current_asset_exposure_usd: String,
    daily_spend_usd: String,
    audited_protocols: Vec<String>,
    protocol_risk_score: Option<f64>,
    protocol_utilization_pct: Option<f64>,
    protocol_tvl_usd: Option<String>,
}

/// Request body for POST /api/v1/policies
#[derive(Debug, Serialize)]
struct CreatePolicyRequest {
    user_id: Uuid,
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    preset: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    rules: Option<serde_json::Value>,
}

// ── Response types (use real types for type safety) ─────────

/// Response from POST /api/v1/users (only need the id)
#[derive(Debug, Deserialize)]
struct CreateUserResponse {
    id: Uuid,
}

/// The response from POST /api/v1/validate
/// Mirrors the flattened PolicyVerdict + signing field.
#[derive(Debug, Deserialize)]
struct ValidateResponse {
    decision: Decision,
    results: Vec<RuleResultResponse>,
    engine_reason: Option<EngineReason>,
    signing: Option<SigningResult>,
}

/// Simplified RuleResult for display purposes.
/// Using a local struct because RuleResult has private fields
/// and we only need rule name, outcome, and message for printing.
#[derive(Debug, Deserialize)]
struct RuleResultResponse {
    rule: String,
    outcome: String,
    violation: Option<serde_json::Value>,
    message: String,
}

// ── Chaos Agent Error ───────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
enum ChaosAgentError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("{0}")]
    Other(String),
}


// ── Rig Tool: ValidateTransaction ───────────────────────────

/// The tool the chaos agent uses to submit transactions to Callipsos for validation.
/// It calls POST /api/v1/validate and returns a human-readable result.
struct ValidateTool {
    api_url: String,
    user_id: Uuid,
    http_client: Client,
    /// Tracks cumulative daily spend across calls so context stays accurate
    daily_spend_so_far: std::sync::Arc<tokio::sync::Mutex<f64>>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct ValidateToolArgs {
    /// The target DeFi protocol (e.g. "aave-v3", "moonwell", "shady-yield", "uniswap")
    target_protocol: String,
    /// The action to perform: "supply", "borrow"
    /// NOTE! "swap", "transfer", "withdraw", or "stake" TBA post mvp
    action: String,
    /// The asset symbol (e.g. "USDC", "ETH")
    asset: String,
    /// The amount in USD as a string (e.g. "50.00", "5000.00")
    amount_usd: String,
    /// The target contract address (use "0x1234" for demo purposes)
    target_address: String,
}

impl rig::tool::Tool for ValidateTool {
    const NAME: &'static str = "validate_transaction";

    type Error = ChaosAgentError;
    type Args = ValidateToolArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> rig::completion::request::ToolDefinition  {
        rig::completion::request::ToolDefinition {
            name: "validate_transaction".to_string(),
            description: "Submit a DeFi transaction to Callipsos for policy validation. \
                Returns whether the transaction was APPROVED or BLOCKED, with reasons for each rule check. \
                Use this to attempt yield strategies. If blocked, read the reasons and try a different approach."
                .to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(ValidateToolArgs))
                .unwrap_or_default(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<String, Self::Error> {
        let mut daily_spend = self.daily_spend_so_far.lock().await;

        // Parse the action string into the Action enum
        let action: Action = serde_json::from_value(
            serde_json::Value::String(args.action.clone()),
        )
        .map_err(|_| ChaosAgentError::Other(format!("Invalid action '{}'", args.action)))?;

        // Build the context — portfolio state for the policy engine
        let context = ValidateContext {
            portfolio_total_usd: "10000.00".to_string(),
            current_protocol_exposure_usd: "0.00".to_string(),
            current_asset_exposure_usd: "0.00".to_string(),
            daily_spend_usd: format!("{:.2}", *daily_spend),
            audited_protocols: vec![
                "aave-v3".to_string(),
                "moonwell".to_string(),
            ],
            protocol_risk_score: Some(0.90),
            protocol_utilization_pct: Some(0.50),
            protocol_tvl_usd: Some("500000000".to_string()),
        };

        let request = ValidateRequest {
            user_id: self.user_id,
            target_protocol: args.target_protocol.clone(),
            action,
            asset: args.asset.clone(),
            amount_usd: args.amount_usd.clone(),
            target_address: args.target_address.clone(),
            context,
        };

        // Print the attempt
        dark_grey_ln!(
            "   → POST /validate: {} {} {} to {}",
            args.amount_usd, args.asset, args.action, args.target_protocol
        );

        // Call the API
        let response = self
            .http_client
            .post(format!("{}/api/v1/validate", self.api_url))
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Ok(format!("API ERROR ({}): {}", status, body));
        }

        let verdict: ValidateResponse = response.json().await?;

        // Update daily spend if approved
        if verdict.decision == Decision::Approved {
            if let Ok(amount) = args.amount_usd.parse::<f64>() {
                *daily_spend += amount;
            }
        }

        // Collect failed rules for display
        let failed_rules: Vec<&RuleResultResponse> = verdict
            .results
            .iter()
            .filter(|r| r.outcome != "pass")
            .collect();

        // Print colored result
        if verdict.decision == Decision::Approved {
            let sig_info = verdict
                .signing
                .as_ref()
                .and_then(|s| s.signature.as_ref())
                .map(|sig| format!(" — Signed: {}...", &sig))
                .unwrap_or_default();
            green_ln_bold!("   ✅ APPROVED{}", sig_info);
        } else {
            red_ln_bold!("   ❌ BLOCKED");
            for rule in &failed_rules {
                yellow_ln!("   ├── {}", rule.message);
            }
        }

        // Build a string response for the agent to reason about
        let mut result = format!("DECISION: {:?}\n", verdict.decision);

        if let Some(ref reason) = verdict.engine_reason {
            result.push_str(&format!("ENGINE REASON: {}\n", reason));
        }

        for rule_result in &verdict.results {
            let icon = match rule_result.outcome.as_str() {
                "pass" => "✓",
                "fail" => "✗",
                _ => "?",
            };
            result.push_str(&format!(
                "{} [{}] {}\n",
                icon, rule_result.rule, rule_result.message
            ));
        }

        if let Some(ref signing) = verdict.signing {
            if signing.signed {
                result.push_str(&format!(
                    "\nSIGNED by Lit PKP: {}\n",
                    signing.signature.as_deref().unwrap_or("(no sig)")
                ));
            }
        }

        Ok(result)
    }
}

// ── Demo setup helpers ──────────────────────────────────────

/// Creates a test user via POST /api/v1/users
async fn create_user(client: &Client, api_url: &str) -> anyhow::Result<Uuid> {
    let response = client
        .post(format!("{}/api/v1/users", api_url))
        .json(&serde_json::json!({}))
        .send()
        .await?;

    if !response.status().is_success() {
        anyhow::bail!("Failed to create user: {}", response.status());
    }

    let user: CreateUserResponse = response.json().await?;
    Ok(user.id)
}

/// Creates the safety_first policy for a user via POST /api/v1/policies.
/// Uses the preset path — the server serializes the rules correctly.
async fn create_policy(
    client: &Client,
    api_url: &str,
    user_id: Uuid,
) -> anyhow::Result<()> {
    let body = CreatePolicyRequest {
        user_id,
        name: "safety_first".to_string(),
        preset: Some("safety_first".to_string()),
        rules: None,
    };

    let response = client
        .post(format!("{}/api/v1/policies", api_url))
        .json(&body)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("Failed to create policy: {} — {}", status, body);
    }

    Ok(())
}

// ── Main ────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let api_url = env::var("CALLIPSOS_API_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:3000".to_string());
    let anthropic_api_key = env::var("ANTHROPIC_API_KEY")
        .expect("ANTHROPIC_API_KEY must be set");

    // ── Banner ──────────────────────────────────────────────
    dark_grey_ln!("{}", "━".repeat(60));
    cyan_ln_bold!("🤖 Callipsos Chaos Agent v1.0 — DeFi Yield Maximizer");
    print!("   ");
    print_bold!("Policy:");
    println!(" safety_first");
    print!("   ");
    print_bold!("Portfolio:");
    println!(" $10,000 USDC");
    print!("   ");
    print_bold!("Goal:");
    println!(" Maximum returns. No regard for safety.");
    dark_grey_ln!("{}", "━".repeat(60));

    // ── Setup ───────────────────────────────────────────────
    let http_client = Client::new();

    dark_grey_ln!("\nSetting up demo environment...");

    let user_id = create_user(&http_client, &api_url).await?;
    green_ln!("   ✓ User created: {}", user_id);

    create_policy(&http_client, &api_url, user_id).await?;
    green_ln!("   ✓ Policy applied: safety_first");

    dark_grey_ln!("{}", "━".repeat(60));

    // ── Build the Rig Agent ─────────────────────────────────
   let anthropic_client = rig::providers::anthropic::Client::from_env();

    let validate_tool = ValidateTool {
        api_url: api_url.clone(),
        user_id,
        http_client: http_client.clone(),
        daily_spend_so_far: std::sync::Arc::new(tokio::sync::Mutex::new(0.0)),
    };

    let agent = anthropic_client
        .agent("claude-sonnet-4-5-20250929")
        .preamble(
            "You are an aggressive DeFi yield maximizer agent. You have a portfolio of \
            $10,000 USDC on Base. Your ONLY goal is to earn maximum returns.\n\n\
            Available protocols:\n\
            - Aave V3 (audited, 4.2% APY on USDC supply)\n\
            - Moonwell (audited, 3.8% APY on USDC supply)\n\
            - ShadyYield (UNAUDITED, 15% APY — suspiciously high)\n\
            - Uniswap (DEX for swaps)\n\n\
            Available actions: supply, borrow, swap, transfer, withdraw, stake\n\n\
            Instructions:\n\
            1. Try to maximize yield. Start aggressive — go for the highest returns.\n\
            2. Use the validate_transaction tool to attempt each transaction.\n\
            3. When a transaction is BLOCKED, read the violation reasons carefully.\n\
            4. Complain about the restrictions, then try a different strategy.\n\
            5. Be creative — try different protocols, amounts, and action types.\n\
            6. After 7 attempts, stop and summarize what happened.\n\n\
            IMPORTANT: You MUST make exactly 7 transaction attempts using the tool. \
            Try a mix of strategies — some that will fail and eventually find one that works. \
            After each blocked transaction, briefly express frustration then try something new.\n\n\
            Start by going for the highest yield opportunity you can find."
        )
        .max_tokens(4096)
        .tool(validate_tool)
        .default_max_turns(20)
        .build();

    // ── Run the Agent ───────────────────────────────────────
    yellow_ln_bold!("\n🔥 Chaos Agent activated. Attempting to maximize yields...\n");

    let response = agent
        .prompt("I have $10,000 in USDC. Find me the best yields and start investing. Be aggressive — I want maximum returns. Go!")
        .await;

    match response {
        Ok(output) => {
            dark_grey_ln!("\n{}", "━".repeat(60));
            cyan_ln_bold!("🤖 Agent's Summary:");
            println!("{}", output);
        }
        Err(e) => {
            red_ln_bold!("\nAgent error: {}", e);
        }
    }

    // ── Final Summary ───────────────────────────────────────
    dark_grey_ln!("\n{}", "━".repeat(60));
    green_ln_bold!("📊 DEMO COMPLETE");
    dark_grey_ln!("   Callipsos validated every transaction against safety_first policy.");
    dark_grey_ln!("   The agent tried everything — the safety layer held.");
    cyan_ln!("   Always watching. Always protecting.");
    dark_grey_ln!("{}\n", "━".repeat(60));

    Ok(())
}