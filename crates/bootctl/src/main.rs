// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::HashMap;
use std::env;
use std::fs::{self, OpenOptions};
use std::io::{Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use gcboot_core::{
    DeviceEvidence, GptMetadataImage, SectorPartitionPlan, TargetAssessment, build_gpt_metadata,
    development_test_identity, plan_layout, plan_sector_layout,
};
#[cfg(target_os = "linux")]
use gcboot_core::{
    LinuxBlockDevice, LinuxDiscoveryReport, LinuxProbePaths, discover_linux_block_devices,
};

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
        Some("list-linux-devices") => list_linux_devices(args.collect()),
        Some("plan-linux-device") => plan_linux_device(args.collect()),
        Some("create-test-gpt-image") => create_test_gpt_image(args.collect()),
        Some("help") | Some("--help") | Some("-h") | None => {
            print_help();
            Ok(())
        }
        Some(command) => Err(format!("unknown command: {command}")),
    }
}

fn plan_device(arguments: Vec<String>) -> Result<(), String> {
    let options = parse_options(arguments)?;
    reject_unknown_options(
        &options,
        &[
            "device",
            "size-bytes",
            "removable",
            "root-mounted",
            "boot-mounted",
            "read-only",
        ],
    )?;

    let evidence = DeviceEvidence {
        device_path: required(&options, "device")?.to_owned(),
        size_bytes: parse_u64(required(&options, "size-bytes")?, "size-bytes")?,
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
        print_rejections(&assessment);
        return Err("target rejected by the current safety policy".to_owned());
    }

    println!("eligible: yes (planning only; not write authorization)");

    let layout = plan_layout(evidence.size_bytes).map_err(|error| error.to_string())?;
    print_partition(&layout.gcboot);
    print_partition(&layout.gcdata);

    Ok(())
}

#[cfg(target_os = "linux")]
fn list_linux_devices(arguments: Vec<String>) -> Result<(), String> {
    if !arguments.is_empty() {
        return Err("list-linux-devices does not accept arguments".to_owned());
    }

    let report = discover_linux_block_devices(&LinuxProbePaths::default())
        .map_err(|error| format!("Linux device discovery failed: {error}"))?;
    print_discovery_warnings(&report);

    println!("GoreeCloud Boot Linux device discovery");
    println!("mode: READ-ONLY METADATA DISCOVERY");
    println!("devices: {}", report.devices.len());

    for device in &report.devices {
        print_linux_device(device);
    }

    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn list_linux_devices(_arguments: Vec<String>) -> Result<(), String> {
    Err("list-linux-devices is available only on Linux".to_owned())
}

#[cfg(target_os = "linux")]
fn plan_linux_device(arguments: Vec<String>) -> Result<(), String> {
    let options = parse_options(arguments)?;
    reject_unknown_options(&options, &["device"])?;
    let requested = Path::new(required(&options, "device")?);

    let report = discover_linux_block_devices(&LinuxProbePaths::default())
        .map_err(|error| format!("Linux device discovery failed: {error}"))?;
    print_discovery_warnings(&report);

    let matches: Vec<&LinuxBlockDevice> = report
        .devices
        .iter()
        .filter(|device| {
            device.devnode.as_path() == requested
                || device
                    .persistent_aliases
                    .iter()
                    .any(|alias| alias.as_path() == requested)
        })
        .collect();

    let device = match matches.as_slice() {
        [device] => *device,
        [] => {
            return Err(format!(
                "no discovered whole block device matches {}",
                requested.display()
            ));
        }
        _ => return Err("device selector matched more than one discovered device".to_owned()),
    };

    println!("GoreeCloud Boot discovered-device planner");
    println!("mode: READ-ONLY DISCOVERY + NON-DESTRUCTIVE PLAN");
    print_linux_device(device);

    let assessment = device.assessment();
    if !assessment.eligible {
        return Err("target rejected by the current safety policy".to_owned());
    }

    let sector_layout = plan_sector_layout(device.size_bytes, device.logical_block_size)
        .map_err(|error| error.to_string())?;
    println!("sector-aware plan:");
    print_sector_partition(&sector_layout.gcboot, sector_layout.logical_block_size);
    print_sector_partition(&sector_layout.gcdata, sector_layout.logical_block_size);
    println!(
        "revalidation token: {:?}",
        device.revalidation_token()
    );
    println!("note: no device node was opened for writing");

    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn plan_linux_device(_arguments: Vec<String>) -> Result<(), String> {
    Err("plan-linux-device is available only on Linux".to_owned())
}

fn create_test_gpt_image(arguments: Vec<String>) -> Result<(), String> {
    let options = parse_options(arguments)?;
    reject_unknown_options(&options, &["output", "size-bytes", "logical-block-size"])?;

    let output = PathBuf::from(required(&options, "output")?);
    let size_bytes = parse_u64(required(&options, "size-bytes")?, "size-bytes")?;
    let logical_block_size = parse_u64(
        required(&options, "logical-block-size")?,
        "logical-block-size",
    )?;

    validate_test_image_output(&output)?;
    let layout = plan_sector_layout(size_bytes, logical_block_size)
        .map_err(|error| format!("invalid test-image layout: {error}"))?;
    let metadata = build_gpt_metadata(&layout, development_test_identity())
        .map_err(|error| format!("could not build GPT metadata: {error}"))?;

    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(&output)
        .map_err(|error| format!("could not create {}: {error}", output.display()))?;

    let result = write_and_verify_test_image(&mut file, &metadata);
    if let Err(error) = result {
        drop(file);
        let _ = fs::remove_file(&output);
        return Err(error);
    }

    println!("GoreeCloud Boot GPT development image created");
    println!("output: {}", output.display());
    println!("logical size: {} bytes", metadata.total_bytes);
    println!("logical block size: {} bytes", metadata.logical_block_size);
    println!(
        "GPT usable LBAs: {}..={}",
        metadata.first_usable_lba, metadata.last_usable_lba
    );
    println!("verification: generated GPT metadata read back successfully");
    println!("note: this is a sparse regular-file test image, not a bootable USB image");
    println!("note: no FAT32/exFAT filesystem or boot runtime is installed");

    Ok(())
}

fn write_and_verify_test_image(
    file: &mut std::fs::File,
    metadata: &GptMetadataImage,
) -> Result<(), String> {
    file.set_len(metadata.total_bytes)
        .map_err(|error| format!("could not size sparse test image: {error}"))?;

    for write in &metadata.writes {
        file.seek(SeekFrom::Start(write.offset_bytes))
            .map_err(|error| format!("could not seek test image: {error}"))?;
        file.write_all(&write.data)
            .map_err(|error| format!("could not write GPT test metadata: {error}"))?;
    }

    file.sync_all()
        .map_err(|error| format!("could not sync GPT test image: {error}"))?;

    let matches = metadata
        .matches_reader(file)
        .map_err(|error| format!("could not verify GPT test image: {error}"))?;
    if !matches {
        return Err("GPT test-image verification mismatch".to_owned());
    }

    Ok(())
}

fn validate_test_image_output(output: &Path) -> Result<(), String> {
    if output.as_os_str().is_empty() {
        return Err("--output must not be empty".to_owned());
    }

    let parent = output
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let canonical_parent = fs::canonicalize(parent)
        .map_err(|error| format!("could not resolve output parent {}: {error}", parent.display()))?;

    for blocked in [Path::new("/dev"), Path::new("/sys"), Path::new("/proc")] {
        if canonical_parent == blocked || canonical_parent.starts_with(blocked) {
            return Err(format!(
                "refusing to create a test image under protected pseudo-filesystem path {}",
                canonical_parent.display()
            ));
        }
    }

    Ok(())
}

#[cfg(target_os = "linux")]
fn print_linux_device(device: &LinuxBlockDevice) {
    println!();
    println!("device: {}", device.devnode.display());
    println!("  kernel name: {}", device.kernel_name);
    println!("  device number: {}", device.device_number);
    println!("  capacity: {} bytes", device.size_bytes);
    println!("  logical block size: {} bytes", device.logical_block_size);
    println!("  physical block size: {} bytes", device.physical_block_size);
    println!("  removable: {}", yes_no(device.removable));
    println!("  read-only: {}", yes_no(device.read_only));
    println!("  contains mounted root: {}", yes_no(device.contains_mounted_root));
    println!("  contains mounted boot: {}", yes_no(device.contains_mounted_boot));
    println!("  diskseq: {}", optional_u64(device.diskseq));
    println!("  vendor: {}", optional_text(device.vendor.as_deref()));
    println!("  model: {}", optional_text(device.model.as_deref()));
    println!("  serial: {}", optional_text(device.serial.as_deref()));
    println!("  WWID: {}", optional_text(device.wwid.as_deref()));
    println!(
        "  persistent identity: {}",
        optional_text(device.persistent_identity().as_deref())
    );
    for alias in &device.persistent_aliases {
        println!("  by-id alias: {}", alias.display());
    }

    let assessment = device.assessment();
    if assessment.eligible {
        println!("  eligible: yes (planning only; not write authorization)");
    } else {
        println!("  eligible: no");
        for reason in assessment.reasons {
            println!("  reject: {reason}");
        }
    }
}

#[cfg(target_os = "linux")]
fn print_discovery_warnings(report: &LinuxDiscoveryReport) {
    for warning in &report.warnings {
        eprintln!(
            "warning: {}: {}",
            warning.context.display(),
            warning.message
        );
    }
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

fn print_sector_partition(partition: &SectorPartitionPlan, logical_block_size: u64) {
    println!(
        "  {}: filesystem={} first_lba={} last_lba={} blocks={} bytes={}",
        partition.label,
        partition.intended_filesystem,
        partition.first_lba,
        partition.last_lba,
        partition.block_count(),
        partition.block_count().saturating_mul(logical_block_size)
    );
}

fn print_rejections(assessment: &TargetAssessment) {
    println!("eligible: no");
    for reason in &assessment.reasons {
        println!("reject: {reason}");
    }
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

fn reject_unknown_options(
    options: &HashMap<String, String>,
    allowed: &[&str],
) -> Result<(), String> {
    for key in options.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(format!("unknown option: --{key}"));
        }
    }

    Ok(())
}

fn required<'a>(options: &'a HashMap<String, String>, key: &str) -> Result<&'a str, String> {
    options
        .get(key)
        .map(String::as_str)
        .ok_or_else(|| format!("missing required option --{key}"))
}

fn parse_u64(value: &str, name: &str) -> Result<u64, String> {
    value
        .parse::<u64>()
        .map_err(|_| format!("--{name} must be an unsigned integer"))
}

fn parse_yes_no(value: &str, name: &str) -> Result<bool, String> {
    match value {
        "yes" => Ok(true),
        "no" => Ok(false),
        _ => Err(format!("--{name} must be 'yes' or 'no'")),
    }
}

fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}

fn optional_text(value: Option<&str>) -> &str {
    value.unwrap_or("not available")
}

fn optional_u64(value: Option<u64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "not available".to_owned())
}

fn print_help() {
    println!(
        "\
GoreeCloud Boot development CLI

USAGE:
  bootctl version
  bootctl plan-device --device PATH --size-bytes BYTES \\
    --removable yes|no --root-mounted yes|no --boot-mounted yes|no --read-only yes|no
  bootctl list-linux-devices
  bootctl plan-linux-device --device PATH
  bootctl create-test-gpt-image --output PATH --size-bytes BYTES \\
    --logical-block-size BYTES

SAFETY:
  plan-device evaluates caller-supplied development evidence only.
  list-linux-devices and plan-linux-device read Linux sysfs/mount metadata but do not open a
  block-device node for writing.
  create-test-gpt-image creates a new sparse regular file only, refuses existing output paths,
  and refuses output under /dev, /sys, or /proc.
  None of these commands authorize or perform physical removable-media provisioning."
    );
}
