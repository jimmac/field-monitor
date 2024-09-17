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
use std::borrow::Cow;
use std::iter;
use std::ops::Deref;

use adw::prelude::*;
use futures::future::try_join_all;
use gettextrs::gettext;
use gtk::{gio, glib};
use itertools::Itertools;

pub use action_row::FieldMonitorCLServerEntryActionRow;
pub use expander_row::FieldMonitorCLServerEntryExpanderRow;
use libfieldmonitor::connection::{
    ConnectionResult, IconSpec, ServerConnection, ServerMap, ServerMetadata,
};

use crate::application::FieldMonitorApplication;
use crate::i18n::gettext_f;

mod action_row;
mod expander_row;

pub async fn new_server_entry_row(
    app: &FieldMonitorApplication,
    connection_id: String,
    server_path: Vec<String>,
    server: Box<dyn ServerConnection>,
) -> ConnectionResult<gtk::ListBoxRow> {
    let path = iter::once(connection_id.as_str())
        .chain(server_path.iter().map(Deref::deref))
        .join("/");
    let metadata = server.metadata();

    let subservers = server.servers().await?;

    if subservers.is_empty() {
        load_single_server_row(app, &connection_id, &path, &metadata, server)
            .await
            .map(Cast::upcast)
    } else {
        load_multi_server_row(app, &connection_id, &path, &metadata, server, subservers)
            .await
            .map(Cast::upcast)
    }
}

async fn load_single_server_row(
    app: &FieldMonitorApplication,
    connection_id: &str,
    path: &str,
    metadata: &ServerMetadata,
    server: Box<dyn ServerConnection>,
) -> ConnectionResult<FieldMonitorCLServerEntryActionRow> {
    let row: FieldMonitorCLServerEntryActionRow = glib::Object::builder()
        .property("application", app)
        .property("connection-id", connection_id)
        .property("path", path)
        .property("title", &metadata.title)
        .property("subtitle", &metadata.subtitle)
        .build();
    row.set_server(server).await;

    finish_load(&row, metadata).await;

    Ok(row)
}

async fn load_multi_server_row(
    app: &FieldMonitorApplication,
    connection_id: &str,
    path: &str,
    metadata: &ServerMetadata,
    server: Box<dyn ServerConnection>,
    subservers: ServerMap,
) -> ConnectionResult<FieldMonitorCLServerEntryExpanderRow> {
    let row: FieldMonitorCLServerEntryExpanderRow = glib::Object::builder()
        .property("application", app)
        .property("connection-id", connection_id)
        .property("path", path)
        .property("title", &metadata.title)
        .property("subtitle", &metadata.subtitle)
        .build();
    row.set_server(server).await;

    let mut load_subservers = Vec::with_capacity(subservers.len());

    for (server_id, server) in subservers {
        let app = app.clone();
        let connection_id = connection_id.to_string();
        let path = path.to_string();
        load_subservers.push(glib::clone!(
            #[strong]
            row,
            async move {
                new_server_entry_row(
                    &app,
                    connection_id,
                    path.split('/')
                        .skip(1)
                        .chain(iter::once(server_id.as_ref()))
                        .map(ToString::to_string)
                        .collect(),
                    server,
                )
                .await
            }
        ));
    }

    let all_servers = try_join_all(load_subservers.into_iter()).await?;
    for server in all_servers {
        row.add_row(&server)
    }

    finish_load(&row, metadata).await;

    Ok(row)
}

async fn finish_load(row: &impl ServerEntry, metadata: &ServerMetadata) {
    row.add_css_class("serverrow");
    add_icon(row, metadata);
    row.with_server_if_exists(|server| {
        let adapters = server.supported_adapters();

        let button = if adapters.len() == 1 {
            let adapter = adapters.into_iter().next().unwrap();
            Some(make_single_connect_button(&row.path(), adapter))
        } else if !adapters.is_empty() {
            Some(make_multi_connection_button(&row.path(), adapters))
        } else {
            None
        };

        if let Some(button) = button {
            row.set_activatable_widget(Some(&button));
            row.add_suffix(&button);
        }
    })
    .await;
}

const DEFAULT_GENERIC_ICON: &str = "network-server-symbolic";

fn add_icon(row: &impl ServerEntry, metadata: &ServerMetadata) {
    let wdg = match &metadata.icon {
        IconSpec::Default => gtk::Image::builder()
            .icon_name(DEFAULT_GENERIC_ICON)
            .build()
            .upcast(),
        IconSpec::None => gtk::Box::builder().width_request(16).build().upcast(),
        IconSpec::Named(name) => gtk::Image::builder()
            .icon_name(name.deref())
            .build()
            .upcast(),
        IconSpec::Custom(factory) => factory(metadata),
    };

    let wdg = add_status(wdg, metadata);
    row.add_prefix(&wdg);
}

fn add_status(child_wdgt: gtk::Widget, metadata: &ServerMetadata) -> gtk::Widget {
    match metadata.is_online {
        Some(status) => {
            let parent = gtk::Box::builder()
                .orientation(gtk::Orientation::Vertical)
                .valign(gtk::Align::Center)
                .vexpand(true)
                .spacing(4)
                .build();

            let (class, tooltip_text) = if status {
                ("success", "Online")
            } else {
                ("error", "Offline")
            };

            let status_icon = gtk::Image::builder()
                .pixel_size(12)
                .icon_name("big-dot-symbolic")
                .css_classes([class])
                .tooltip_text(tooltip_text)
                .halign(gtk::Align::Center)
                .build();

            parent.append(&status_icon);
            parent.append(&child_wdgt);
            parent.upcast()
        }
        None => child_wdgt,
    }
}

fn make_multi_connection_button(path: &str, adapters: Vec<(Cow<str>, Cow<str>)>) -> gtk::Widget {
    let menu = gio::Menu::new();
    for (adapter_id, adapter_label) in adapters {
        let action_target = (path, &*adapter_id).to_variant();
        menu.append(
            Some(&*adapter_label),
            Some(
                gio::Action::print_detailed_name("app.connect-to-server", Some(&action_target))
                    .as_str(),
            ),
        );
    }

    gtk::MenuButton::builder()
        .menu_model(&menu)
        .icon_name("display-with-window-symbolic")
        .tooltip_text(gettext("Connect"))
        .valign(gtk::Align::Center)
        .css_classes(["flat"])
        .build()
        .upcast()
}

fn make_single_connect_button(
    path: &str,
    (adapter_id, adapter_label): (Cow<str>, Cow<str>),
) -> gtk::Widget {
    gtk::Button::builder()
        .action_name("app.connect-to-server")
        .action_target(&(path, &*adapter_id).to_variant())
        .icon_name("display-with-window-symbolic")
        .tooltip_text(gettext_f(
            "Connect via {adapter}",
            &[("adapter", &adapter_label)],
        ))
        .valign(gtk::Align::Center)
        .css_classes(["flat"])
        .build()
        .upcast()
}

trait ServerEntry {
    async fn set_server(&self, server: Box<dyn ServerConnection>);
    fn add_prefix(&self, widget: &impl IsA<gtk::Widget>);
    fn add_suffix(&self, widget: &impl IsA<gtk::Widget>);
    fn add_css_class(&self, class_name: &str);
    fn set_activatable_widget(&self, widget: Option<&impl IsA<gtk::Widget>>);
    fn path(&self) -> String;
    async fn with_server_if_exists<F>(&self, cb: F)
    where
        F: FnOnce(&dyn ServerConnection);
}
