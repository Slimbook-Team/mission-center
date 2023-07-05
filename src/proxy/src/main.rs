/* proxy/src/main.rs
 *
 * Copyright 2023 Romeo Calota
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

use clap::Parser;

mod apps;
mod processes;

include!("../../common/util.rs");

#[derive(Parser, Debug)]
#[command(about, long_about = None)]
struct ArgsAppsProcesses {
    #[arg(short, long)]
    process_cache: String,
}

const SUBCOMMANDS: &[(&str, fn(std::env::ArgsOs))] = &[("apps-processes", sub_apps_processes)];

fn sub_apps_processes(args: std::env::ArgsOs) {
    use std::io::Write;

    let args = ArgsAppsProcesses::parse_from(args);

    let processes = processes::load_stats_from_cache(&args.process_cache).unwrap_or_default();
    let processes = processes::load_process_list(processes);
    processes::save_stats_to_cache(&args.process_cache, processes.values()).unwrap();

    let mut stdout = std::io::stdout().lock();
    stdout.write(to_binary(&processes.len())).unwrap();
    for p in processes.values() {
        p.serialize(&mut stdout).unwrap();
    }

    let apps = apps::installed_apps();
    stdout.write(to_binary(&apps.len())).unwrap();
    for app in apps {
        app.serialize(&mut stdout).unwrap();
    }
}

fn main() {
    let mut args = std::env::args_os();
    args.next(); // skip program name
    let subcommand = args.next();

    if let Some(subcommand) = subcommand {
        let subcommand = subcommand.to_string_lossy();
        for (valid_subcommand, subcommand_handler) in SUBCOMMANDS.iter() {
            if subcommand == *valid_subcommand {
                let mut args = std::env::args_os();
                args.next(); // skip program name
                subcommand_handler(args);
                std::process::exit(0);
            }
        }
    }

    eprintln!("No or invalid subcommand provided. Valid options are:");
    for (subcommand, _) in SUBCOMMANDS {
        eprintln!("  {}", subcommand);
    }
    std::process::exit(1);
}
