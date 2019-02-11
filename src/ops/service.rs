
use crate::utils::{ok_future, odd_future};
use crate::runtime::{Runtime};
use crate::msg;
use crate::utils::Op;
use libfly::*;
use crate::js::*;

pub fn op_service_response(rt: &mut Runtime, base: &msg::Base, raw: fly_buf) -> Box<Op> {
  debug!("handle service");
  let msg = base.msg_as_service_response().unwrap();
  let req_id = msg.id();

  let mut responses = rt.service_responses.lock().unwrap();
  match responses.remove(&req_id) {
    Some(sender) => {
      if let Err(_) = sender.send(JsServiceResponse {
        success: msg.success(),
        data: match serde_json::from_str(msg.data().unwrap()) {
          Ok(v) => v,
          Err(_) => return odd_future("error parsing service response json".to_string().into()),
        },
      }) {
        return odd_future("error sending service response".to_string().into());
      }
    },
    None => return odd_future("no service response receiver!".to_string().into()),
  };

  ok_future(None)
}