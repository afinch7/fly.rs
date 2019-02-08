use crate::runtime::Runtime;
use crate::module_resolver::{ ModuleResolver };
use crate::settings::{ Settings };
use crate::runtime_manager::{ RuntimeManager, RuntimeManagerError, RuntimeManagerCallbacks };
use crate::errors::{ FlyError, FlyResult };

use std::collections::HashMap;

use std::sync::{ Mutex, Arc, RwLock };

use uuid::Uuid;

pub struct StandardRuntimeManager {
    uuid_to_runtime: RwLock<HashMap<String, Arc<Mutex<Box<Runtime>>>>>,
    hostname_to_uuid: RwLock<HashMap<String, String>>,
    servicename_to_uuid: RwLock<HashMap<String, String>>,
    self_ref: Option<Arc<Mutex<StandardRuntimeManager>>>,
}

impl StandardRuntimeManager {
    pub fn new() -> Arc<Mutex<Self>> {
        let new_self = Self { 
            uuid_to_runtime: RwLock::new(HashMap::new()),
            hostname_to_uuid: RwLock::new(HashMap::new()),
            servicename_to_uuid: RwLock::new(HashMap::new()),
            self_ref: None,
        };
        let new_self_ref = Arc::new(Mutex::new(new_self));
        new_self_ref.lock().unwrap().self_ref = Some(new_self_ref.clone());
        new_self_ref
    }
}

impl RuntimeManager for StandardRuntimeManager {
    fn new_runtime(
        &mut self,
        name: Option<String>,
        version: Option<String>,
        settings: &RwLock<Settings>,
        module_resolvers: Option<Vec<Box<ModuleResolver>>>,
    ) -> Arc<Mutex<Box<Runtime>>> {
        let runtime = Runtime::new(name, version, &settings.read().unwrap(), module_resolvers);
        let uuid_map_lock = self.uuid_to_runtime.get_mut().unwrap();
        let uuid = runtime.get_uuid();
        let rt_arc = Arc::new(Mutex::new(runtime));
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
        let send_message = Box::new(move |recieiver: Uuid, message: String| -> FlyResult<String> {
            let man_read_lock = man_send_msg_clone.lock().unwrap();
            let recieiver_rt = man_read_lock.get_by_uuid(recieiver).unwrap();
            let rt_lock = rt_send_msg_clone.lock().unwrap();
            match recieiver_rt {
                Some(v) => {
                    let recieiver_rt_lock = v.lock().unwrap();
                    recieiver_rt_lock.receive_message(uuid::Uuid::parse_str(&rt_lock.get_uuid()).unwrap(), message)
                },
                None => Err(FlyError::from("Receiver not found.".to_string())),
            }
        });
        let man_uuid_by_servicename_clone = man_arc.clone();
        let uuid_by_servicename = Box::new(move |servicename: String| -> FlyResult<Option<Uuid>>{
            return match man_uuid_by_servicename_clone.lock().unwrap().get_by_servicename(&servicename) {
                Ok(Some(v)) => {
                    let rt_lock = v.lock().unwrap();
                    Ok(Some(uuid::Uuid::parse_str(&rt_lock.get_uuid()).unwrap()))
                },
                Ok(None) => Ok(None),
                Err(err) => Err(FlyError::from(err)), 
            };
        });
        let rt_mut_clone = rt_arc.clone();
        let rt_lock = rt_arc.lock().unwrap();
        let mut rt_mut = rt_lock.ptr.to_runtime();
        rt_mut.register_rt_manager_callbacks(RuntimeManagerCallbacks {
            send_message,
            uuid_by_servicename,
        });
        rt_arc.clone()
    }
    fn remove_runtime(&self, uuid: Uuid) -> Result<(), RuntimeManagerError> {
        Err(RuntimeManagerError::Failure("Not implemented".to_string()))
    }
    fn bind_servicename_to(&mut self, uuid: Uuid, servicename: &str) -> Result<(), RuntimeManagerError> {
        let servicename_map_lock = self.hostname_to_uuid.get_mut().unwrap();
        let uuid_string = uuid.to_simple().to_string();
        servicename_map_lock.insert(servicename.to_string(), uuid_string);
        Ok(())
    }
    fn bind_hostname_to(&mut self, uuid: Uuid, hostname: &str) -> Result<(), RuntimeManagerError> {
        let hostname_map_lock = self.hostname_to_uuid.get_mut().unwrap();
        let uuid_string = uuid.to_simple().to_string();
        hostname_map_lock.insert(hostname.to_string(), uuid_string);
        Ok(())
    }
    fn get_by_hostname(&self, hostname: &str) -> Result<Option<Arc<Mutex<Box<Runtime>>>>, RuntimeManagerError> {
        return match self.hostname_to_uuid.read().unwrap().get(hostname) {
            Some(v) => self.get_by_uuid(uuid::Uuid::parse_str(v).unwrap()),
            None => Ok(None),
        };
    }
    fn get_by_servicename(&self, servicename: &str) -> Result<Option<Arc<Mutex<Box<Runtime>>>>, RuntimeManagerError> {
        return match self.servicename_to_uuid.read().unwrap().get(servicename) {
            Some(v) => self.get_by_uuid(uuid::Uuid::parse_str(v).unwrap()),
            None => Ok(None),
        };
    }
    fn get_by_uuid(&self, uuid: Uuid) -> Result<Option<Arc<Mutex<Box<Runtime>>>>, RuntimeManagerError> {
        return match self.uuid_to_runtime.read().unwrap().get(&uuid.to_simple().to_string()) {
            Some(v) => Ok(Some(v.clone())),
            None => Ok(None),
        };
    }
}
