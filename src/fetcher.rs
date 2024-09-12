pub use casper_event_standard::Schemas;

use casper_types::{
    bytesrepr::FromBytes,
    execution::execution_result_v1::TransformKindV1::WriteCLValue,
    execution::{ExecutionResult, ExecutionResultV1, TransformKindV2},
    DeployHash, StoredValue,
};

use crate::error::ToolkitError;
use crate::event::Event;
use crate::metadata::CesMetadataRef;
use crate::parser::{parse_event, parse_raw_event_name_and_data};
use crate::rpc::client::CasperClient;
use crate::utils::parse_hash;

pub struct Fetcher {
    pub client: CasperClient,
    // Metdadata
    pub ces_metadata: CesMetadataRef,
}

impl Fetcher {
    pub async fn fetch_events_count(&self) -> Result<u32, ToolkitError> {
        let events_length_uref = &self.ces_metadata.events_length;
        let events_length_value = self.client.get_stored_clvalue(events_length_uref).await?;
        let events_length: u32 = events_length_value
            .into_t()
            .map_err(|e| ToolkitError::InvalidCLValue(e.to_string()))?;

        Ok(events_length)
    }

    pub async fn fetch_schema(&self) -> Result<Schemas, ToolkitError> {
        let events_schema_uref = &self.ces_metadata.events_schema;
        let events_schema_value = self.client.get_stored_clvalue(events_schema_uref).await?;
        let events_schema: Schemas = events_schema_value
            .into_t()
            .map_err(|e| ToolkitError::InvalidCLValue(e.to_string()))?;

        Ok(events_schema)
    }

    pub async fn fetch_event(
        &self,
        id: u32,
        event_schema: &Schemas,
    ) -> Result<Event, ToolkitError> {
        let events_data_uref = &self.ces_metadata.events_data;
        let event_value = self
            .client
            .get_stored_clvalue_from_dict(events_data_uref, &id.to_string())
            .await?;
        let event_value_bytes = event_value.inner_bytes();
        let (event_name, event_data) = parse_raw_event_name_and_data(event_value_bytes)?;

        // Parse dynamic event data.
        let dynamic_event = parse_event(event_name, &event_data, event_schema)?;

        Ok(dynamic_event)
    }

    pub async fn fetch_events_from_deploy(
        &self,
        deploy_hash: &str,
        event_schema: &Schemas,
    ) -> Result<Vec<Event>, ToolkitError> {
        // Build deploy hash.
        let contract_hash_bytes = parse_hash(deploy_hash)?;
        let deploy_hash = DeployHash::new(contract_hash_bytes.into());

        let execution_result = self.client.get_deploy_result(deploy_hash).await?;
        match execution_result {
            ExecutionResult::V1(execution_result_v1) => match execution_result_v1 {
                ExecutionResultV1::Failure { error_message, .. } => {
                    Err(ToolkitError::FailedDeployError(error_message.to_string()))
                }
                ExecutionResultV1::Success { effect, .. } => {
                    let mut events = vec![];
                    for entry in effect.transforms {
                        // Look for data writes into the global state.
                        let WriteCLValue(clvalue) = entry.transform else {
                            continue;
                        };

                        // Look specifically for dictionaries writes.
                        const DICTIONARY_PREFIX: &str = "dictionary-";
                        if !entry.key.starts_with(DICTIONARY_PREFIX) {
                            continue;
                        }

                        // Try parsing CES value, but ignore errors - we don't really know if this is CES dictionary,
                        // because write address is based on key (event ID).
                        let Ok((_total_length, event_value_bytes)) =
                            u32::from_bytes(clvalue.inner_bytes())
                        else {
                            continue;
                        };
                        let Ok((event_name, event_data)) =
                            parse_raw_event_name_and_data(event_value_bytes)
                        else {
                            continue;
                        };

                        // Parse dynamic event data.
                        let dynamic_event = parse_event(event_name, &event_data, event_schema)?;

                        events.push(dynamic_event);
                    }
                    Ok(events)
                }
            },
            ExecutionResult::V2(execution_result_v2) => match &execution_result_v2.error_message {
                None => {
                    let mut events = vec![];
                    for entry in execution_result_v2.effects.value() {
                        // Look for data writes into the global state.
                        let TransformKindV2::Write(stored_value) = entry.kind() else {
                            continue;
                        };

                        let StoredValue::CLValue(clvalue) = stored_value else {
                            continue;
                        };

                        // Look specifically for dictionaries writes.
                        const DICTIONARY_PREFIX: &str = "dictionary-";
                        if !entry
                            .key()
                            .to_formatted_string()
                            .starts_with(DICTIONARY_PREFIX)
                        {
                            continue;
                        }

                        // Try parsing CES value, but ignore errors - we don't really know if this is CES dictionary,
                        // because write address is based on key (event ID).
                        let Ok((_total_length, event_value_bytes)) =
                            u32::from_bytes(clvalue.inner_bytes())
                        else {
                            continue;
                        };
                        let Ok((event_name, event_data)) =
                            parse_raw_event_name_and_data(event_value_bytes)
                        else {
                            continue;
                        };

                        // Parse dynamic event data.
                        let dynamic_event = parse_event(event_name, &event_data, event_schema)?;

                        events.push(dynamic_event);
                    }
                    Ok(events)
                }
                Some(error_message) => {
                    Err(ToolkitError::FailedDeployError(error_message.to_string()))
                }
            },
        }
    }
}
