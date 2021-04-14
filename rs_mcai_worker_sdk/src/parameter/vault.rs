use serde_json::Value;

#[derive(Debug, Deserialize)]
pub(crate) struct VaultSecretResponse {
  pub(crate) data: VaultSecretResponseData,
}

#[derive(Debug, Deserialize)]
pub(crate) struct VaultSecretResponseData {
  pub(crate) data: Value,
}
