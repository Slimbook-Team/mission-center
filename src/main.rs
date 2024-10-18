/* main.rs
 *
 * Copyright 2024 Romeo Calota
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
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */
use std::{
    env,
    path::{Path, PathBuf},
    sync::{Arc, OnceLock},
};

use gettextrs::{bind_textdomain_codeset, bindtextdomain, textdomain};
use gtk::{gio, prelude::*};
use lazy_static::lazy_static;

use application::MissionCenterApplication;
use config::{GETTEXT_PACKAGE, LOCALEDIR, PKGDATADIR};
use window::MissionCenterWindow;

mod application;
mod apps_page;
mod i18n;
mod performance_page;
mod preferences;
mod services_page;
mod sys_info_v2;
mod window;

#[macro_export]
macro_rules! glib_clone {
    ($var:expr) => {{
        unsafe { &*$var.as_ptr() }.clone()
    }};
}

mod config {
    include!(concat!(env!("BUILD_ROOT"), "/src/config.rs"));
}

lazy_static! {
    pub static ref HW_DB_DIR: String = {
        if let Ok(path) = std::env::var("HW_DB_DIR") {
            path
        } else {
            PKGDATADIR.to_owned()
        }
    };
}

fn user_home() -> &'static Path {
    static HOME: OnceLock<PathBuf> = OnceLock::new();

    HOME.get_or_init(|| {
        env::var_os("HOME")
            .or(env::var_os("USERPROFILE"))
            .map(|v| PathBuf::from(v))
            .unwrap_or(if cfg!(windows) {
                "C:/".into()
            } else {
                "/tmp".into()
            })
    })
    .as_path()
}

pub fn to_human_readable_adv(
    value: f32,
    divisor: f32,
    min_type: usize,
) -> (f32, &'static str, usize) {
    const UNITS: [&'static str; 9] = ["", "K", "M", "G", "T", "P", "E", "Z", "Y"];

    let mut index = 0;
    let mut value = value;

    // Only display bit/byte values in the given range or higher, not individual bits/bytes
    // This sacrifices some precision for the sake of readability
    if divisor == 1024. {
        while index < min_type {
            value /= divisor;
            index += 1;
        }
    }

    while value >= divisor && index < UNITS.len() - 1 {
        value /= divisor;
        index += 1;
    }

    // Calculate number of decimals to display
    // Only display fractional values for bit/byte numbers in the defined range and bigger
    // This sacrifices some precision for the sake of readability
    let mut dec_to_display = 0;
    if index > min_type || (divisor == 1000.) {
        if value < 100.0 {
            dec_to_display = 1
        }
        if value < 10.0 {
            dec_to_display = 2
        }
    }

    (value, UNITS[index], dec_to_display)
}

pub fn to_human_readable(value: f32, divisor: f32) -> (f32, &'static str, usize) {
    return to_human_readable_adv(value, divisor, 1);
}

pub fn show_error_dialog_and_exit(message: &str) -> ! {
    use crate::i18n::*;
    use adw::prelude::*;

    let message = Arc::<str>::from(message);
    gtk::glib::idle_add_once(move || {
        let app_window = app!().window();

        let error_dialog = adw::MessageDialog::new(
            app_window.as_ref(),
            Some("A fatal error has occurred"),
            Some(message.as_ref()),
        );
        error_dialog.set_modal(true);
        error_dialog.add_responses(&[("close", &i18n("_Quit"))]);
        error_dialog.set_response_appearance("close", adw::ResponseAppearance::Destructive);
        error_dialog.connect_response(None, |dialog, _| {
            dialog.close();
            std::process::exit(-1);
        });
        error_dialog.present();
    });

    loop {}
}

fn main() {
    let home = user_home().to_string_lossy().to_string();
    let mut xdg_data_dirs = env::var_os("XDG_DATA_DIRS")
        .map(|str| str.to_string_lossy().to_string())
        .unwrap_or("/usr/share:/usr/local/share".into());
    xdg_data_dirs.push_str(&format!(":{home}/.local/share"));
    env::set_var("XDG_DATA_DIRS", xdg_data_dirs.replace('~', &home));

    bindtextdomain(GETTEXT_PACKAGE, LOCALEDIR).expect("Unable to bind the text domain");
    bind_textdomain_codeset(GETTEXT_PACKAGE, "UTF-8")
        .expect("Unable to set the text domain encoding");
    textdomain(GETTEXT_PACKAGE).expect("Unable to switch to the text domain");

    let gresource_dir = if let Ok(gresource_dir) = std::env::var("MC_RESOURCE_DIR") {
        gresource_dir
    } else {
        PKGDATADIR.to_owned()
    };

    let resources = gio::Resource::load(gresource_dir + "/missioncenter.gresource")
        .expect("Could not load resources");
    gio::resources_register(&resources);

    let app = MissionCenterApplication::new(
        "io.missioncenter.MissionCenter",
        &gio::ApplicationFlags::empty(),
    );
    gtk::Application::set_default(app.upcast_ref::<gtk::Application>());

    let exit_code = app.run();
    std::process::exit(exit_code.into());
}
