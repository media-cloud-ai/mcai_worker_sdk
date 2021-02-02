/// Worker activity mode
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum WorkerActivity {
  Idle,
  Busy,
}
