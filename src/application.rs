/* application.rs
 *
 * Copyright 2024 Marco
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

use adw::prelude::AdwDialogExt;
use gtk::prelude::*;
use adw::subclass::prelude::*;
use gettextrs::gettext;
use gtk::{gio, glib};

use crate::config::VERSION;
use crate::StageScreenWindow;

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub struct StageScreenApplication {}

    #[glib::object_subclass]
    impl ObjectSubclass for StageScreenApplication {
        const NAME: &'static str = "StageScreenApplication";
        type Type = super::StageScreenApplication;
        type ParentType = adw::Application;
    }

    impl ObjectImpl for StageScreenApplication {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.setup_gactions();
            obj.set_accels_for_action("app.quit", &["<primary>q"]);
        }
    }

    impl ApplicationImpl for StageScreenApplication {
        // We connect to the activate callback to create a window when the application
        // has been launched. Additionally, this callback notifies us when the user
        // tries to launch a "second instance" of the application. When they try
        // to do that, we'll just present any existing window.
        fn activate(&self) {
            let application = self.obj();
            // Get the current window or create one if necessary
            let window = if let Some(window) = application.active_window() {
                window
            } else {
                let window = StageScreenWindow::new(&*application);
                window.upcast()
            };

            // Ask the window manager/compositor to present the window
            window.present();
        }
    }

    impl GtkApplicationImpl for StageScreenApplication {}
    impl AdwApplicationImpl for StageScreenApplication {}
}

glib::wrapper! {
    pub struct StageScreenApplication(ObjectSubclass<imp::StageScreenApplication>)
        @extends gio::Application, gtk::Application, adw::Application,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl StageScreenApplication {
    pub fn new(application_id: &str, flags: &gio::ApplicationFlags) -> Self {
        glib::Object::builder()
            .property("application-id", application_id)
            .property("flags", flags)
            .build()
    }

    fn setup_gactions(&self) {
        let quit_action = gio::ActionEntry::builder("quit")
            .activate(move |app: &Self, _, _| app.quit())
            .build();
        let about_action = gio::ActionEntry::builder("about")
            .activate(move |app: &Self, _, _| app.show_about())
            .build();
        self.add_action_entries([quit_action, about_action]);
    }

    fn show_about(&self) {
        let window = self.active_window().unwrap();
        let about = adw::AboutDialog::builder()
            .application_name(gettext("Stage Screen"))
            .application_icon("de.capypara.StageScreen")
            .license_type(gtk::License::Gpl30)
            .developer_name("Marco Köpcke")
            .version(VERSION)
            .developers(vec!["Marco Köpcke"])
            .copyright("© 2024 Marco Köpcke")
            .website("https://github.com/theCapypara/StageScreen")
            .issue_url("https://github.com/theCapypara/StageScreen/issues")
            .support_url("https://matrix.to/#/#stagescreen:matrix.org")
            .translator_credits(gettext("translator-credits"))
            .build();

        about.present(&window);
    }
}
