#[derive(Clone, Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct MediaSegment {
  pub start: u64,
  pub end: u64,
}

impl MediaSegment {
  pub fn new(start: u64, end: u64) -> MediaSegment {
    MediaSegment { start, end }
  }
}
