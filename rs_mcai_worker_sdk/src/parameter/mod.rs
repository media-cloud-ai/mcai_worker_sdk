use crate::parameter::media_segment::MediaSegment;

pub mod container;
pub mod credential;
pub mod media_segment;

#[derive(Clone, Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct Requirement {
  pub paths: Option<Vec<String>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum Parameter {
  #[serde(rename = "array_of_media_segments")]
  ArrayOfMediaSegmentsParam {
    id: String,
    default: Option<Vec<MediaSegment>>,
    value: Option<Vec<MediaSegment>>,
  },
  #[serde(rename = "array_of_strings")]
  ArrayOfStringsParam {
    id: String,
    default: Option<Vec<String>>,
    value: Option<Vec<String>>,
  },
  #[serde(rename = "boolean")]
  BooleanParam {
    id: String,
    default: Option<bool>,
    value: Option<bool>,
  },
  #[serde(rename = "credential")]
  CredentialParam {
    id: String,
    default: Option<String>,
    value: Option<String>,
  },
  #[serde(rename = "integer")]
  IntegerParam {
    id: String,
    default: Option<i64>,
    value: Option<i64>,
  },
  #[serde(rename = "json")]
  JsonParam {
    id: String,
    default: Option<String>,
    value: Option<String>,
  },
  #[serde(rename = "requirements")]
  RequirementParam {
    id: String,
    default: Option<Requirement>,
    value: Option<Requirement>,
  },
  #[serde(rename = "string")]
  StringParam {
    id: String,
    default: Option<String>,
    value: Option<String>,
  },
}

impl Parameter {
  pub fn get_id(&self) -> String {
    match self {
      Parameter::ArrayOfMediaSegmentsParam { id, .. }
      | Parameter::ArrayOfStringsParam { id, .. }
      | Parameter::BooleanParam { id, .. }
      | Parameter::CredentialParam { id, .. }
      | Parameter::IntegerParam { id, .. }
      | Parameter::JsonParam { id, .. }
      | Parameter::RequirementParam { id, .. }
      | Parameter::StringParam { id, .. } => id.clone(),
    }
  }

  pub fn has_value_or_default(&self) -> bool {
    match self {
      Parameter::ArrayOfMediaSegmentsParam { value, default, .. } => {
        value.is_some() || default.is_some()
      }
      Parameter::ArrayOfStringsParam { value, default, .. } => value.is_some() || default.is_some(),
      Parameter::BooleanParam { value, default, .. } => value.is_some() || default.is_some(),
      Parameter::CredentialParam { value, default, .. } => value.is_some() || default.is_some(),
      Parameter::IntegerParam { value, default, .. } => value.is_some() || default.is_some(),
      Parameter::JsonParam { value, default, .. } => value.is_some() || default.is_some(),
      Parameter::RequirementParam { value, default, .. } => value.is_some() || default.is_some(),
      Parameter::StringParam { value, default, .. } => value.is_some() || default.is_some(),
    }
  }
}

macro_rules! parameter_to_string {
  ($default:tt, $value:tt, $pattern:tt) => {{
    let current_value = if let Some(value) = $value {
      value
    } else if let Some(default) = $default {
      default
    } else {
      return "".to_string();
    };
    format!($pattern, current_value)
  }};
}

impl ToString for Parameter {
  fn to_string(&self) -> String {
    match self {
      Parameter::ArrayOfMediaSegmentsParam { default, value, .. } => {
        parameter_to_string!(default, value, "{:?}")
      }
      Parameter::ArrayOfStringsParam { default, value, .. } => {
        parameter_to_string!(default, value, "{:?}")
      }
      Parameter::RequirementParam { default, value, .. } => {
        parameter_to_string!(default, value, "{:?}")
      }
      Parameter::BooleanParam { default, value, .. } => parameter_to_string!(default, value, "{}"),
      Parameter::CredentialParam { default, value, .. } => {
        parameter_to_string!(default, value, "{}")
      }
      Parameter::IntegerParam { default, value, .. } => parameter_to_string!(default, value, "{}"),
      Parameter::JsonParam { default, value, .. } => parameter_to_string!(default, value, "{}"),
      Parameter::StringParam { default, value, .. } => parameter_to_string!(default, value, "{}"),
    }
  }
}
