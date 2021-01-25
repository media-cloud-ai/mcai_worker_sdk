
/// Worker activity mode (idle or busy)
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum WorkerActivity {
  Idle,
  Busy,
}
