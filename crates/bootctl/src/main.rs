// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::HashMap;
use std::env;
use std::process::ExitCode;

use gcboot_core::{DeviceEvidence, TargetAssessment, plan_layout};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("version") => {
            println!("bootctl {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        Some("plan-device") => plan_device(args.collect()),
        Some("help") | Some("--help") | Some("-h") | None => {
            print_help();
            Ok(())
        }
        Some(command) => Err(format!("unknown command: {command}")),
    }
}

fn plan_device(arguments: Vec<String>) -> Result<(), String> {
    let options = parse_options(arguments)?;

    let evidence = DeviceEvidence {
        device_path: required(&options, "device")?.to_owned(),
        size_bytes: required(&options, "size-bytes")?
            .parse::<u64>()
            .map_err(|_| "--size-bytes must be an unsigned integer".to_owned())?,
        removable: parse_yes_no(required(&options, "removable")?, "removable")?,
        contains_mounted_root: parse_yes_no(required(&options, "root-mounted")?, "root-mounted")?,
        contains_mounted_boot: parse_yes_no(required(&options, "boot-mounted")?, "boot-mounted")?,
        read_only: parse_yes_no(required(&options, "read-only")?, "read-only")?,
    };

    println!("GoreeCloud Boot device planner");
    println!("mode: NON-DESTRUCTIVE DEVELOPMENT PLAN");
    println!("device path label: {}", evidence.device_path);
    println!("supplied capacity: {} bytes", evidence.size_bytes);
    println!("note: this command does not inspect or modify the supplied path");

    let assessment = TargetAssessment::evaluate(&evidence);
    if !assessment.eligible {
        println!("eligible: no");
        for reason in assessment.reasons {
            println!("reject: {reason}");
        }
        return Err("target rejected by the current safety policy".to_owned());
    }

    println!("eligible: yes (planning only; not write authorization)");

    let layout = plan_layout(evidence.size_bytes).map_err(|error| error.to_string())?;
    print_partition(&layout.gcboot);
    print_partition(&layout.gcdata);

    Ok(())
}

fn print_partition(partition: &gcboot_core::PartitionPlan) {
    println!(
        "partition {}: filesystem={} start={} end={} size={} bytes",
        partition.label,
        partition.intended_filesystem,
        partition.start_bytes,
        partition.end_bytes,
        partition.size_bytes()
    );
}

fn parse_options(arguments: Vec<String>) -> Result<HashMap<String, String>, String> {
    if arguments.len() % 2 != 0 {
        return Err("every option must have a value".to_owned());
    }

    let mut options = HashMap::new();
    for pair in arguments.chunks_exact(2) {
        let key = pair[0]
            .strip_prefix("--")
            .ok_or_else(|| format!("expected option beginning with --, got {}", pair[0]))?;

        if key.is_empty() {
            return Err("option name cannot be empty".to_owned());
        }
        if options.insert(key.to_owned(), pair[1].clone()).is_some() {
            return Err(format!("duplicate option: --{key}"));
        }
    }

    Ok(options)
}

fn required<'a>(options: &'a HashMap<String, String>, key: &str) -> Result<&'a str, String> {
    options
        .get(key)
        .map(String::as_str)
        .ok_or_else(|| format!("missing required option --{key}"))
}

fn parse_yes_no(value: &str, name: &str) -> Result<bool, String> {
    match value {
        "yes" => Ok(true),
        "no" => Ok(false),
        _ => Err(format!("--{name} must be 'yes' or 'no'")),
    }
}

fn print_help() {
    println!(
        "\
GoreeCloud Boot development CLI

USAGE:
  bootctl version
  bootctl plan-device --device PATH --size-bytes BYTES \\
    --removable yes|no --root-mounted yes|no --boot-mounted yes|no --read-only yes|no

SAFETY:
  plan-device does not inspect, open, partition, format, or write the supplied device path.
  Its inputs are development evidence only and are not sufficient authorization for a future
  destructive operation."
    );
}
