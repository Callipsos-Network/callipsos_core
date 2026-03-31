#![allow(clippy::too_many_arguments)]

use alloy::network::EthereumWallet;
use alloy::primitives::{Address, B256, U256};
use alloy::providers::ProviderBuilder;
use alloy::signers::local::PrivateKeySigner;
use alloy::sol;
use alloy::sol_types::SolEvent;
use anyhow::{anyhow, Context};

sol! {
    #[sol(rpc)]
    contract IIdentityRegistry {
        function register(string calldata agentURI) external returns (uint256 agentId);
        function setAgentURI(uint256 agentId, string calldata agentURI) external;
        function tokenURI(uint256 tokenId) external view returns (string memory);
        function ownerOf(uint256 tokenId) external view returns (address);
        event Transfer(address indexed from, address indexed to, uint256 indexed tokenId);
    }
}

sol! {
    #[sol(rpc)]
    contract IReputationRegistry {
        function authorizeFeedback(
            uint256 agentId,
            address clientAddress,
            uint256 nonce
        ) external;

        function giveFeedback(
            uint256 agentId,
            address clientAddress,
            int64 value,
            uint8 valueDecimals,
            string calldata endpoint,
            string calldata feedbackURI,
            bytes32 feedbackHash,
            string calldata tag
        ) external;

        function getSummary(uint256 agentId) external view returns (
            uint256 totalFeedback,
            int256 aggregateValue
        );
    }
}

pub struct AgentIdentity {
    rpc_url: String,
    private_key: String,
    identity_registry: Address,
    reputation_registry: Address,
}

impl AgentIdentity {
    pub fn new(
        rpc_url: &str,
        private_key: &str,
        identity_registry: &str,
        reputation_registry: &str,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            rpc_url: rpc_url.to_string(),
            private_key: private_key.to_string(),
            identity_registry: identity_registry.parse()?,
            reputation_registry: reputation_registry.parse()?,
        })
    }

    fn signer(&self) -> anyhow::Result<PrivateKeySigner> {
        self.private_key
            .parse()
            .context("failed to parse ERC-8004 signer private key")
    }

    fn build_provider(
        &self,
    ) -> anyhow::Result<impl alloy::providers::Provider + Clone> {
        let signer = self.signer()?;
        let wallet = EthereumWallet::from(signer);
        let url = self
            .rpc_url
            .parse()
            .context("failed to parse ERC-8004 RPC URL")?;

        Ok(ProviderBuilder::new().wallet(wallet).connect_http(url))
    }

    fn build_read_provider(
        &self,
    ) -> anyhow::Result<impl alloy::providers::Provider + Clone> {
        let url = self
            .rpc_url
            .parse()
            .context("failed to parse ERC-8004 RPC URL")?;

        Ok(ProviderBuilder::new().connect_http(url))
    }

    fn topic_to_address(topic: &B256) -> Address {
        let bytes = topic.as_slice();
        Address::from_slice(&bytes[12..])
    }

    fn topic_to_u256(topic: &B256) -> U256 {
        U256::from_be_bytes(topic.0)
    }

    fn agent_id_from_receipt(
        &self,
        receipt: &alloy::rpc::types::eth::TransactionReceipt,
    ) -> anyhow::Result<u64> {
        if let Some(decoded) = receipt.decoded_log::<IIdentityRegistry::Transfer>() {
            let token_id = decoded.data.tokenId.to::<u64>();
            return Ok(token_id);
        }

        for log in receipt.logs() {
            let topics = log.topics();
            if topics.len() != 4 {
                continue;
            }

            if topics[0] != IIdentityRegistry::Transfer::SIGNATURE_HASH {
                continue;
            }

            let from = Self::topic_to_address(&topics[1]);
            if from != Address::ZERO {
                continue;
            }

            let token_id = Self::topic_to_u256(&topics[3]).to::<u64>();
            return Ok(token_id);
        }

        Err(anyhow!(
            "registration receipt did not contain an ERC-721 mint Transfer event"
        ))
    }

    /// Register the Callipsos agent in the ERC-8004 Identity Registry.
    /// Mints an ERC-721 NFT. Returns the agentId (tokenId).
    pub async fn register_agent(&self, agent_uri: &str) -> anyhow::Result<u64> {
        let provider = self.build_provider()?;
        let registry = IIdentityRegistry::new(self.identity_registry, &provider);

        tracing::info!("Registering agent with ERC-8004 Identity Registry...");
        tracing::info!("  Registry: {}", self.identity_registry);
        tracing::info!("  URI: {}", agent_uri);

        let pending = registry.register(agent_uri.to_string()).send().await?;
        let receipt = pending.get_receipt().await?;
        let agent_id = self.agent_id_from_receipt(&receipt)?;

        tracing::info!("Agent registered! agentId: {}", agent_id);
        tracing::info!("  TX: {:?}", receipt.transaction_hash);

        Ok(agent_id)
    }

    /// Update the metadata URI for an existing ERC-8004 identity.
    pub async fn set_agent_uri(&self, agent_id: u64, agent_uri: &str) -> anyhow::Result<()> {
        let provider = self.build_provider()?;
        let registry = IIdentityRegistry::new(self.identity_registry, &provider);

        let pending = registry
            .setAgentURI(U256::from(agent_id), agent_uri.to_string())
            .send()
            .await?;
        let receipt = pending.get_receipt().await?;

        tracing::info!(
            "Updated agent URI for agentId={} in tx {:?}",
            agent_id,
            receipt.transaction_hash
        );

        Ok(())
    }

    /// Get agent's tokenURI from the Identity Registry.
    pub async fn get_agent_uri(&self, agent_id: u64) -> anyhow::Result<String> {
        let provider = self.build_read_provider()?;
        let registry = IIdentityRegistry::new(self.identity_registry, &provider);
        let result = registry.tokenURI(U256::from(agent_id)).call().await?;
        Ok(result)
    }

    /// Read the current owner of the ERC-8004 identity NFT.
    pub async fn owner_of(&self, agent_id: u64) -> anyhow::Result<Address> {
        let provider = self.build_read_provider()?;
        let registry = IIdentityRegistry::new(self.identity_registry, &provider);
        let result = registry.ownerOf(U256::from(agent_id)).call().await?;
        Ok(result)
    }

    /// Authorize a client address to give feedback for this agent.
    /// Must be called by the agent owner before giveFeedback.
    pub async fn authorize_feedback(
        &self,
        agent_id: u64,
        client_address: Address,
        nonce: u64,
    ) -> anyhow::Result<()> {
        let provider = self.build_provider()?;
        let registry = IReputationRegistry::new(self.reputation_registry, &provider);

        let pending = registry
            .authorizeFeedback(U256::from(agent_id), client_address, U256::from(nonce))
            .send()
            .await?;
        let receipt = pending.get_receipt().await?;

        tracing::info!(
            "Feedback authorized for agent {} from {} in tx {:?}",
            agent_id,
            client_address,
            receipt.transaction_hash
        );

        Ok(())
    }

    /// Submit reputation feedback after a successful policy-compliant transaction.
    pub async fn give_feedback(
        &self,
        agent_id: u64,
        client_address: Address,
        value: i64,
        tag: &str,
        endpoint: &str,
    ) -> anyhow::Result<()> {
        let provider = self.build_provider()?;
        let registry = IReputationRegistry::new(self.reputation_registry, &provider);

        let pending = registry
            .giveFeedback(
                U256::from(agent_id),
                client_address,
                value,
                0u8,
                endpoint.to_string(),
                String::new(),
                B256::ZERO,
                tag.to_string(),
            )
            .send()
            .await?;
        let receipt = pending.get_receipt().await?;

        tracing::info!(
            "Reputation feedback submitted: agent={}, value={}, tag={}, tx={:?}",
            agent_id,
            value,
            tag,
            receipt.transaction_hash
        );

        Ok(())
    }

    /// Read the agent's reputation summary from on-chain.
    /// Returns (total_feedback_count, aggregate_value).
    pub async fn get_reputation(&self, agent_id: u64) -> anyhow::Result<(u64, i64)> {
        let provider = self.build_read_provider()?;
        let registry = IReputationRegistry::new(self.reputation_registry, &provider);

        let summary = registry.getSummary(U256::from(agent_id)).call().await?;
        let aggregate_value = i64::try_from(summary.aggregateValue)
            .map_err(|_| anyhow!("aggregate reputation value does not fit in i64"))?;

        Ok((
            summary.totalFeedback.to::<u64>(),
            aggregate_value,
        ))
    }

    /// Get the signer's address (useful for demo self-feedback flows).
    pub fn signer_address(&self) -> anyhow::Result<Address> {
        Ok(self.signer()?.address())
    }
}
