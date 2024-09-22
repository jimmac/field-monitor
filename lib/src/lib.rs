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
use anyhow::anyhow;
use gettextrs::gettext;

pub use secrets::ManagesSecrets;

use crate::connection::ConnectionError;

#[macro_use]
mod macros;
pub mod adapter;
pub mod busy;
pub mod connection;
pub mod gtk;
mod secrets;

pub fn config_error(connection_title: Option<String>) -> ConnectionError {
    ConnectionError::General(
        connection_title,
        anyhow!(gettext("The connection configuration is invalid")),
    )
}
