// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::{
    env,
    fs::{File, OpenOptions},
    io::{Read, Write},
    os::unix::io::AsRawFd,
    process::exit,
    str::FromStr,
};

use clap::{Arg, Command};
use env_logger::Builder;
use log::LevelFilter;
use nix::{
    fcntl::{flock, FlockArg},
    unistd::getpid,
};

use stratisd::stratis::{run, StratisError, StratisResult, VERSION};

const STRATISD_PID_PATH: &str = "/run/stratisd.pid";
const STRATISD_MIN_PID_PATH: &str = "/run/stratisd-min.pid";

/// Configure and initialize the logger.
/// If optional log_level argument is set, use that to set the log level
/// for both stratisd and libstratis. Otherwise, read log configuration
/// parameters from the environment if RUST_LOG is set. Otherwise, just
/// accept the default configuration, which is to log at the severity of
/// error only.
fn initialize_log(log_level: Option<&str>) {
    let mut builder = Builder::new();

    if let Some(log_level) = log_level {
        builder.filter(
            Some("stratisd"),
            LevelFilter::from_str(log_level)
                .expect("argument parser only accepts valid log levels"),
        );
    } else if let Ok(s) = env::var("RUST_LOG") {
        builder.parse_filters(&s);
    }

    builder.init()
}

/// To ensure only one instance of stratisd runs at a time, acquire an
/// exclusive lock. Return an error if lock attempt fails.
fn trylock_pid_file() -> StratisResult<File> {
    let mut f = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(STRATISD_PID_PATH)
        .map_err(|err| {
            StratisError::Chained(
                format!("Failed to create or open the stratisd PID file at {STRATISD_PID_PATH}"),
                Box::new(StratisError::from(err)),
            )
        })?;
    let stratisd_file = match flock(f.as_raw_fd(), FlockArg::LockExclusiveNonblock) {
        Ok(_) => {
            f.write_all(getpid().to_string().as_bytes())?;
            Ok(f)
        }
        Err(_) => {
            let mut buf = String::new();

            if f.read_to_string(&mut buf).is_err() {
                buf = "<unreadable>".to_string();
            }

            Err(StratisError::Msg(format!(
                "Daemon already running with supposed pid: {buf}"
            )))
        }
    };

    let f = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(STRATISD_MIN_PID_PATH)
        .map_err(|err| {
            StratisError::Chained(
                format!(
                    "Failed to create or open the stratisd-min PID file at {STRATISD_MIN_PID_PATH}"
                ),
                Box::new(StratisError::from(err)),
            )
        })?;
    match flock(f.as_raw_fd(), FlockArg::LockExclusive) {
        Ok(_) => drop(f),
        Err(e) => {
            return Err(StratisError::Chained(
                "Failed to wait on stratisd-min to exit".to_string(),
                Box::new(StratisError::from(e)),
            ))
        }
    };

    stratisd_file
}

fn main() {
    let matches = Command::new("stratisd")
        .version(VERSION)
        .about("Stratis storage management")
        .arg(Arg::new("sim").long("sim").help("Use simulator engine"))
        .arg(
            Arg::new("log-level")
                .forbid_empty_values(true)
                .long("log-level")
                .possible_values(["trace", "debug", "info", "warn", "error"])
                .help("Sets level for generation of log messages."),
        )
        .get_matches();

    // Using a let-expression here so that the scope of the lock file
    // is the rest of the block.
    let lock_file = trylock_pid_file();

    let result = {
        match lock_file {
            Err(err) => Err(err),
            Ok(_) => {
                initialize_log(matches.value_of("log-level"));
                run(matches.is_present("sim"))
            }
        }
    };

    if let Err(err) = result {
        eprintln!("{err}");
        exit(1);
    } else {
        exit(0);
    }
}
