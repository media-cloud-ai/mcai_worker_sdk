
use amq_protocol_types::AMQPValue;
use lapin_futures::message::Delivery;

pub fn get_message_death_count(message: &Delivery) -> Option<i64> {
  let message_header = message.properties.headers().clone();
  if let Some(header) = message_header {
    let header = header.inner();
    if let Some(death) = header.get("x-death") {
      if let AMQPValue::FieldArray(array) = death {
        let pouet = array.as_slice();
        if let AMQPValue::FieldTable(params) = &pouet[0] {
          if let Some(AMQPValue::LongLongInt(value)) = params.inner().get("count") {
            return Some(*value)
          }
        }
      }
    }
  };

  None
}
