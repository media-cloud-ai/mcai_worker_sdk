#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum WorkerParameterType {
  #[serde(rename = "array_of_strings")]
  ArrayOfStrings,
  #[serde(rename = "boolean")]
  Boolean,
  #[serde(rename = "credential")]
  Credential,
  #[serde(rename = "integer")]
  Integer,
  #[serde(rename = "requirements")]
  Requirements,
  #[serde(rename = "string")]
  String,
}
