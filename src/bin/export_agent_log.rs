use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use dotenvy::dotenv;
use serde::Serialize;
use serde_json::Value;
use sqlx::PgPool;

use callipsos_core::db;

#[derive(Debug, Serialize, sqlx::FromRow)]
struct ExportRow {
    request_json: Value,
    verdict: String,
    reasons_json: Value,
    reasoning_json: Option<Value>,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct AgentLogFile {
    #[serde(rename = "agentId")]
    agent_id: Option<u64>,
    #[serde(rename = "agentRegistry")]
    agent_registry: String,
    version: String,
    logs: Vec<AgentLogEntry>,
}

#[derive(Debug, Serialize)]
struct AgentLogEntry {
    timestamp: DateTime<Utc>,
    action: String,
    input: AgentLogInput,
    result: AgentLogResult,
    reasoning: Option<Value>,
    #[serde(rename = "reputationTxHash")]
    reputation_tx_hash: Option<String>,
    #[serde(rename = "chainId")]
    chain_id: u64,
}

#[derive(Debug, Serialize)]
struct AgentLogInput {
    protocol: String,
    action: String,
    asset: String,
    amount: String,
    #[serde(rename = "targetAddress")]
    target_address: String,
}

#[derive(Debug, Serialize)]
struct AgentLogResult {
    decision: String,
    #[serde(rename = "rules_passed")]
    rules_passed: usize,
    #[serde(rename = "rules_failed")]
    rules_failed: usize,
}

fn optional_env(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

async fn fetch_rows(pool: &PgPool, limit: i64) -> Result<Vec<ExportRow>> {
    let rows = sqlx::query_as::<_, ExportRow>(
        r#"
        SELECT
            request_json,
            verdict,
            reasons_json,
            reasoning_json,
            created_at
        FROM transaction_log
        ORDER BY created_at DESC
        LIMIT $1
        "#,
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

fn count_rule_outcomes(reasons_json: &Value) -> (usize, usize) {
    let mut passed = 0;
    let mut failed = 0;

    if let Some(results) = reasons_json.as_array() {
        for result in results {
            match result.get("outcome").and_then(Value::as_str) {
                Some("pass") => passed += 1,
                Some("fail") => failed += 1,
                _ => {}
            }
        }
    }

    (passed, failed)
}

fn entry_from_row(row: ExportRow, chain_id: u64) -> AgentLogEntry {
    let protocol = row
        .request_json
        .get("target_protocol")
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_string();
    let action = row
        .request_json
        .get("action")
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_string();
    let asset = row
        .request_json
        .get("asset")
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_string();
    let amount = row
        .request_json
        .get("amount_usd")
        .and_then(Value::as_str)
        .unwrap_or("0.00")
        .to_string();
    let target_address = row
        .request_json
        .get("target_address")
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_string();

    let (rules_passed, rules_failed) = count_rule_outcomes(&row.reasons_json);

    AgentLogEntry {
        timestamp: row.created_at,
        action: "validate_transaction".to_string(),
        input: AgentLogInput {
            protocol,
            action,
            asset,
            amount,
            target_address,
        },
        result: AgentLogResult {
            decision: row.verdict,
            rules_passed,
            rules_failed,
        },
        reasoning: row.reasoning_json,
        reputation_tx_hash: None,
        chain_id,
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let database_url = std::env::var("DATABASE_URL")
        .context("DATABASE_URL must be set")?;
    let pool = db::connect(&database_url).await?;
    db::migrate(&pool).await?;

    let limit = optional_env("AGENT_LOG_LIMIT")
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(10);
    let chain_id = optional_env("ERC8004_CHAIN_ID")
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(84532);
    let agent_id = optional_env("ERC8004_AGENT_ID")
        .and_then(|value| value.parse::<u64>().ok());
    let identity_registry = optional_env("ERC8004_IDENTITY_REGISTRY")
        .unwrap_or_else(|| "0x8004A818BFB912233c491871b3d84c89A494BD9e".to_string());

    let rows = fetch_rows(&pool, limit).await?;
    let logs = rows
        .into_iter()
        .map(|row| entry_from_row(row, chain_id))
        .collect();

    let agent_log = AgentLogFile {
        agent_id,
        agent_registry: format!("eip155:{chain_id}:{identity_registry}"),
        version: "1.0.0".to_string(),
        logs,
    };

    let output = serde_json::to_string_pretty(&agent_log)?;
    let output_path = PathBuf::from("agent_log.json");
    fs::write(&output_path, output)
        .with_context(|| format!("failed to write {}", output_path.display()))?;

    println!("Wrote {}", output_path.display());

    Ok(())
}
