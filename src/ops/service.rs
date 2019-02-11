
use crate::utils::{ok_future, odd_future};
use crate::runtime::{Runtime};
use crate::msg;
use flatbuffers::FlatBufferBuilder;
use crate::utils::Op;
use libfly::*;
use crate::js::*;
use crate::utils::*;
use futures::future;

pub fn op_request_service_request(rt: &mut Runtime, base: &msg::Base, raw: fly_buf) -> Box<Op> {
    let cmd_id = base.cmd_id();
    let msg = base.msg_as_request_service_request().unwrap();

    let destination_name = msg.destination_name().unwrap().to_string();
    let data = msg.data().unwrap().to_string();

    match &rt.manager_callbacks {
        Some(v) => {
            match (v.uuid_by_servicename)(destination_name) {
                Ok(Some(destination_uuid)) => {
                    match (v.send_message)(destination_uuid, data) {
                        Ok(resp) => {
                            Box::new(future::lazy(move || {
                                let builder = &mut FlatBufferBuilder::new();
                                let data_string = builder.create_string(&resp.data);

                                let msg = msg::RequestServiceResponse::create(
                                    builder,
                                    &msg::RequestServiceResponseArgs {
                                        success: resp.success,
                                        data: Some(data_string),
                                    },
                                );
                                Ok(serialize_response(
                                    cmd_id,
                                    builder,
                                    msg::BaseArgs {
                                        msg: Some(msg.as_union_value()),
                                        msg_type: msg::Any::RequestServiceResponse,
                                        ..Default::default()
                                    },
                                ))
                            }))
                        }
                        Err(_) => odd_future("Failed to make request!".to_string().into()),
                    }
                }
                Ok(None) => odd_future("Runtime not found bound to servicename!".to_string().into()),
                Err(_) => odd_future("An error occured while looking for servicename binding!".to_string().into()),
            }
        },
        None => odd_future("Manager callbacks missing!".to_string().into())
    }
}

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