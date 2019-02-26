use crate::runtime::Runtime;
use crate::utils::*;
use flatbuffers::FlatBufferBuilder;
use libfly::*;
use crate::msg;
use futures::future;

pub fn op_get_runtime_info(rt: &mut Runtime, base: &msg::Base, _raw: fly_buf) -> Box<Op> {
    if let Err(e) = rt.permissions.check_read_rt_infos() {
        return odd_future(e);
    }

    let cmd_id = base.cmd_id();
    let msg = base.msg_as_get_runtime_info().unwrap();

    let read_uuid = msg.runtime_uuid().unwrap();

    match &rt.manager_callbacks {
        Some(man_callbacks) => match man_callbacks.read_runtime_info(uuid::Uuid::parse_str(read_uuid).unwrap()) {
            Ok(Some(info)) => Box::new(future::lazy(move || {
                let builder = &mut FlatBufferBuilder::new();
                let name = builder.create_string(&info.name);
                let version = builder.create_string(&info.version);
                let default_module_url = builder.create_string(&info.default_module_url);
                let msg = msg::GetRuntimeInfoResp::create(
                    builder,
                    &msg::GetRuntimeInfoRespArgs {
                        name: Some(name),
                        version: Some(version),
                        default_module_url: Some(default_module_url),
                    },
                );
                Ok(serialize_response(
                    cmd_id,
                    builder,
                    msg::BaseArgs {
                        msg: Some(msg.as_union_value()),
                        msg_type: msg::Any::GetRuntimeInfoResp,
                        ..Default::default()
                    },
                ))
            })),
            Ok(None) => odd_future("Runtime not found.".to_string().into()),
            Err(e) => odd_future(e),
        },
        None => odd_future("Manager callbacks missing.".to_string().into()),
    }

}

pub fn op_read_file_url(rt: &mut Runtime, base: &msg::Base, _raw: fly_buf) -> Box<Op> {
    if let Err(e) = rt.permissions.check_read_file_url() {
        return odd_future(e);
    }

    let cmd_id = base.cmd_id();
    let msg = base.msg_as_read_file_url().unwrap();

    let read_url = msg.url().unwrap();

    match url::Url::parse(read_url) {
        Ok(parsed_url) => match parsed_url.to_file_path() {
                Ok(file_path) => {
                    Box::new(future::lazy(move || {
                        let builder = &mut FlatBufferBuilder::new();
                        if file_path.is_dir() {
                            Err("This call doesn't currently support directories.".to_string().into())
                        }
                        let content = match std::fs::read_to_string(file_path) {
                            Ok(v) => Some(builder.create_string(v.as_str())),
                            Err(_) => None,
                        };

                        let msg = msg::ReadFileUrlResp::create(
                            builder,
                            &msg::ReadFileUrlRespArgs {
                                exists: true,
                                content,
                            },
                        );
                        Ok(serialize_response(
                            cmd_id,
                            builder,
                            msg::BaseArgs {
                                msg: Some(msg.as_union_value()),
                                msg_type: msg::Any::ReadFileUrlResp,
                                ..Default::default()
                            },
                        ))
                    }))
                },
                Err(e) => odd_future(e.into()),
        },
        Err(e) => odd_future(e.into()),
    }
}
