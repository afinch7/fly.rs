use crate::js::JsServiceResponse;
use crate::runtime::{ Runtime, RuntimeConfig };

use crate::errors::{ FlyResult };

use crate::module_resolver::{ ModuleResolver };

use crate::settings::{ Settings };

use std::sync::{ RwLock, Arc, Mutex };

use uuid::Uuid;

pub struct RuntimeManagerCallbacks {
    pub send_message: Box<(Fn(Uuid, String) -> FlyResult<JsServiceResponse>) + Send + Sync>,
    pub uuid_by_servicename: Box<(Fn(String) -> FlyResult<Option<Uuid>>) + Send + Sync>,
}

pub trait RuntimeManager: Send + Sync {
    fn new_runtime(&mut self, config: RuntimeConfig) -> Arc<RwLock<Box<Runtime>>>;
    fn remove_runtime(&self, uuid: Uuid) -> Result<(), RuntimeManagerError>;
    fn bind_servicename_to(&mut self, uuid: Uuid, servicename: &str) -> Result<(), RuntimeManagerError>;
    fn bind_hostname_to(&mut self, uuid: Uuid, hostname: &str) -> Result<(), RuntimeManagerError>;
    fn get_by_hostname(&self, hostname: &str) -> Result<Option<Arc<RwLock<Box<Runtime>>>>, RuntimeManagerError>;
    fn get_by_servicename(&self, servicename: &str) -> Result<Option<Arc<RwLock<Box<Runtime>>>>, RuntimeManagerError>;
    fn get_by_uuid(&self, uuid: Uuid) -> Result<Option<Arc<RwLock<Box<Runtime>>>>, RuntimeManagerError>;
}

pub fn register_manager_with_rt(manager: Box<&'static RuntimeManager>, runtime: Arc<Box<Runtime>>) {
    
}

#[derive(Debug)]
pub enum RuntimeManagerError {
    Failure(String),
}
