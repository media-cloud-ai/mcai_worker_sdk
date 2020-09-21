mod time_expression;

use std::io::{Read, Write};
pub use time_expression::{Frames, TimeExpression, TimeUnit};
use yaserde::{YaDeserialize, YaSerialize};

pub fn default_lang() -> String {
  "en".to_owned()
}

#[derive(Debug, Default, Clone, PartialEq, YaDeserialize, YaSerialize)]
#[yaserde(
  rename = "tt",
  prefix = "tt",
  namespace = "tt: http://www.w3.org/ns/ttml",
  namespace = "xml: http://www.w3.org/XML/1998/namespace",
  namespace = "ttm: http://www.w3.org/ns/ttml#metadata",
  namespace = "ttp: http://www.w3.org/ns/ttml#parameter",
  namespace = "ebuttp: urn:ebu:tt:parameters"
)]
pub struct EbuTtmlLive {
  #[yaserde(rename = "lang", prefix = "xml", attribute)]
  pub language: Option<String>,
  #[yaserde(rename = "sequenceIdentifier", prefix = "ebuttp", attribute)]
  pub sequence_identifier: Option<String>,
  #[yaserde(rename = "sequenceNumber", prefix = "ebuttp", attribute)]
  pub sequence_number: Option<u64>,
  #[yaserde(rename = "clockMode", prefix = "ttp", attribute)]
  pub clock_mode: Option<String>,
  #[yaserde(rename = "timeBase", prefix = "ttp", attribute)]
  pub time_base: Option<String>,
  pub head: Head,
  pub body: Body,
}

#[derive(Debug, Default, Clone, PartialEq, YaDeserialize, YaSerialize)]
#[yaserde(
  rename = "head",
  prefix = "tt",
  namespace = "tt: http://www.w3.org/ns/ebu_ttml_live"
)]
pub struct Head {
  #[yaserde(prefix = "tt")]
  pub metadata: Option<Metadata>,
  #[yaserde(prefix = "tt")]
  pub styling: Option<Styling>,
  #[yaserde(prefix = "tt")]
  pub layout: Option<Layout>,
}

#[derive(Clone, Debug, Default, PartialEq, YaDeserialize, YaSerialize)]
#[yaserde(
  rename = "metadata",
  prefix = "tt",
  namespace = "tt: http://www.w3.org/ns/ttml",
  namespace = "ttm: http://www.w3.org/ns/ttml#metadata"
)]
pub struct Metadata {
  #[yaserde(prefix = "ttm")]
  pub title: Option<Title>,
  #[yaserde(prefix = "ttm")]
  pub desc: Option<String>,
  #[yaserde(prefix = "ttm")]
  pub copyright: Option<String>,
  #[yaserde(prefix = "ttm")]
  pub agent: Option<String>,
  #[yaserde(prefix = "ttm")]
  pub actor: Option<String>,
}

#[derive(Clone, Default, Debug, PartialEq, YaDeserialize, YaSerialize)]
#[yaserde(
  rename = "styling",
  prefix = "tt",
  namespace = "tt: http://www.w3.org/ns/ttml"
)]
pub struct Styling {
  #[yaserde(prefix = "tt", attribute, default = "default_lang")]
  pub lang: String,
}

#[derive(Clone, Default, Debug, PartialEq, YaDeserialize, YaSerialize)]
#[yaserde(prefix = "ttm", namespace = "ttm: http://www.w3.org/ns/ttml#metadata")]
pub struct Title {
  #[yaserde(prefix = "ttm")]
  pub id: String,
  #[yaserde(prefix = "ttm", default = "default_lang")]
  pub lang: String,
  #[yaserde(prefix = "ttm", text)]
  pub content: String,
}

#[derive(Clone, Default, Debug, PartialEq, YaDeserialize, YaSerialize)]
#[yaserde(prefix = "tt", namespace = "tt: http://www.w3.org/ns/ttml")]
pub struct Layout {
  #[yaserde(attribute, default = "default_lang")]
  pub lang: String,
}

#[derive(Debug, Default, Clone, PartialEq, YaDeserialize, YaSerialize)]
#[yaserde(
  rename = "body",
  prefix = "tt",
  namespace = "tt: http://www.w3.org/ns/ttml"
)]
pub struct Body {
  #[yaserde(rename = "dur", attribute)]
  pub duration: Option<TimeExpression>,
  #[yaserde(rename = "begin", attribute)]
  pub begin: Option<TimeExpression>,
  #[yaserde(rename = "end", attribute)]
  pub end: Option<TimeExpression>,
  #[yaserde(rename = "div")]
  pub divs: Vec<Div>,
}

#[derive(Debug, Default, Clone, PartialEq, YaDeserialize, YaSerialize)]
#[yaserde(
  rename = "div",
  prefix = "tt",
  namespace = "tt: http://www.w3.org/ns/ttml"
)]
pub struct Div {
  #[yaserde(rename = "p")]
  pub paragraphs: Vec<Paragraph>,
}

#[derive(Debug, Default, Clone, PartialEq, YaDeserialize, YaSerialize)]
#[yaserde(
  rename = "p",
  prefix = "tt",
  namespace = "tt: http://www.w3.org/ns/ttml"
)]
pub struct Paragraph {
  #[yaserde(rename = "span")]
  pub spans: Vec<Span>,
  #[yaserde(rename = "dur", attribute)]
  pub duration: Option<TimeExpression>,
  #[yaserde(rename = "begin", attribute)]
  pub begin: Option<TimeExpression>,
  #[yaserde(rename = "end", attribute)]
  pub end: Option<TimeExpression>,
}

#[derive(Debug, Default, Clone, PartialEq, YaDeserialize, YaSerialize)]
#[yaserde(
  rename = "span",
  prefix = "tt",
  namespace = "tt: http://www.w3.org/ns/ttml"
)]
pub struct Span {
  #[yaserde(text)]
  pub content: String,
}

#[derive(Debug, Default, Clone, PartialEq, YaDeserialize, YaSerialize)]
#[yaserde(
  rename = "br",
  prefix = "tt",
  namespace = "tt: http://www.w3.org/ns/ttml"
)]
pub struct BreakLine {}

// #[test]
// pub fn test_deser() {
//   let content = "<?xml version=\"1.0\" encoding=\"utf-8\"?><tt:tt xmlns:ebuttp=\"urn:ebu:tt:parameters\" xmlns:tt=\"http://www.w3.org/ns/ttml\" xmlns:ttm=\"http://www.w3.org/ns/ttml#metadata\" xmlns:ttp=\"http://www.w3.org/ns/ttml#parameter\" xml:lang=\"fr-FR\" ebuttp:sequenceIdentifier=\"LiveSubtitle\" ebuttp:sequenceNumber=\"0\" ttp:clockMode=\"local\" ttp:timeBase=\"clock\"><head><tt:metadata /><tt:styling /><tt:layout /></head><body dur=\"00:00:10:00\" begin=\"0ms\"><div><p begin=\"0ms\" end=\"10ms\"><span>Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore disputandum putant. Sed ut perspiciatis, unde omnis iste.</span></p></div></body></tt:tt>";
//   let result = yaserde::de::from_str::<EbuTtmlLive>(content);
//   println!("result: {:?}", result);
// }
