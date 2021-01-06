use lapin::message::Delivery;
use std::fmt::{Display, Formatter};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CurrentOrders {
  pub init: Option<Delivery>,
  pub start: Option<Delivery>,
  pub stop: Option<Delivery>,
  pub status: Option<Delivery>,
}

impl CurrentOrders {
  pub(crate) fn reset_process_deliveries(&mut self) {
    self.init = None;
    self.start = None;
    self.stop = None;
  }

  pub(crate) fn reset_status_deliveries(&mut self) {
    self.status = None;
  }

  pub(crate) fn get_process_deliveries(&self) -> Vec<Delivery> {
    Self::filter_sort_and_dedup_deliveries(vec![
      self.init.clone(),
      self.start.clone(),
      self.stop.clone(),
    ])
  }

  pub(crate) fn get_status_deliveries(&self) -> Vec<Delivery> {
    Self::filter_sort_and_dedup_deliveries(vec![self.status.clone()])
  }

  fn filter_sort_and_dedup_deliveries(deliveries: Vec<Option<Delivery>>) -> Vec<Delivery> {
    // Filter nones
    let mut deliveries: Vec<Delivery> = deliveries
      .iter()
      .cloned()
      .filter(|delivery| delivery.is_some())
      .map(|delivery| delivery.unwrap())
      .collect();

    // Sort deliveries by tag
    deliveries.sort_by(|a, b| a.delivery_tag.partial_cmp(&b.delivery_tag).unwrap());

    // Remove duplicated deliveries
    deliveries.dedup();

    deliveries
  }
}

impl Display for CurrentOrders {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
    write!(
      f,
      "CurrentOrders {{ init: {:?}, start: {:?}, stop: {:?}, status: {:?} }}",
      self.init.clone().map(|d| d.delivery_tag),
      self.start.clone().map(|d| d.delivery_tag),
      self.stop.clone().map(|d| d.delivery_tag),
      self.status.clone().map(|d| d.delivery_tag)
    )
  }
}
