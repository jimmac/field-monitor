/* Copyright 2024 Marco Köpcke
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use anyhow::anyhow;
use futures::future::LocalBoxFuture;
use gettextrs::gettext;
use glib;
use glib::prelude::*;
use glib::subclass::prelude::*;
use log::{debug, error};

use crate::connection::{ConnectionError, ConnectionMetadata, ConnectionResult, ServerMap};
use crate::connection::configuration::ConnectionConfiguration;
use crate::connection::types::{Connection, ConnectionProvider};

mod imp {
    use super::*;

    #[derive(Default, glib::Properties)]
    #[properties(wrapper_type = super::ConnectionInstance)]
    pub struct ConnectionInstance {
        #[property(get, set)]
        pub title: RefCell<Option<String>>,
        #[property(get, construct_only)]
        pub connection_id: RefCell<String>,
        pub configuration: RefCell<Option<ConnectionConfiguration>>,
        pub provider: RefCell<Option<Rc<Box<dyn ConnectionProvider>>>>,
        pub implementation: RefCell<Option<Box<dyn Connection>>>,
        pub load_error: RefCell<Option<Arc<ConnectionError>>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ConnectionInstance {
        const NAME: &'static str = "ConnectionInstance";
        type Type = super::ConnectionInstance;
        type ParentType = glib::Object;
    }

    #[glib::derived_properties]
    impl ObjectImpl for ConnectionInstance {}
}

glib::wrapper! {
    pub struct ConnectionInstance(ObjectSubclass<imp::ConnectionInstance>);
}

static NOT_INIT: &str = "ConnectionInstance was not properly initialized";

impl ConnectionInstance {
    pub async fn new(
        configuration: ConnectionConfiguration,
        provider: Rc<Box<dyn ConnectionProvider>>,
    ) -> Self {
        let slf_id = Arc::new(configuration.id().to_string());
        let slf: Self = glib::Object::builder()
            .property("connection-id", &*slf_id)
            .build();

        // Listen to own signals for debug purposes
        slf.connect_notify(
            Some("configuration"),
            glib::clone!(
                #[strong]
                slf_id,
                move |slf, _| {
                    let brw = slf.imp().configuration.borrow();
                    let (id, tag) = match brw.as_ref() {
                        Some(c) => (Some(c.id()), Some(c.tag())),
                        None => (None, None),
                    };
                    debug!(
                        "connection instance (orig ID: {} got new config (tag: {:?}, id: {:?}).",
                        slf_id, id, tag,
                    )
                }
            ),
        );

        let imp = slf.imp();
        imp.provider.replace(Some(provider));
        slf.set_configuration(configuration).await;
        slf
    }

    /// Changes the configuration and recreates the implementation.
    pub async fn set_configuration(&self, value: ConnectionConfiguration) {
        assert_eq!(value.id(), self.connection_id().as_str());

        let slf_imp = self.imp();
        let provider = slf_imp.provider.borrow().as_ref().expect(NOT_INIT).clone();
        match provider.load_connection(value.clone()).await {
            Ok(implementation) => {
                self.set_title(implementation.metadata().title.as_str());
                slf_imp.implementation.replace(Some(implementation));
            }
            Err(err) => {
                self.set_title(provider.title().as_ref());
                error!(
                    "Failed to load connection implementation (provider: {}): {:?}",
                    provider.tag(),
                    err
                );
                slf_imp.load_error.replace(Some(Arc::new(err)));
            }
        }
        slf_imp.configuration.replace(Some(value));
    }

    pub fn configuration(&self) -> ConnectionConfiguration {
        self.imp().configuration.borrow().as_ref().unwrap().clone()
    }

    pub fn provider(&self) -> Rc<Box<dyn ConnectionProvider>> {
        self.imp().provider.borrow().as_ref().unwrap().clone()
    }

    pub fn provider_tag(&self) -> Option<String> {
        self.imp()
            .configuration
            .borrow()
            .as_ref()
            .map(|c| c.tag().to_string())
    }
}

impl Connection for ConnectionInstance {
    fn metadata(&self) -> ConnectionMetadata {
        let brw = self.imp().implementation.borrow();
        match brw.as_ref() {
            Some(implementation) => implementation.metadata(),
            None => ConnectionMetadata {
                title: gettext("Unknown Connection"),
                subtitle: None,
            },
        }
    }

    // TODO: This SHOULD be okay, since we will never re-enter this function during loading servers.
    #[allow(clippy::await_holding_refcell_ref)]
    fn servers(&self) -> LocalBoxFuture<ConnectionResult<ServerMap>> {
        Box::pin(async move {
            let brw = self.imp().implementation.borrow();
            match brw.as_ref() {
                Some(implementation) => implementation.servers().await,
                None => Err(ConnectionError::General(
                    Some(gettext("Unable to load connection.")),
                    self.imp()
                        .load_error
                        .borrow()
                        .as_ref()
                        .map(|e| anyhow::Error::new(e.clone()))
                        .unwrap_or_else(|| anyhow!(gettext("Unknown error"))),
                )),
            }
        })
    }
}