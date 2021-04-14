mod time_expression;

use mcai_worker_sdk::prelude::*;
use pyo3::prelude::*;
use time_expression::PyTtmlTimeExpression;

#[pyclass]
#[derive(Debug, Default, Clone, PartialEq)]
pub struct PyEbuTtmlLive {
  #[pyo3(get, set)]
  pub language: Option<String>,
  #[pyo3(get, set)]
  pub sequence_identifier: Option<String>,
  #[pyo3(get, set)]
  pub sequence_number: Option<u64>,
  #[pyo3(get, set)]
  pub clock_mode: Option<String>,
  #[pyo3(get, set)]
  pub time_base: Option<String>,
  #[pyo3(get, set)]
  pub head: Head,
  #[pyo3(get, set)]
  pub body: PyTtmlBody,
}

impl From<EbuTtmlLive> for PyEbuTtmlLive {
  fn from(ebu_ttml_live: EbuTtmlLive) -> Self {
    PyEbuTtmlLive {
      language: ebu_ttml_live.language,
      sequence_identifier: ebu_ttml_live.sequence_identifier,
      sequence_number: ebu_ttml_live.sequence_number,
      clock_mode: ebu_ttml_live.clock_mode,
      time_base: ebu_ttml_live.time_base,
      head: ebu_ttml_live.head,
      body: ebu_ttml_live.body.into(),
    }
  }
}

#[pyclass]
#[derive(Debug, Default, Clone, PartialEq)]
pub struct PyTtmlBody {
  #[pyo3(get, set)]
  pub duration: Option<PyTtmlTimeExpression>,
  #[pyo3(get, set)]
  pub begin: Option<PyTtmlTimeExpression>,
  #[pyo3(get, set)]
  pub end: Option<PyTtmlTimeExpression>,
  #[pyo3(get, set)]
  pub divs: Vec<PyTtmlDiv>,
}

impl From<Body> for PyTtmlBody {
  fn from(body: Body) -> Self {
    PyTtmlBody {
      duration: body.duration.map(|time_expression| time_expression.into()),
      begin: body.begin.map(|time_expression| time_expression.into()),
      end: body.end.map(|time_expression| time_expression.into()),
      divs: body.divs.iter().map(|div| div.clone().into()).collect(),
    }
  }
}

#[pyclass]
#[derive(Debug, Default, Clone, PartialEq)]
pub struct PyTtmlDiv {
  #[pyo3(get, set)]
  pub paragraphs: Vec<PyTtmlParagraph>,
}

impl From<Div> for PyTtmlDiv {
  fn from(div: Div) -> Self {
    PyTtmlDiv {
      paragraphs: div.paragraphs.iter().map(|p| p.clone().into()).collect(),
    }
  }
}

#[pyclass]
#[derive(Debug, Default, Clone, PartialEq)]
pub struct PyTtmlParagraph {
  #[pyo3(get, set)]
  pub spans: Vec<PyTtmlSpan>,
  #[pyo3(get, set)]
  pub duration: Option<PyTtmlTimeExpression>,
  #[pyo3(get, set)]
  pub begin: Option<PyTtmlTimeExpression>,
  #[pyo3(get, set)]
  pub end: Option<PyTtmlTimeExpression>,
}

impl From<Paragraph> for PyTtmlParagraph {
  fn from(paragraph: Paragraph) -> Self {
    PyTtmlParagraph {
      spans: paragraph
        .spans
        .iter()
        .cloned()
        .map(|span| span.into())
        .collect(),
      duration: paragraph.duration.map(|time_expr| time_expr.into()),
      begin: paragraph.begin.map(|time_expr| time_expr.into()),
      end: paragraph.end.map(|time_expr| time_expr.into()),
    }
  }
}

#[pyclass]
#[derive(Debug, Default, Clone, PartialEq)]
pub struct PyTtmlSpan {
  #[pyo3(get, set)]
  pub text: String,
}

impl From<Span> for PyTtmlSpan {
  fn from(span: Span) -> Self {
    PyTtmlSpan { text: span.content }
  }
}

#[cfg(test)]
use mcai_worker_sdk::prelude::{TimeExpression, TimeUnit};

#[test]
pub fn test_py_ttml_paragraph() {
  let span = Span {
    content: "Hello world!".to_string(),
  };
  let py_ttml_span = PyTtmlSpan::from(span.clone());

  let paragraph = Paragraph {
    spans: vec![span],
    duration: Some(TimeExpression::OffsetTime {
      offset: 123.0,
      unit: TimeUnit::Frames,
    }),
    begin: None,
    end: None,
  };
  let py_ttml_paragraph = PyTtmlParagraph::from(paragraph);

  assert_eq!(vec![py_ttml_span], py_ttml_paragraph.spans);
  assert_eq!(
    "00:00:04:23".to_string(),
    py_ttml_paragraph.duration.unwrap().to_time_code()
  );
  assert!(py_ttml_paragraph.begin.is_none());
  assert!(py_ttml_paragraph.end.is_none());
}

#[test]
pub fn test_py_ttml_span() {
  let span = Span {
    content: "Hello world!".to_string(),
  };
  let py_ttml_span = PyTtmlSpan::from(span.clone());

  assert_eq!(span.content, py_ttml_span.text);
}
