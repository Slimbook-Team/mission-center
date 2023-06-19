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

const SUBCOMMAND_APPS_PROCESSES: &str = "apps-processes";
const SUBCOMMANDS: &[&str] = &[SUBCOMMAND_APPS_PROCESSES];
const SUBCOMMAND_HANDLERS: &[fn(std::env::ArgsOs)] = &[sub_apps_processes];

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
        for (i, valid_subcommand) in SUBCOMMANDS.iter().enumerate() {
            if subcommand == *valid_subcommand {
                let mut args = std::env::args_os();
                args.next(); // skip program name
                SUBCOMMAND_HANDLERS[i](args);
                std::process::exit(0);
            }
        }
    }

    eprintln!("No or invalid subcommand provided. Valid options are:");
    for subcommand in SUBCOMMANDS {
        eprintln!("  {}", subcommand);
    }
    std::process::exit(1);
}
