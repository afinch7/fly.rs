extern crate futures;
use futures::{future, Future};

extern crate tokio;

extern crate trust_dns as dns;
extern crate trust_dns_proto;
extern crate trust_dns_server;

use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use std::sync::Arc;

extern crate flatbuffers;

#[macro_use]
extern crate log;
extern crate env_logger;
extern crate fly;
extern crate libfly;

use fly::runtime::*;
use fly::settings::SETTINGS;
use fly::runtime_manager::RuntimeManager;
use fly::{dns_server::DnsServer, standard_runtime_manager::StandardRuntimeManager};
use fly::module_resolver::{ ModuleResolver, JsonSecretsResolver, LocalDiskModuleResolver };

use env_logger::Env;

extern crate clap;

use std::path::{ PathBuf };

static mut SELECTOR: Option<Arc<StandardRuntimeManager>> = None;

fn main() {
  let env = Env::default().filter_or("LOG_LEVEL", "info");
  env_logger::init_from_env(env);
  debug!("V8 version: {}", libfly::version());

  let matches = clap::App::new("fly-dns")
    .version("0.0.1-alpha")
    .about("Fly DNS server")
    .arg(
      clap::Arg::with_name("port")
        .short("p")
        .long("port")
        .takes_value(true),
    )
    .arg(
      clap::Arg::with_name("secrets-file")
        .short("sf")
        .long("secrets-file")
        .takes_value(true)
    )
    .arg(
      clap::Arg::with_name("input")
        .help("Sets the input file to use")
        .required(true)
        .index(1),
    )
    .get_matches();

  let mut module_resolvers: Vec<Box<ModuleResolver>> = std::vec::Vec::new();

  let secrets_file = match matches.value_of("secrets-file") {
    Some(v) => v,
    None => "./secrets.json", 
  };

  let secrets_file_path = PathBuf::from(secrets_file);
  info!("Loading secrets file from path {}", secrets_file_path.to_str().unwrap().to_string());
  match secrets_file_path.is_file() {
    true => {
      let secrets_json = match std::fs::read_to_string(&secrets_file_path.to_str().unwrap().to_string()) {
        Ok(v) => v,
        Err(_err) => {
          info!("Failed to load secrets file!");
          "{}".to_string()
        },
      };
      let json_value: serde_json::Value = match serde_json::from_str(secrets_json.as_str()) {
        Ok(v) => v,
        Err(_err) => {
          // TODO: actual error output
          info!("Failed to parse json");
          serde_json::from_str("{}").unwrap()
        }
      };
      module_resolvers.push(Box::new(JsonSecretsResolver::new(json_value)));
    },
    false => {
      info!("Secrets file invalid");
    },
  };


  module_resolvers.push(Box::new(LocalDiskModuleResolver::new(None)));

  info!("Module resolvers length {}", module_resolvers.len().to_string());

  let entry_file = matches.value_of("input").unwrap();

  let rt_manager = StandardRuntimeManager::new();

  let runtime = rt_manager.lock().unwrap().new_runtime(None, None, &SETTINGS, Some(module_resolvers));

  {
    let rt_ref_clone = runtime.clone();
    let rt_lock = rt_ref_clone.lock().unwrap();
    debug!("Loading dev tools");
    rt_lock.eval_file("v8env/dist/dev-tools.js");
    rt_lock.eval("<installDevTools>", "installDevTools();");
    debug!("Loading dev tools done");
    rt_lock.eval(entry_file, &format!("dev.run('{}')", entry_file));
  }

  let port: u16 = match matches.value_of("port") {
    Some(pstr) => pstr.parse::<u16>().unwrap(),
    None => 8053,
  };

  let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), port);

  tokio::run(future::lazy(move || -> Result<(), ()> {
    let rt_lock = runtime.lock().unwrap();
    tokio::spawn(
      rt_lock
        .ptr.to_runtime()
        .run()
        .map_err(|e| error!("error running runtime event loop: {}", e)),
    );
    let server = DnsServer::new(addr, rt_manager.clone());
    server.start();
    Ok(())
  }));
}
