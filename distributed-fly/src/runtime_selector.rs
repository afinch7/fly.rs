use futures::Future;

use fly::{
    runtime::{Runtime, RuntimeConfig},
    RuntimeSelector, SelectorError,
};

use std::collections::HashMap;
use std::sync::RwLock;

use crate::libs::fetch_libs;
use crate::release::Release;
use crate::settings::GLOBAL_SETTINGS;

pub struct DistributedRuntimeSelector {
    uuid_to_runtime: RwLock<HashMap<String, Box<Runtime>>>,
    hostname_to_runtime: RwLock<HashMap<String, Box<Runtime>>>,
}

impl DistributedRuntimeSelector {
    pub fn new() -> Self {
        DistributedRuntimeSelector {
            uuid_to_runtime: RwLock::new(HashMap::new()),
            hostname_to_runtime: RwLock::new(HashMap::new()),
        }
    }
}

impl Drop for DistributedRuntimeSelector {
    fn drop(&mut self) {
        let mut writer = match self.runtimes.write() {
            Ok(w) => w,
            Err(poisoned) => poisoned.into_inner(),
        };
        writer.iter_mut().for_each(|(k, rt)| {
            debug!("Disposing of runtime: {}", k);
            rt.dispose();
        });
    }
}

impl RuntimeSelector for DistributedRuntimeSelector {
    fn get_by_hostname(&self, hostname: &str) -> Result<Option<&mut Runtime>, SelectorError> {
        let rel = match Release::get(hostname) {
            Err(e) => return Err(SelectorError::Failure(e)),
            Ok(maybe_rel) => match maybe_rel {
                None => return Ok(None),
                Some(rel) => rel,
            },
        };

        let key = format!("{}:{}", rel.app_id, rel.version);

        let runtimes = &self.runtimes;

        let exists = {
            match runtimes.read() {
                Ok(guard) => guard.contains_key(&key),
                Err(e) => return Err(SelectorError::Failure(format!("{}", e))),
            }
        };

        if !exists {
            let mut writer = match runtimes.write() {
                Ok(w) => w,
                Err(poisoned) => {
                    error!("runtimes writer is poisoned! {}", poisoned);
                    poisoned.into_inner() // recover...
                }
            };
            let settings = {
                use fly::settings::*;
                let global_settings = &*GLOBAL_SETTINGS.read().unwrap();
                Settings {
                    data_store: Some(DataStore::Postgres(PostgresStoreConfig {
                        url: global_settings.cockroach_host.clone(),
                        database: Some(format!("objectstore_{}", rel.app_id)),
                        tls_ca_crt: if let Some(ref certs_path) =
                            global_settings.cockroach_certs_path
                        {
                            Some(format!("{}/ca.crt", certs_path))
                        } else {
                            None
                        },
                        tls_client_crt: if let Some(ref certs_path) =
                            global_settings.cockroach_certs_path
                        {
                            Some(format!("{}/client.root.crt", certs_path))
                        } else {
                            None
                        },
                        tls_client_key: if let Some(ref certs_path) =
                            global_settings.cockroach_certs_path
                        {
                            Some(format!("{}/client.root.key", certs_path))
                        } else {
                            None
                        },
                    })), // TODO: use postgres store
                    cache_store: Some(CacheStore::Redis(RedisStoreConfig {
                        url: global_settings.redis_cache_url.clone(),
                        namespace: Some(rel.app_id.to_string()),
                    })), // TODO: use redis store
                    cache_store_notifier: match global_settings.redis_cache_notifier_url {
                        Some(ref url) => {
                            Some(CacheStoreNotifier::Redis(RedisCacheNotifierConfig {
                                reader_url: url.clone(),
                                writer_url: global_settings
                                    .redis_cache_notifier_writer_url
                                    .as_ref()
                                    .unwrap_or(url)
                                    .clone(),
                            }))
                        }
                        None => None,
                    },
                    fs_store: Some(FsStore::Redis(RedisStoreConfig {
                        namespace: Some(format!("app:{}:release:latest:file:", rel.app_id)),
                        url: global_settings.redis_url.clone(),
                    })),
                    acme_store: Some(AcmeStoreConfig::Redis(RedisStoreConfig {
                        url: global_settings.redis_url.clone(),
                        namespace: None,
                    })),
                }
            };

            let mut rt = Runtime::new(RuntimeConfig {
                name: Some(rel.app_id.to_string()),
                version: Some(rel.version.to_string()),
                settings: &settings,
                module_resolvers: Some(vec![]),
                app_logger: &slog_scope::logger(),
                msg_handler: None,
                permissions: None,
                dev_tools: false,
            });
            let merged_conf = rel.clone().parsed_config().unwrap();
            rt.eval(
                "<app config>",
                &format!(
                    "window.fly.app = {{ config: {}, version: {} }};",
                    merged_conf, rel.version
                ),
            );

            // load external libraries if requested
            if let Some(libs) = rel.libs {
                match fetch_libs(&libs[..]) {
                    Ok(lib_sources) => {
                        for (key, source) in lib_sources.iter() {
                            if let Some(source) = source {
                                rt.eval(&format!("<lib:{}>", key), source);
                            } else {
                                warn!("app {} requested missing lib: {}", &rel.app_id, &key);
                            }
                        }
                    }
                    Err(e) => warn!("error loading libs for app {}: {}", &rel.app, e),
                }
            }

            rt.eval("app.js", &rel.source);
            let app = rel.app;
            let app_id = rel.app_id;
            let version = rel.version;

            // TODO: ughh, refactor!
            // let _key2 = key.clone();
            tokio::spawn(rt.run().then(move |res: Result<(), _>| {
                if let Err(_) = res {
                    error!("app: {} ({}) v{} ended abruptly", app, app_id, version);
                }
                // runtimes.write().unwrap().remove(&key2);
                Ok(())
            }));
            writer.insert(key.clone(), rt);
        }

        let runtimes = runtimes.read().unwrap(); // TODO: no unwrap
        match runtimes.get(&key) {
            Some(rt) => Ok(Some(rt.ptr.to_runtime())),
            None => Ok(None),
        }
    }
}
