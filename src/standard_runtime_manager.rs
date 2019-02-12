use crate::runtime::RuntimeConfig;
use crate::runtime::Runtime;
use crate::runtime_manager::{ RuntimeManager, RuntimeManagerError, RuntimeManagerCallbacks };
use crate::errors::{ FlyError, FlyResult };

use crate::{get_next_stream_id};

use crate::js::*;
use crate::utils::*;

use std::collections::HashMap;

use std::sync::{ Mutex, Arc, RwLock };

use uuid::Uuid;

use futures::future::Future;

use futures::sync::oneshot;

pub struct StandardRuntimeManager {
    uuid_to_runtime: RwLock<HashMap<String, Arc<RwLock<Box<Runtime>>>>>,
    hostname_to_uuid: RwLock<HashMap<String, String>>,
    servicename_to_uuid: RwLock<HashMap<String, String>>,
    self_ref: Option<Arc<RwLock<StandardRuntimeManager>>>,
}

impl StandardRuntimeManager {
    pub fn new() -> Arc<RwLock<Self>> {
        let mut new_self_ref = Arc::new(RwLock::new(Self { 
            uuid_to_runtime: RwLock::new(HashMap::new()),
            hostname_to_uuid: RwLock::new(HashMap::new()),
            servicename_to_uuid: RwLock::new(HashMap::new()),
            self_ref: None,
        }));
        new_self_ref.write().unwrap().self_ref = Some(new_self_ref.clone());
        new_self_ref
    }
}

impl RuntimeManager for StandardRuntimeManager {
    fn new_runtime(
        &mut self,
        config: RuntimeConfig,
    ) -> Arc<RwLock<Box<Runtime>>> {
        let runtime = Runtime::new(config);
        let uuid_map_lock = self.uuid_to_runtime.get_mut().unwrap();
        let uuid = runtime.get_uuid();
        let rt_arc = Arc::new(RwLock::new(runtime));
        uuid_map_lock.insert(uuid, rt_arc.clone());
        let man_arc = match &self.self_ref {
            Some(v) => v.clone(),
            None => {
                warn!("Self ref missing.");
                std::process::exit(1);
            },
        };
        let man_send_msg_clone = man_arc.clone();
        let rt_send_msg_clone = rt_arc.clone();
        let send_message = Box::new(move |recieiver: Uuid, message: String| -> FlyResult<oneshot::Receiver<JsServiceResponse>> {
            let man_read_lock = man_send_msg_clone.read().unwrap();
            let recieiver_rt = man_read_lock.get_by_uuid(recieiver).unwrap();
            let rt_lock = rt_send_msg_clone.read().unwrap();
            match recieiver_rt {
                Some(v) => {
                    let recieiver_rt_lock = v.read().unwrap();
                    let eid = get_next_stream_id();
                    if recieiver_rt_lock.get_uuid() == rt_lock.get_uuid() {
                        return Err("Cannot send requests to the same runtime.(Creates race condition with blocking operations)".to_string().into());
                    }
                    match recieiver_rt_lock.dispatch_event(
                        eid,
                        JsEvent::Serve(JsServiceRequest {
                            id: eid,
                            sender: recieiver_rt_lock.get_uuid(),
                            data: message,
                        }),
                    ) {
                        None => Err("Failed to dispatch service request".to_string().into()),
                        Some(Err(e)) => Err(format!("error sending js service request: {:?}", e).to_string().into()),
                        Some(Ok(EventResponseChannel::Service(rx))) => Ok(rx),
                        _ => unimplemented!(),
                    }
                },
                None => Err(FlyError::from("Receiver not found.".to_string())),
            }
        });
        let man_uuid_by_servicename_clone = man_arc.clone();
        let uuid_by_servicename = Box::new(move |servicename: String| -> FlyResult<Option<Uuid>>{
            return match man_uuid_by_servicename_clone.read().unwrap().get_by_servicename(&servicename) {
                Ok(Some(v)) => {
                    let rt_lock = v.read().unwrap();
                    Ok(Some(uuid::Uuid::parse_str(&rt_lock.get_uuid()).unwrap()))
                },
                Ok(None) => Ok(None),
                Err(err) => Err(FlyError::from(err)), 
            };
        });
        let rt_mut_clone = rt_arc.clone();
        {
            let rt_lock = rt_arc.read().unwrap();
            let rt_mut = rt_lock.ptr.to_runtime();
            rt_mut.register_rt_manager_callbacks(RuntimeManagerCallbacks {
                send_message,
                uuid_by_servicename,
            });
        }
        rt_arc.clone()
    }
    fn remove_runtime(&self, uuid: Uuid) -> Result<(), RuntimeManagerError> {
        Err(RuntimeManagerError::Failure("Not implemented".to_string()))
    }
    fn bind_servicename_to(&mut self, uuid: Uuid, servicename: &str) -> Result<(), RuntimeManagerError> {
        let uuid_string = uuid.to_simple().to_string();
        match self.servicename_to_uuid.get_mut().unwrap().insert(servicename.to_string(), uuid_string) {
            Some(v) => Ok(()),
            None => Ok(()),
        }
    }
    fn bind_hostname_to(&mut self, uuid: Uuid, hostname: &str) -> Result<(), RuntimeManagerError> {
        let hostname_map_lock = self.hostname_to_uuid.get_mut().unwrap();
        let uuid_string = uuid.to_simple().to_string();
        hostname_map_lock.insert(hostname.to_string(), uuid_string);
        Ok(())
    }
    fn get_by_hostname(&self, hostname: &str) -> Result<Option<Arc<RwLock<Box<Runtime>>>>, RuntimeManagerError> {
        return match self.hostname_to_uuid.read().unwrap().get(hostname) {
            Some(v) => self.get_by_uuid(uuid::Uuid::parse_str(v).unwrap()),
            None => Ok(None),
        };
    }
    fn get_by_servicename(&self, servicename: &str) -> Result<Option<Arc<RwLock<Box<Runtime>>>>, RuntimeManagerError> {
        return match self.servicename_to_uuid.read().unwrap().get(servicename) {
            Some(v) => self.get_by_uuid(uuid::Uuid::parse_str(v).unwrap()),
            None => Ok(None),
        };
    }
    fn get_by_uuid(&self, uuid: Uuid) -> Result<Option<Arc<RwLock<Box<Runtime>>>>, RuntimeManagerError> {
        return match self.uuid_to_runtime.read().unwrap().get(&uuid.to_simple().to_string()) {
            Some(v) => Ok(Some(v.clone())),
            None => Ok(None),
        };
    }
}
