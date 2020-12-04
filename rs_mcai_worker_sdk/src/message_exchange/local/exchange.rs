use crate::{
  message_exchange::{
    OrderMessage,
    ResponseMessage,
    ExternalExchange,
    InternalExchange,
  },
  Result,
};
use std::sync::mpsc::{channel, Sender, Receiver};

pub struct LocalExchange {
  order_sender: Sender<OrderMessage>,
  order_receiver: Receiver<OrderMessage>,
  response_sender: Sender<ResponseMessage>,
  response_receiver: Receiver<ResponseMessage>,
}

impl LocalExchange {
  pub fn new() -> Self {
    let (order_sender, order_receiver) = channel();
    let (response_sender, response_receiver) = channel();

    LocalExchange {
      order_sender,
      order_receiver,
      response_sender,
      response_receiver,
    }
  }
}

impl ExternalExchange for LocalExchange {
  fn send_order(&mut self, message: OrderMessage) -> Result<()> {
    self.order_sender.send(message).unwrap();
    Ok(())
  }

  fn next_response(&mut self) -> Result<Option<ResponseMessage>> {
    Ok(self.response_receiver.recv().ok())
  }
}

impl InternalExchange for LocalExchange {
  fn send_response(&mut self, message: ResponseMessage) -> Result<()> {
    self.response_sender.send(message).unwrap();
    Ok(())
  }

  fn next_order(&mut self) -> Result<Option<OrderMessage>> {
    Ok(self.order_receiver.recv().ok())
  }
}
