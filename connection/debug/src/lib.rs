use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use adw::prelude::AdwDialogExt;
use anyhow::anyhow;
use async_std::task::sleep;
use futures::future::LocalBoxFuture;
use gtk::prelude::*;
use indexmap::IndexMap;
use rand::{Rng, thread_rng};

use libfieldmonitor::adapter::rdp::RdpAdapter;
use libfieldmonitor::adapter::spice::SpiceAdapter;
use libfieldmonitor::adapter::types::Adapter;
use libfieldmonitor::adapter::vnc::VncAdapter;
use libfieldmonitor::connection::*;

use crate::behaviour_preferences::{DebugBehaviour, DebugBehaviourPreferences};
use crate::preferences::{DebugConfiguration, DebugMode, DebugPreferences};
use crate::vte_adapter::DebugVteAdapter;

mod behaviour_preferences;
mod preferences;
mod vte_adapter;

pub struct DebugConnectionProviderConstructor;

impl ConnectionProviderConstructor for DebugConnectionProviderConstructor {
    fn new(&self) -> Box<dyn ConnectionProvider> {
        Box::new(DebugConnectionProvider {})
    }
}

pub struct DebugConnectionProvider {}

impl ConnectionProvider for DebugConnectionProvider {
    fn tag(&self) -> &'static str {
        "debug"
    }

    fn title(&self) -> Cow<'static, str> {
        Cow::Borrowed("Debug Connection")
    }

    fn title_plural(&self) -> Cow<str> {
        Cow::Borrowed("Debug Connections")
    }

    fn add_title(&self) -> Cow<str> {
        Cow::Borrowed("Add Debug Connection")
    }

    fn description(&self) -> Cow<str> {
        Cow::Borrowed("Debug Connection")
    }

    fn preferences(&self, configuration: Option<&ConnectionConfiguration>) -> gtk::Widget {
        DebugPreferences::new(configuration).upcast()
    }

    fn update_connection(
        &self,
        preferences: gtk::Widget,
        mut configuration: ConnectionConfiguration,
    ) -> LocalBoxFuture<anyhow::Result<ConnectionConfiguration>> {
        Box::pin(async {
            sleep(Duration::from_millis(thread_rng().gen_range(100..1200))).await;

            let preferences = preferences
                .downcast::<DebugPreferences>()
                .expect("update_connection got invalid widget type");

            // Update general config
            configuration.set_title(&preferences.title());
            configuration.set_mode(preferences.mode());
            configuration.set_vnc_adapter_enable(preferences.vnc_adapter_enable());
            configuration.set_vnc_host(&preferences.vnc_host());
            configuration.set_vnc_user(&preferences.vnc_user());
            configuration.set_vnc_password(&preferences.vnc_password());
            configuration.set_rdp_adapter_enable(preferences.rdp_adapter_enable());
            configuration.set_rdp_host(&preferences.rdp_host());
            configuration.set_rdp_user(&preferences.rdp_user());
            configuration.set_rdp_password(&preferences.rdp_password());
            configuration.set_spice_adapter_enable(preferences.spice_adapter_enable());
            configuration.set_vte_adapter_enable(preferences.vte_adapter_enable());

            // Update credentials
            let credentials = preferences.behaviour();
            self.store_credentials(credentials.clone().upcast(), configuration)
                .await
        })
    }

    fn configure_credentials(&self, configuration: &ConnectionConfiguration) -> gtk::Widget {
        DebugBehaviourPreferences::new(Some(configuration)).upcast()
    }

    fn store_credentials(
        &self,
        preferences: gtk::Widget,
        mut configuration: ConnectionConfiguration,
    ) -> LocalBoxFuture<anyhow::Result<ConnectionConfiguration>> {
        Box::pin(async move {
            sleep(Duration::from_millis(thread_rng().gen_range(100..400))).await;

            let preferences = preferences
                .downcast::<DebugBehaviourPreferences>()
                .expect("store_credentials got invalid widget type");

            configuration.set_load_servers_behaviour(preferences.load_servers_behaviour());
            configuration.set_connect_behaviour(preferences.connect_behaviour());
            Ok(configuration)
        })
    }

    fn load_connection(
        &self,
        configuration: ConnectionConfiguration,
    ) -> LocalBoxFuture<ConnectionResult<Box<dyn Connection>>> {
        Box::pin(async move {
            sleep(Duration::from_millis(thread_rng().gen_range(100..1200))).await;

            let title = configuration.title().to_string();

            let subtitle = match configuration.mode() {
                DebugMode::Single => None,
                DebugMode::Multi => Some("multi mode".to_string()),
                DebugMode::Complex => Some("complex mode".to_string()),
                DebugMode::NoServers => Some("no servers".to_string()),
            };

            let c: Box<dyn Connection> =
                Box::new(DebugConnection::new(title, subtitle, configuration));
            Ok(c)
        })
    }
}

#[derive(Clone)]
pub struct DebugConnection {
    title: String,
    subtitle: Option<String>,
    config: ConnectionConfiguration,
}

impl Connection for DebugConnection {
    fn metadata(&self) -> ConnectionMetadata {
        ConnectionMetadataBuilder::default()
            .title(self.title.clone())
            .subtitle(self.subtitle.clone())
            .build()
            .unwrap()
    }

    fn servers(&self) -> LocalBoxFuture<ConnectionResult<ServerMap>> {
        Box::pin(async move {
            sleep(Duration::from_millis(thread_rng().gen_range(100..1200))).await;

            match self.config.load_servers_behaviour() {
                DebugBehaviour::Ok => {
                    let mut hm: IndexMap<Cow<_>, Box<dyn ServerConnection>> = IndexMap::new();

                    match self.config.mode() {
                        DebugMode::Single => {
                            hm.insert(
                                "server1".into(),
                                Box::new(DebugConnectionServer::new(
                                    ServerMetadataBuilder::default()
                                        .title("Debug Server".to_string())
                                        .build()
                                        .unwrap(),
                                    self.config.clone(),
                                )),
                            );
                        }
                        DebugMode::Multi => {
                            hm.insert(
                                "server1".into(),
                                Box::new(DebugConnectionServer::new(
                                    ServerMetadataBuilder::default()
                                        .title("Debug Server".to_string())
                                        .subtitle(Some("Is the first server".to_string()))
                                        .build()
                                        .unwrap(),
                                    self.config.clone(),
                                )),
                            );
                            hm.insert(
                                "server2".into(),
                                Box::new(DebugConnectionServer::new(
                                    ServerMetadataBuilder::default()
                                        .title("Server 2".to_string())
                                        .subtitle(Some("Has no icon".to_string()))
                                        .icon(IconSpec::None)
                                        .build()
                                        .unwrap(),
                                    self.config.clone(),
                                )),
                            );
                            hm.insert(
                                "server3".into(),
                                Box::new(DebugConnectionServer::new(
                                    ServerMetadataBuilder::default()
                                        .title("Server 3".to_string())
                                        .subtitle(Some("This is marked as offline".to_string()))
                                        .is_online(Some(false))
                                        .build()
                                        .unwrap(),
                                    self.config.clone(),
                                )),
                            );
                        }
                        DebugMode::Complex => {
                            let mut root1 = DebugConnectionServer::new(
                                ServerMetadataBuilder::default()
                                    .title("Root 1".to_string())
                                    .build()
                                    .unwrap(),
                                self.config.clone(),
                            );
                            let mut r1_level1_1 = DebugConnectionServer::new(
                                ServerMetadataBuilder::default()
                                    .title("R1 L1_1".to_string())
                                    .subtitle(Some("Has no icon".to_string()))
                                    .icon(IconSpec::None)
                                    .build()
                                    .unwrap(),
                                self.config.clone(),
                            );
                            let mut r1_level1_2 = DebugConnectionServer::new(
                                ServerMetadataBuilder::default()
                                    .title("R1 L1_2".to_string())
                                    .subtitle(Some("Is online".to_string()))
                                    .is_online(Some(true))
                                    .build()
                                    .unwrap(),
                                self.config.clone(),
                            );
                            let r1_level2_1 = DebugConnectionServer::new(
                                ServerMetadataBuilder::default()
                                    .title("R1 L1_2 L2_1".to_string())
                                    .subtitle(Some("Is offline".to_string()))
                                    .is_online(Some(false))
                                    .build()
                                    .unwrap(),
                                self.config.clone(),
                            );
                            r1_level1_2.add_server(r1_level2_1);
                            r1_level1_1.no_adapters();
                            r1_level1_2.expose_dummy_actions();
                            root1.add_server(r1_level1_1);
                            root1.add_server(r1_level1_2);
                            root1.expose_dummy_actions();

                            let mut root2 = DebugConnectionServer::new(
                                ServerMetadataBuilder::default()
                                    .title("Root 2".to_string())
                                    .subtitle(Some("Is online".to_string()))
                                    .is_online(Some(true))
                                    .build()
                                    .unwrap(),
                                self.config.clone(),
                            );
                            let mut r2_level1 = DebugConnectionServer::new(
                                ServerMetadataBuilder::default()
                                    .title("R2 1".to_string())
                                    .build()
                                    .unwrap(),
                                self.config.clone(),
                            );
                            let mut r2_level2 = DebugConnectionServer::new(
                                ServerMetadataBuilder::default()
                                    .title("R2 2".to_string())
                                    .subtitle(Some("has a named icon".to_string()))
                                    .icon(IconSpec::Named(Cow::Borrowed("go-home-symbolic")))
                                    .build()
                                    .unwrap(),
                                self.config.clone(),
                            );
                            let mut r2_level3 = DebugConnectionServer::new(
                                ServerMetadataBuilder::default()
                                    .title("R2 3".to_string())
                                    .subtitle(Some("has a custom widget icon".to_string()))
                                    .icon(IconSpec::Custom(Arc::new(Box::new(|_meta| {
                                        gtk::Spinner::builder().spinning(true).build().upcast()
                                    }))))
                                    .build()
                                    .unwrap(),
                                self.config.clone(),
                            );
                            let r2_level4 = DebugConnectionServer::new(
                                ServerMetadataBuilder::default()
                                    .title("R2 4".to_string())
                                    .build()
                                    .unwrap(),
                                self.config.clone(),
                            );
                            r2_level3.add_server(r2_level4);
                            r2_level2.add_server(r2_level3);
                            r2_level1.add_server(r2_level2);
                            r2_level1.no_adapters();
                            root2.add_server(r2_level1);

                            hm.insert("server1".into(), Box::new(root1));
                            hm.insert("server2".into(), Box::new(root2));
                        }
                        DebugMode::NoServers => {}
                    }

                    Ok(hm)
                }
                DebugBehaviour::AuthError => Err(ConnectionError::AuthFailed(
                    Some("debug auth failure (servers)".to_string()),
                    anyhow!("debug auth failure (servers)"),
                )),
                DebugBehaviour::GeneralError => Err(ConnectionError::General(
                    Some("debug general failure (servers)".to_string()),
                    anyhow!("debug general failure (servers)"),
                )),
            }
        })
    }

    fn actions<'a>(&self) -> ActionMap<'a> {
        match self.config.mode() {
            DebugMode::Complex => {
                let mut map: ActionMap = IndexMap::new();

                map.insert(
                    Cow::Borrowed("foobar"),
                    ServerAction::new(
                        "Show dialog".to_string(),
                        Box::new(|window, _toasts| {
                            Box::pin(async move {
                                adw::AlertDialog::builder()
                                    .title("Foobar")
                                    .build()
                                    .present(Some(&window))
                            })
                        }),
                    ),
                );

                map.insert(
                    Cow::Borrowed("bazbaz"),
                    ServerAction::new(
                        "Show toast".to_string(),
                        Box::new(|_window, toasts| {
                            Box::pin(async move {
                                sleep(Duration::from_secs(2)).await;
                                let toast =
                                    adw::Toast::builder().title("Foobar").timeout(10).build();
                                toasts.add_toast(toast);
                            })
                        }),
                    ),
                );

                map
            }
            _ => IndexMap::new(),
        }
    }
}

impl DebugConnection {
    fn new(title: String, subtitle: Option<String>, config: ConnectionConfiguration) -> Self {
        Self {
            title,
            subtitle,
            config,
        }
    }
}

#[derive(Clone)]
pub struct DebugConnectionServer {
    metadata: ServerMetadata,
    config: ConnectionConfiguration,
    servers: HashMap<Cow<'static, str>, DebugConnectionServer>,
    has_adapters: bool,
    has_actions: bool,
}

impl DebugConnectionServer {
    fn new(metadata: ServerMetadata, config: ConnectionConfiguration) -> Self {
        Self {
            metadata,
            config,
            servers: HashMap::new(),
            has_adapters: true,
            has_actions: false,
        }
    }

    fn add_server(&mut self, server: DebugConnectionServer) {
        self.servers
            .insert(Cow::Owned(server.metadata.title.clone()), server);
    }

    fn no_adapters(&mut self) {
        self.has_adapters = false;
    }

    fn expose_dummy_actions(&mut self) {
        self.has_actions = true;
    }
}

impl ServerConnection for DebugConnectionServer {
    fn metadata(&self) -> ServerMetadata {
        self.metadata.clone()
    }

    fn supported_adapters(&self) -> Vec<(Cow<str>, Cow<str>)> {
        if !self.has_adapters {
            return vec![];
        }
        let mut adapters = Vec::with_capacity(4);
        if self.config.vnc_adapter_enable() {
            adapters.push((VncAdapter::TAG, VncAdapter::label()));
        }
        if self.config.rdp_adapter_enable() {
            adapters.push((RdpAdapter::TAG, RdpAdapter::label()));
        }
        if self.config.spice_adapter_enable() {
            adapters.push((SpiceAdapter::TAG, SpiceAdapter::label()));
        }
        if self.config.vte_adapter_enable() {
            adapters.push((DebugVteAdapter::TAG, "VTE".into()));
        }
        adapters
    }

    fn create_adapter(
        &self,
        tag: &str,
    ) -> LocalBoxFuture<Result<Box<dyn Adapter>, ConnectionError>> {
        Box::pin(async move { todo!() })
    }

    fn servers(&self) -> LocalBoxFuture<ConnectionResult<ServerMap>> {
        Box::pin(async move {
            sleep(Duration::from_millis(thread_rng().gen_range(50..200))).await;

            let mut hm: IndexMap<Cow<_>, Box<dyn ServerConnection>> = IndexMap::new();

            for (name, server) in &self.servers {
                hm.insert(name.clone(), Box::new(server.clone()));
            }

            Ok(hm)
        })
    }

    fn actions<'a>(&self) -> ActionMap<'a> {
        if self.has_actions {
            let mut map: ActionMap = IndexMap::new();
            map.insert(
                Cow::Borrowed("foobar"),
                ServerAction::new(
                    "Show dialog".to_string(),
                    Box::new(|window, _toasts| {
                        Box::pin(async move {
                            adw::AlertDialog::builder()
                                .title("Foobar")
                                .build()
                                .present(Some(&window))
                        })
                    }),
                ),
            );

            map.insert(
                Cow::Borrowed("bazbaz"),
                ServerAction::new(
                    "Show toast".to_string(),
                    Box::new(|_window, toasts| {
                        Box::pin(async move {
                            sleep(Duration::from_secs(2)).await;
                            let toast = adw::Toast::builder().title("Foobar").timeout(10).build();
                            toasts.add_toast(toast);
                        })
                    }),
                ),
            );

            map
        } else {
            IndexMap::new()
        }
    }
}
