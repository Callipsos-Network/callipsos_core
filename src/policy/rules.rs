use serde::{Deserialize, Serialize};

use crate::policy::types::{
    Action, BasisPoints, EvaluationContext, Money, ProtocolId, RiskScore, RuleId, RuleResult,
    TransactionRequest,
};

// ── PolicyRule ──────────────────────────────────────────────

/// Each variant carries its threshold. The engine iterates a `Vec<PolicyRule>`
/// and calls `evaluate()` on each one.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PolicyRule {
    MaxTransactionAmount(Money),
    MaxPercentPerProtocol(BasisPoints),
    MaxPercentPerAsset(BasisPoints),
    OnlyAuditedProtocols,
    AllowedProtocols(Vec<ProtocolId>),
    BlockedActions(Vec<Action>),
    MaxDailySpend(Money),
    MinRiskScore(RiskScore),
    MaxProtocolUtilization(BasisPoints),
    MinProtocolTvl(Money),
}

impl PolicyRule {
    /// Evaluate this single rule against a transaction request and its context.
    /// Returns a `RuleResult` indicating pass, fail, or indeterminate.
    pub fn evaluate(
        &self,
        _request: &TransactionRequest,
        _context: &EvaluationContext,
    ) -> RuleResult {
        todo!("implement rule evaluation — tests come first")
    }

    /// Returns the `RuleId` for this rule variant.
    pub fn id(&self) -> RuleId {
        match self {
            PolicyRule::MaxTransactionAmount(_) => RuleId::MaxTransactionAmount,
            PolicyRule::MaxPercentPerProtocol(_) => RuleId::MaxPercentPerProtocol,
            PolicyRule::MaxPercentPerAsset(_) => RuleId::MaxPercentPerAsset,
            PolicyRule::OnlyAuditedProtocols => RuleId::OnlyAuditedProtocols,
            PolicyRule::AllowedProtocols(_) => RuleId::AllowedProtocols,
            PolicyRule::BlockedActions(_) => RuleId::BlockedActions,
            PolicyRule::MaxDailySpend(_) => RuleId::MaxDailySpend,
            PolicyRule::MinRiskScore(_) => RuleId::MinRiskScore,
            PolicyRule::MaxProtocolUtilization(_) => RuleId::MaxProtocolUtilization,
            PolicyRule::MinProtocolTvl(_) => RuleId::MinProtocolTvl,
        }
    }
}