use amq_protocol_types::{AMQPValue, FieldTable};
use lapin::message::Delivery;

pub fn get_message_death_count(message: &Delivery) -> Option<i64> {
  get_count_from_header(message.properties.headers())
}

fn get_count_from_header(header: &Option<FieldTable>) -> Option<i64> {
  if let Some(header) = header {
    if let Some(death) = header.inner().get("x-death") {
      if let AMQPValue::FieldArray(array) = death {
        let raw_array = array.as_slice();
        if raw_array.is_empty() {
          return None;
        }
        if let AMQPValue::FieldTable(params) = &raw_array[0] {
          if let Some(AMQPValue::LongLongInt(value)) = params.inner().get("count") {
            return Some(*value);
          }
        }
      }
    }
  };
  None
}

#[test]
fn header_information() {
  use std::collections::BTreeMap;

  let header = None;
  let count = get_count_from_header(&header);
  assert!(count == None);

  let mut map = FieldTable::from(BTreeMap::new());
  let mut properties = FieldTable::from(BTreeMap::new());
  properties.insert("count".into(), AMQPValue::LongLongInt(666));

  map.insert("x-death".into(), AMQPValue::FieldArray(vec![].into()));
  let header = Some(map);
  let count = get_count_from_header(&header);
  assert!(count == None);

  let mut map = FieldTable::from(BTreeMap::new());
  let mut properties = FieldTable::from(BTreeMap::new());
  properties.insert("count".into(), AMQPValue::LongLongInt(666));

  map.insert(
    "x-death".into(),
    AMQPValue::FieldArray(vec![AMQPValue::FieldTable(properties).into()].into()),
  );
  let header = Some(map);
  let count = get_count_from_header(&header);
  assert!(count == Some(666));
}
