use casper_types::{
    addressable_entity::NamedKeys, execution::ExecutionResult, CLValue, DeployHash, Digest,
    HashAddr, Key, StoredValue, URef,
};

use crate::error::ToolkitError;
use crate::rpc::id_generator::JsonRpcIdGenerator;

pub const DEFAULT_MAINNET_RPC_ENDPOINT: &str = "https://mainnet.casper-node.xyz/rpc";
pub const DEFAULT_TESTNET_RPC_ENDPOINT: &str = "https://testnet.casper-node.xyz/rpc";

pub struct CasperClient {
    rpc_endpoint: String,
    id_generator: JsonRpcIdGenerator,
}

impl CasperClient {
    pub fn new(rpc_endpoint: &str) -> Self {
        Self {
            rpc_endpoint: rpc_endpoint.to_string(),
            id_generator: JsonRpcIdGenerator::default(),
        }
    }

    pub fn default_mainnet() -> Self {
        Self::new(DEFAULT_MAINNET_RPC_ENDPOINT)
    }

    pub fn default_testnet() -> Self {
        Self::new(DEFAULT_TESTNET_RPC_ENDPOINT)
    }

    // Fetch latest state root hash.
    pub(crate) async fn get_state_root_hash(&self) -> Result<Digest, ToolkitError> {
        // No block given means the latest available.
        let block_identifier = None;

        // Common parameters.
        let rpc_id = self.id_generator.next_id().into();
        let verbosity = casper_client::Verbosity::Low;

        let response = casper_client::get_state_root_hash(
            rpc_id,
            &self.rpc_endpoint,
            verbosity,
            block_identifier,
        )
        .await?;

        match response.result.state_root_hash {
            Some(v) => Ok(v),
            None => Err(ToolkitError::UnexpectedError {
                context: "empty state root hash".into(),
            }),
        }
    }

    async fn query_global_state(
        &self,
        state_root_hash: Digest,
        key: Key,
        path: Vec<String>,
    ) -> Result<StoredValue, ToolkitError> {
        // Wrap state root hash.
        let global_state_identifier =
            casper_client::rpcs::GlobalStateIdentifier::StateRootHash(state_root_hash);

        // Common parameters.
        let rpc_id = self.id_generator.next_id().into();
        let verbosity = casper_client::Verbosity::Low;

        let response = casper_client::query_global_state(
            rpc_id,
            &self.rpc_endpoint,
            verbosity,
            global_state_identifier,
            key,
            path,
        )
        .await?;
        let stored_value = response.result.stored_value;

        Ok(stored_value)
    }

    pub(crate) async fn get_contract_named_keys(
        &self,
        contract_hash: HashAddr,
    ) -> Result<NamedKeys, ToolkitError> {
        // Fetch latest state root hash.
        let state_root_hash = self.get_state_root_hash().await?;

        // Contract is stored directly at given hash.
        let key = Key::Hash(contract_hash);
        let path = vec![];

        let stored_value = self.query_global_state(state_root_hash, key, path).await?;
        match stored_value {
            StoredValue::Contract(v) => Ok(v.named_keys().clone()),
            _ => Err(ToolkitError::UnexpectedStoredValueType {
                expected_type: "contract",
            }),
        }
    }

    pub(crate) async fn get_stored_clvalue(
        &self,
        uref: &casper_types::URef,
    ) -> Result<CLValue, ToolkitError> {
        // Fetch latest state root hash.
        let state_root_hash = self.get_state_root_hash().await?;

        // Build uref key.
        let key = Key::URef(*uref);
        let path = vec![];

        let stored_value = self.query_global_state(state_root_hash, key, path).await?;
        match stored_value {
            StoredValue::CLValue(v) => Ok(v),
            _ => Err(ToolkitError::UnexpectedStoredValueType {
                expected_type: "clvalue",
            }),
        }
    }

    pub(crate) async fn get_stored_clvalue_from_dict(
        &self,
        dictionary_seed_uref: &URef,
        dictionary_item_key: &str,
    ) -> Result<CLValue, ToolkitError> {
        // Fetch latest state root hash.
        let state_root_hash = self.get_state_root_hash().await?;

        // Build dictionary item identifier.
        let dictionary_item_key = dictionary_item_key.to_string();
        let dictionary_item_identifier =
            casper_client::rpcs::DictionaryItemIdentifier::new_from_seed_uref(
                *dictionary_seed_uref,
                dictionary_item_key,
            );

        // Common parameters.
        let rpc_id = self.id_generator.next_id().into();
        let verbosity = casper_client::Verbosity::Low;

        let response = casper_client::get_dictionary_item(
            rpc_id,
            &self.rpc_endpoint,
            verbosity,
            state_root_hash,
            dictionary_item_identifier,
        )
        .await?;
        let stored_value = response.result.stored_value;

        match stored_value {
            StoredValue::CLValue(v) => Ok(v),
            _ => Err(ToolkitError::UnexpectedStoredValueType {
                expected_type: "clvalue",
            }),
        }
    }

    pub(crate) async fn get_deploy_result(
        &self,
        deploy_hash: DeployHash,
    ) -> Result<ExecutionResult, ToolkitError> {
        // Approvals originally received by the node are okay.
        let finalized_approvals = false;

        // Common parameters.
        let rpc_id = self.id_generator.next_id().into();
        let verbosity = casper_client::Verbosity::Low;

        let response = casper_client::get_deploy(
            rpc_id,
            &self.rpc_endpoint,
            verbosity,
            deploy_hash,
            finalized_approvals,
        )
        .await?;
        response
            .result
            .execution_info
            .expect("TODO")
            .execution_result
            .ok_or(ToolkitError::UnexpectedError {
                context: "".to_string(),
            })
    }
}
