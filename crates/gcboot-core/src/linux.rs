// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::{BTreeSet, VecDeque};
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::{DeviceEvidence, TargetAssessment};

const SYSFS_CAPACITY_SECTOR_BYTES: u64 = 512;

/// Linux block-device major/minor identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DeviceNumber {
    pub major: u32,
    pub minor: u32,
}

impl DeviceNumber {
    pub fn parse(value: &str) -> Result<Self, DeviceNumberParseError> {
        let (major, minor) = value.trim().split_once(':').ok_or(DeviceNumberParseError)?;

        Ok(Self {
            major: major.parse::<u32>().map_err(|_| DeviceNumberParseError)?,
            minor: minor.parse::<u32>().map_err(|_| DeviceNumberParseError)?,
        })
    }
}

impl Display for DeviceNumber {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}:{}", self.major, self.minor)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeviceNumberParseError;

impl Display for DeviceNumberParseError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "device number must use MAJOR:MINOR decimal form")
    }
}

impl Error for DeviceNumberParseError {}

/// Filesystem locations used by Linux block-device discovery.
///
/// The default points at the running Linux system. Alternate roots make the
/// parser testable without depending on CI-runner hardware.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinuxProbePaths {
    pub sys_block_root: PathBuf,
    pub dev_root: PathBuf,
    pub dev_disk_by_id: PathBuf,
    pub mountinfo: PathBuf,
    pub proc_swaps: PathBuf,
}

impl Default for LinuxProbePaths {
    fn default() -> Self {
        Self {
            sys_block_root: PathBuf::from("/sys/block"),
            dev_root: PathBuf::from("/dev"),
            dev_disk_by_id: PathBuf::from("/dev/disk/by-id"),
            mountinfo: PathBuf::from("/proc/self/mountinfo"),
            proc_swaps: PathBuf::from("/proc/swaps"),
        }
    }
}

/// Read-only Linux evidence for one whole block device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinuxBlockDevice {
    pub kernel_name: String,
    pub devnode: PathBuf,
    pub device_number: DeviceNumber,
    pub partition_device_numbers: Vec<DeviceNumber>,
    pub topology_device_numbers: Vec<DeviceNumber>,
    pub mounted_topology_device_numbers: Vec<DeviceNumber>,
    pub active_swap_topology_device_numbers: Vec<DeviceNumber>,
    pub size_bytes: u64,
    pub logical_block_size: u64,
    pub physical_block_size: u64,
    pub removable: bool,
    pub read_only: bool,
    pub contains_mounted_root: bool,
    pub contains_mounted_boot: bool,
    pub contains_mounted_filesystem: bool,
    pub contains_active_swap: bool,
    pub diskseq: Option<u64>,
    pub vendor: Option<String>,
    pub model: Option<String>,
    pub serial: Option<String>,
    pub wwid: Option<String>,
    pub persistent_aliases: Vec<PathBuf>,
}

impl LinuxBlockDevice {
    #[must_use]
    pub fn evidence(&self) -> DeviceEvidence {
        DeviceEvidence {
            device_path: self.devnode.display().to_string(),
            size_bytes: self.size_bytes,
            removable: self.removable,
            contains_mounted_root: self.contains_mounted_root,
            contains_mounted_boot: self.contains_mounted_boot,
            contains_mounted_filesystem: self.contains_mounted_filesystem,
            contains_active_swap: self.contains_active_swap,
            read_only: self.read_only,
        }
    }

    #[must_use]
    pub fn assessment(&self) -> TargetAssessment {
        TargetAssessment::evaluate(&self.evidence())
    }

    /// Return the strongest currently discovered persistent identity evidence.
    ///
    /// This is evidence for later revalidation, not write authorization.
    #[must_use]
    pub fn persistent_identity(&self) -> Option<String> {
        if let Some(wwid) = &self.wwid {
            return Some(format!("wwid:{wwid}"));
        }

        self.persistent_aliases
            .first()
            .map(|alias| format!("by-id:{}", alias.display()))
    }

    #[must_use]
    pub fn revalidation_token(&self) -> LinuxRevalidationToken {
        LinuxRevalidationToken {
            device_number: self.device_number,
            diskseq: self.diskseq,
            size_bytes: self.size_bytes,
            logical_block_size: self.logical_block_size,
            removable: self.removable,
            read_only: self.read_only,
            persistent_identity: self.persistent_identity(),
            serial: self.serial.clone(),
            topology_device_numbers: self.topology_device_numbers.clone(),
            mounted_topology_device_numbers: self.mounted_topology_device_numbers.clone(),
            active_swap_topology_device_numbers: self.active_swap_topology_device_numbers.clone(),
        }
    }
}

/// A snapshot used to detect path reuse, device replacement, or relevant
/// topology/mount/swap-state changes between probes.
///
/// A matching token is necessary evidence for a future destructive workflow,
/// but the current project does not treat it as sufficient authorization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinuxRevalidationToken {
    pub device_number: DeviceNumber,
    pub diskseq: Option<u64>,
    pub size_bytes: u64,
    pub logical_block_size: u64,
    pub removable: bool,
    pub read_only: bool,
    pub persistent_identity: Option<String>,
    pub serial: Option<String>,
    pub topology_device_numbers: Vec<DeviceNumber>,
    pub mounted_topology_device_numbers: Vec<DeviceNumber>,
    pub active_swap_topology_device_numbers: Vec<DeviceNumber>,
}

impl LinuxRevalidationToken {
    #[must_use]
    pub fn matches(&self, device: &LinuxBlockDevice) -> bool {
        self == &device.revalidation_token()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveryWarning {
    pub context: PathBuf,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinuxDiscoveryReport {
    pub devices: Vec<LinuxBlockDevice>,
    pub warnings: Vec<DiscoveryWarning>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinuxDiscoveryError {
    pub context: PathBuf,
    pub message: String,
}

impl LinuxDiscoveryError {
    fn new(context: impl Into<PathBuf>, message: impl Into<String>) -> Self {
        Self {
            context: context.into(),
            message: message.into(),
        }
    }

    fn io(context: &Path, error: &io::Error) -> Self {
        Self::new(context, error.to_string())
    }
}

impl Display for LinuxDiscoveryError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.context.display(), self.message)
    }
}

impl Error for LinuxDiscoveryError {}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MountPoint {
    device: DeviceNumber,
    path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SystemMounts {
    mounted_devices: BTreeSet<DeviceNumber>,
    root_device: DeviceNumber,
    boot_devices: BTreeSet<DeviceNumber>,
    mount_points: Vec<MountPoint>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PartitionDevice {
    path: PathBuf,
    device_number: DeviceNumber,
}

/// Discover whole Linux block devices using read-only sysfs, mountinfo,
/// active-swap metadata, and persistent-device alias metadata.
///
/// The function never opens a block-device node. A per-device metadata or
/// topology failure causes that device to be omitted and reported as a warning.
/// A failure to resolve global active-swap evidence fails the discovery so
/// incomplete safety evidence cannot accidentally become an eligible target.
pub fn discover_linux_block_devices(
    paths: &LinuxProbePaths,
) -> Result<LinuxDiscoveryReport, LinuxDiscoveryError> {
    let mount_text = fs::read_to_string(&paths.mountinfo)
        .map_err(|error| LinuxDiscoveryError::io(&paths.mountinfo, &error))?;
    let mounts = parse_system_mounts(&mount_text)
        .map_err(|message| LinuxDiscoveryError::new(&paths.mountinfo, message))?;

    let swap_text = fs::read_to_string(&paths.proc_swaps)
        .map_err(|error| LinuxDiscoveryError::io(&paths.proc_swaps, &error))?;
    let active_swap_devices = discover_active_swap_devices(paths, &mounts, &swap_text)?;

    let entries = fs::read_dir(&paths.sys_block_root)
        .map_err(|error| LinuxDiscoveryError::io(&paths.sys_block_root, &error))?;

    let mut names = Vec::new();
    let mut warnings = Vec::new();

    for entry in entries {
        match entry {
            Ok(entry) => match entry.file_name().into_string() {
                Ok(name) => names.push(name),
                Err(_) => warnings.push(DiscoveryWarning {
                    context: paths.sys_block_root.clone(),
                    message: "ignored a block-device entry with a non-UTF-8 name".to_owned(),
                }),
            },
            Err(error) => warnings.push(DiscoveryWarning {
                context: paths.sys_block_root.clone(),
                message: format!("failed to read block-device directory entry: {error}"),
            }),
        }
    }

    names.sort();

    let mut devices = Vec::new();
    for name in names {
        let sys_device = paths.sys_block_root.join(&name);

        if sys_device.join("partition").exists() {
            continue;
        }

        match discover_one(
            paths,
            &mounts,
            &active_swap_devices,
            &name,
            &sys_device,
            &mut warnings,
        ) {
            Ok(device) => devices.push(device),
            Err(error) => warnings.push(DiscoveryWarning {
                context: error.context,
                message: error.message,
            }),
        }
    }

    Ok(LinuxDiscoveryReport { devices, warnings })
}

fn discover_one(
    paths: &LinuxProbePaths,
    mounts: &SystemMounts,
    active_swap_devices: &BTreeSet<DeviceNumber>,
    kernel_name: &str,
    sys_device: &Path,
    warnings: &mut Vec<DiscoveryWarning>,
) -> Result<LinuxBlockDevice, LinuxDiscoveryError> {
    let device_number = read_device_number(&sys_device.join("dev"))?;
    let capacity_sectors = read_u64(&sys_device.join("size"))?;
    let size_bytes = capacity_sectors
        .checked_mul(SYSFS_CAPACITY_SECTOR_BYTES)
        .ok_or_else(|| LinuxDiscoveryError::new(sys_device.join("size"), "capacity overflow"))?;
    let logical_block_size = read_u64(&sys_device.join("queue/logical_block_size"))?;
    let physical_block_size = read_u64(&sys_device.join("queue/physical_block_size"))?;
    let removable = read_bool(&sys_device.join("removable"))?;
    let read_only = read_bool(&sys_device.join("ro"))?;
    let partitions = discover_partitions(sys_device)?;
    let partition_device_numbers = partitions
        .iter()
        .map(|partition| partition.device_number)
        .collect();

    let mut topology_roots = Vec::with_capacity(partitions.len() + 1);
    topology_roots.push(sys_device.to_path_buf());
    topology_roots.extend(partitions.iter().map(|partition| partition.path.clone()));
    let topology_device_numbers = collect_topology_device_numbers(&topology_roots)?;
    let topology_set: BTreeSet<DeviceNumber> = topology_device_numbers.iter().copied().collect();
    let mounted_topology_device_numbers = mounts
        .mounted_devices
        .intersection(&topology_set)
        .copied()
        .collect::<Vec<_>>();
    let active_swap_topology_device_numbers = active_swap_devices
        .intersection(&topology_set)
        .copied()
        .collect::<Vec<_>>();

    let contains_mounted_root = topology_set.contains(&mounts.root_device);
    let contains_mounted_boot = mounts
        .boot_devices
        .iter()
        .any(|device| topology_set.contains(device));
    let contains_mounted_filesystem = !mounted_topology_device_numbers.is_empty();
    let contains_active_swap = !active_swap_topology_device_numbers.is_empty();
    let devnode = paths.dev_root.join(kernel_name);
    let persistent_aliases = collect_persistent_aliases(paths, &devnode, warnings);

    Ok(LinuxBlockDevice {
        kernel_name: kernel_name.to_owned(),
        devnode,
        device_number,
        partition_device_numbers,
        topology_device_numbers,
        mounted_topology_device_numbers,
        active_swap_topology_device_numbers,
        size_bytes,
        logical_block_size,
        physical_block_size,
        removable,
        read_only,
        contains_mounted_root,
        contains_mounted_boot,
        contains_mounted_filesystem,
        contains_active_swap,
        diskseq: read_optional_u64(&sys_device.join("diskseq"), warnings),
        vendor: read_optional_text(&sys_device.join("device/vendor"), warnings),
        model: read_optional_text(&sys_device.join("device/model"), warnings),
        serial: read_optional_text(&sys_device.join("device/serial"), warnings),
        wwid: read_optional_text(&sys_device.join("device/wwid"), warnings),
        persistent_aliases,
    })
}

fn discover_partitions(sys_device: &Path) -> Result<Vec<PartitionDevice>, LinuxDiscoveryError> {
    let entries =
        fs::read_dir(sys_device).map_err(|error| LinuxDiscoveryError::io(sys_device, &error))?;
    let mut partitions = Vec::new();

    for entry in entries {
        let entry = entry.map_err(|error| LinuxDiscoveryError::io(sys_device, &error))?;
        let path = entry.path();
        if !path.join("partition").exists() {
            continue;
        }

        partitions.push(PartitionDevice {
            device_number: read_device_number(&path.join("dev"))?,
            path,
        });
    }

    partitions.sort_by_key(|partition| partition.device_number);
    partitions.dedup_by_key(|partition| partition.device_number);
    Ok(partitions)
}

fn collect_topology_device_numbers(
    roots: &[PathBuf],
) -> Result<Vec<DeviceNumber>, LinuxDiscoveryError> {
    let mut queue: VecDeque<PathBuf> = roots.iter().cloned().collect();
    let mut visited_paths = BTreeSet::new();
    let mut device_numbers = BTreeSet::new();

    while let Some(path) = queue.pop_front() {
        let canonical_path =
            fs::canonicalize(&path).map_err(|error| LinuxDiscoveryError::io(&path, &error))?;
        if !visited_paths.insert(canonical_path.clone()) {
            continue;
        }

        device_numbers.insert(read_device_number(&canonical_path.join("dev"))?);

        let holders_path = canonical_path.join("holders");
        let holders = match fs::read_dir(&holders_path) {
            Ok(entries) => entries,
            Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
            Err(error) => return Err(LinuxDiscoveryError::io(&holders_path, &error)),
        };

        for holder in holders {
            let holder = holder.map_err(|error| LinuxDiscoveryError::io(&holders_path, &error))?;
            let holder_path = holder.path();
            let canonical_holder = fs::canonicalize(&holder_path)
                .map_err(|error| LinuxDiscoveryError::io(&holder_path, &error))?;
            queue.push_back(canonical_holder);
        }
    }

    Ok(device_numbers.into_iter().collect())
}

fn discover_active_swap_devices(
    paths: &LinuxProbePaths,
    mounts: &SystemMounts,
    swap_text: &str,
) -> Result<BTreeSet<DeviceNumber>, LinuxDiscoveryError> {
    let mut lines = swap_text.lines();
    let header = lines
        .next()
        .ok_or_else(|| LinuxDiscoveryError::new(&paths.proc_swaps, "active-swap table is empty"))?;
    let header_fields = header.split_whitespace().collect::<Vec<_>>();
    if header_fields.len() < 2 || header_fields[0] != "Filename" || header_fields[1] != "Type" {
        return Err(LinuxDiscoveryError::new(
            &paths.proc_swaps,
            "active-swap table has an unexpected header",
        ));
    }

    let mut devices = BTreeSet::new();
    for (index, line) in lines.enumerate() {
        if line.trim().is_empty() {
            continue;
        }

        let fields = line.split_whitespace().collect::<Vec<_>>();
        if fields.len() < 5 {
            return Err(LinuxDiscoveryError::new(
                &paths.proc_swaps,
                format!(
                    "active-swap line {} is too short to parse safely",
                    index + 2
                ),
            ));
        }

        let swap_path = PathBuf::from(decode_proc_path_field(fields[0]));
        let device = match fields[1] {
            "partition" => resolve_swap_partition_device(paths, &swap_path)?,
            "file" => resolve_swap_file_device(mounts, &swap_path)?,
            swap_type => {
                return Err(LinuxDiscoveryError::new(
                    &paths.proc_swaps,
                    format!(
                        "active-swap line {} has unsupported type {swap_type}",
                        index + 2
                    ),
                ));
            }
        };
        devices.insert(device);
    }

    Ok(devices)
}

fn resolve_swap_partition_device(
    paths: &LinuxProbePaths,
    swap_path: &Path,
) -> Result<DeviceNumber, LinuxDiscoveryError> {
    let canonical =
        fs::canonicalize(swap_path).map_err(|error| LinuxDiscoveryError::io(swap_path, &error))?;
    let kernel_name = canonical
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            LinuxDiscoveryError::new(
                swap_path,
                "active swap block-device path has no UTF-8 kernel name",
            )
        })?;

    if let Some(device) =
        try_read_device_number(&paths.sys_block_root.join(kernel_name).join("dev"))?
    {
        return Ok(device);
    }

    let entries = fs::read_dir(&paths.sys_block_root)
        .map_err(|error| LinuxDiscoveryError::io(&paths.sys_block_root, &error))?;
    for entry in entries {
        let entry =
            entry.map_err(|error| LinuxDiscoveryError::io(&paths.sys_block_root, &error))?;
        if let Some(device) = try_read_device_number(&entry.path().join(kernel_name).join("dev"))? {
            return Ok(device);
        }
    }

    Err(LinuxDiscoveryError::new(
        swap_path,
        format!("could not resolve active swap block device {kernel_name} in sysfs"),
    ))
}

fn resolve_swap_file_device(
    mounts: &SystemMounts,
    swap_path: &Path,
) -> Result<DeviceNumber, LinuxDiscoveryError> {
    let canonical =
        fs::canonicalize(swap_path).map_err(|error| LinuxDiscoveryError::io(swap_path, &error))?;
    mounts
        .mount_points
        .iter()
        .filter(|mount| canonical.starts_with(&mount.path))
        .max_by_key(|mount| mount.path.components().count())
        .map(|mount| mount.device)
        .ok_or_else(|| {
            LinuxDiscoveryError::new(
                swap_path,
                "could not associate active swap file with a mounted filesystem",
            )
        })
}

fn collect_persistent_aliases(
    paths: &LinuxProbePaths,
    devnode: &Path,
    warnings: &mut Vec<DiscoveryWarning>,
) -> Vec<PathBuf> {
    let canonical_device = match fs::canonicalize(devnode) {
        Ok(path) => path,
        Err(error) => {
            if error.kind() != io::ErrorKind::NotFound {
                warnings.push(DiscoveryWarning {
                    context: devnode.to_path_buf(),
                    message: format!("could not canonicalize device node: {error}"),
                });
            }
            return Vec::new();
        }
    };

    let entries = match fs::read_dir(&paths.dev_disk_by_id) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Vec::new(),
        Err(error) => {
            warnings.push(DiscoveryWarning {
                context: paths.dev_disk_by_id.clone(),
                message: format!("could not enumerate persistent aliases: {error}"),
            });
            return Vec::new();
        }
    };

    let mut aliases = Vec::new();
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                warnings.push(DiscoveryWarning {
                    context: paths.dev_disk_by_id.clone(),
                    message: format!("could not read persistent-alias entry: {error}"),
                });
                continue;
            }
        };

        let alias = entry.path();
        match fs::canonicalize(&alias) {
            Ok(target) if target == canonical_device => aliases.push(alias),
            Ok(_) => {}
            Err(error) => warnings.push(DiscoveryWarning {
                context: alias,
                message: format!("could not resolve persistent alias: {error}"),
            }),
        }
    }

    aliases.sort();
    aliases
}

fn read_device_number(path: &Path) -> Result<DeviceNumber, LinuxDiscoveryError> {
    let value = read_text(path)?;
    DeviceNumber::parse(&value).map_err(|error| LinuxDiscoveryError::new(path, error.to_string()))
}

fn try_read_device_number(path: &Path) -> Result<Option<DeviceNumber>, LinuxDiscoveryError> {
    match fs::read_to_string(path) {
        Ok(value) => DeviceNumber::parse(&value)
            .map(Some)
            .map_err(|error| LinuxDiscoveryError::new(path, error.to_string())),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(LinuxDiscoveryError::io(path, &error)),
    }
}

fn read_u64(path: &Path) -> Result<u64, LinuxDiscoveryError> {
    let value = read_text(path)?;
    value
        .parse::<u64>()
        .map_err(|_| LinuxDiscoveryError::new(path, "expected an unsigned integer"))
}

fn read_bool(path: &Path) -> Result<bool, LinuxDiscoveryError> {
    match read_text(path)?.as_str() {
        "0" => Ok(false),
        "1" => Ok(true),
        _ => Err(LinuxDiscoveryError::new(path, "expected 0 or 1")),
    }
}

fn read_text(path: &Path) -> Result<String, LinuxDiscoveryError> {
    fs::read_to_string(path)
        .map(|value| value.trim().to_owned())
        .map_err(|error| LinuxDiscoveryError::io(path, &error))
}

fn read_optional_text(path: &Path, warnings: &mut Vec<DiscoveryWarning>) -> Option<String> {
    match fs::read_to_string(path) {
        Ok(value) => {
            let value = value.trim();
            (!value.is_empty()).then(|| value.to_owned())
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => {
            warnings.push(DiscoveryWarning {
                context: path.to_path_buf(),
                message: format!("could not read optional identity metadata: {error}"),
            });
            None
        }
    }
}

fn read_optional_u64(path: &Path, warnings: &mut Vec<DiscoveryWarning>) -> Option<u64> {
    let value = read_optional_text(path, warnings)?;
    match value.parse::<u64>() {
        Ok(value) => Some(value),
        Err(_) => {
            warnings.push(DiscoveryWarning {
                context: path.to_path_buf(),
                message: "ignored invalid optional unsigned integer".to_owned(),
            });
            None
        }
    }
}

fn parse_system_mounts(mountinfo: &str) -> Result<SystemMounts, String> {
    let mut mounted_devices = BTreeSet::new();
    let mut root_device = None;
    let mut boot_devices = BTreeSet::new();
    let mut mount_points = Vec::new();

    for (line_number, line) in mountinfo.lines().enumerate() {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() < 6 {
            return Err(format!(
                "mountinfo line {} is too short to parse safely",
                line_number + 1
            ));
        }

        let device = DeviceNumber::parse(fields[2]).map_err(|error| {
            format!(
                "mountinfo line {} has an invalid device number: {error}",
                line_number + 1
            )
        })?;
        let mount_point = PathBuf::from(decode_proc_path_field(fields[4]));
        mounted_devices.insert(device);
        mount_points.push(MountPoint {
            device,
            path: mount_point.clone(),
        });

        if mount_point == Path::new("/") {
            root_device = Some(device);
        }
        if mount_point == Path::new("/boot") || mount_point.starts_with("/boot/") {
            boot_devices.insert(device);
        }
    }

    Ok(SystemMounts {
        mounted_devices,
        root_device: root_device
            .ok_or_else(|| "mountinfo does not identify the mounted root filesystem".to_owned())?,
        boot_devices,
        mount_points,
    })
}

fn decode_proc_path_field(value: &str) -> String {
    value
        .replace("\\040", " ")
        .replace("\\011", "\t")
        .replace("\\012", "\n")
        .replace("\\134", "\\")
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::symlink;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;
    use crate::layout::GIB;

    const SWAP_HEADER: &str = "Filename\t\t\tType\t\tSize\t\tUsed\t\tPriority\n";

    struct Fixture {
        root: PathBuf,
    }

    impl Fixture {
        fn new() -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock must be after Unix epoch")
                .as_nanos();
            let root = std::env::temp_dir()
                .join(format!("gcboot-linux-test-{}-{nonce}", std::process::id()));
            fs::create_dir_all(&root).expect("fixture root must be creatable");
            Self { root }
        }

        fn paths(&self) -> LinuxProbePaths {
            LinuxProbePaths {
                sys_block_root: self.root.join("sys/block"),
                dev_root: self.root.join("dev"),
                dev_disk_by_id: self.root.join("dev/disk/by-id"),
                mountinfo: self.root.join("proc/self/mountinfo"),
                proc_swaps: self.root.join("proc/swaps"),
            }
        }

        fn write(&self, relative: &str, value: &str) {
            let path = self.root.join(relative);
            fs::create_dir_all(path.parent().expect("fixture path must have a parent"))
                .expect("fixture directory must be creatable");
            fs::write(path, value).expect("fixture file must be writable");
        }

        fn symlink(&self, target: &str, relative: &str) {
            let path = self.root.join(relative);
            fs::create_dir_all(path.parent().expect("fixture path must have a parent"))
                .expect("fixture directory must be creatable");
            symlink(target, path).expect("fixture symlink must be creatable");
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    fn create_test_disk(fixture: &Fixture) {
        let sectors = (16 * GIB) / SYSFS_CAPACITY_SECTOR_BYTES;
        fixture.write("sys/block/sdz/dev", "8:0\n");
        fixture.write("sys/block/sdz/size", &format!("{sectors}\n"));
        fixture.write("sys/block/sdz/removable", "1\n");
        fixture.write("sys/block/sdz/ro", "0\n");
        fixture.write("sys/block/sdz/diskseq", "42\n");
        fixture.write("sys/block/sdz/queue/logical_block_size", "512\n");
        fixture.write("sys/block/sdz/queue/physical_block_size", "4096\n");
        fixture.write("sys/block/sdz/device/vendor", "GoreeCloud\n");
        fixture.write("sys/block/sdz/device/model", "Test USB\n");
        fixture.write("sys/block/sdz/device/serial", "GCBOOT-TEST-1\n");
        fixture.write("sys/block/sdz/device/wwid", "test-wwid-1\n");
        fixture.write("sys/block/sdz/sdz1/partition", "1\n");
        fixture.write("sys/block/sdz/sdz1/dev", "8:1\n");
        fixture.write("dev/sdz", "fixture device node placeholder\n");
        fixture.write("dev/sdz1", "fixture partition node placeholder\n");
        fixture.write("proc/swaps", SWAP_HEADER);

        let by_id = fixture.root.join("dev/disk/by-id");
        fs::create_dir_all(&by_id).expect("by-id directory must be creatable");
        symlink(
            "../../sdz",
            by_id.join("usb-GoreeCloud_Test_USB_GCBOOT-TEST-1"),
        )
        .expect("fixture symlink must be creatable");
    }

    fn write_root_mount(fixture: &Fixture) {
        fixture.write(
            "proc/self/mountinfo",
            "36 25 259:2 / / rw,relatime - ext4 /dev/nvme0n1p2 rw\n",
        );
    }

    fn write_swap_partition(fixture: &Fixture, path: &Path) {
        fixture.write(
            "proc/swaps",
            &format!(
                "{SWAP_HEADER}{}\tpartition\t1048572\t0\t-2\n",
                path.display()
            ),
        );
    }

    fn test_disk(report: &LinuxDiscoveryReport) -> &LinuxBlockDevice {
        report
            .devices
            .iter()
            .find(|device| device.kernel_name == "sdz")
            .expect("test disk must be discovered")
    }

    #[test]
    fn parses_device_numbers() {
        assert_eq!(
            DeviceNumber::parse("259:7"),
            Ok(DeviceNumber {
                major: 259,
                minor: 7
            })
        );
        assert!(DeviceNumber::parse("not-a-device").is_err());
    }

    #[test]
    fn discovers_read_only_linux_metadata_and_persistent_identity() {
        let fixture = Fixture::new();
        create_test_disk(&fixture);
        write_root_mount(&fixture);

        let report = discover_linux_block_devices(&fixture.paths()).expect("fixture must discover");
        assert!(
            report.warnings.is_empty(),
            "warnings: {:?}",
            report.warnings
        );
        assert_eq!(report.devices.len(), 1);

        let device = test_disk(&report);
        assert_eq!(device.kernel_name, "sdz");
        assert_eq!(device.size_bytes, 16 * GIB);
        assert_eq!(device.logical_block_size, 512);
        assert_eq!(device.physical_block_size, 4096);
        assert!(device.removable);
        assert!(!device.read_only);
        assert!(!device.contains_mounted_root);
        assert!(!device.contains_mounted_boot);
        assert!(!device.contains_mounted_filesystem);
        assert!(!device.contains_active_swap);
        assert_eq!(device.diskseq, Some(42));
        assert_eq!(device.wwid.as_deref(), Some("test-wwid-1"));
        assert_eq!(device.persistent_aliases.len(), 1);
        assert_eq!(
            device.persistent_identity().as_deref(),
            Some("wwid:test-wwid-1")
        );
        assert_eq!(
            device.topology_device_numbers,
            vec![
                DeviceNumber { major: 8, minor: 0 },
                DeviceNumber { major: 8, minor: 1 }
            ]
        );
        assert!(device.mounted_topology_device_numbers.is_empty());
        assert!(device.active_swap_topology_device_numbers.is_empty());
        assert!(device.assessment().eligible);

        let token = device.revalidation_token();
        assert!(token.matches(device));
    }

    #[test]
    fn rejects_disk_when_root_is_mounted_from_its_partition() {
        let fixture = Fixture::new();
        create_test_disk(&fixture);
        fixture.write(
            "proc/self/mountinfo",
            "36 25 8:1 / / rw,relatime - ext4 /dev/sdz1 rw\n",
        );

        let report = discover_linux_block_devices(&fixture.paths()).expect("fixture must discover");
        let device = test_disk(&report);
        assert!(device.contains_mounted_root);
        assert!(device.contains_mounted_filesystem);
        assert!(!device.assessment().eligible);
    }

    #[test]
    fn rejects_disk_when_non_boot_filesystem_is_mounted_from_its_partition() {
        let fixture = Fixture::new();
        create_test_disk(&fixture);
        fixture.write(
            "proc/self/mountinfo",
            "36 25 259:2 / / rw,relatime - ext4 /dev/nvme0n1p2 rw\n\
37 25 8:1 / /media/test rw,relatime - ext4 /dev/sdz1 rw\n",
        );

        let report = discover_linux_block_devices(&fixture.paths()).expect("fixture must discover");
        let device = test_disk(&report);
        assert!(!device.contains_mounted_root);
        assert!(!device.contains_mounted_boot);
        assert!(device.contains_mounted_filesystem);
        assert_eq!(
            device.mounted_topology_device_numbers,
            vec![DeviceNumber { major: 8, minor: 1 }]
        );
        assert!(!device.assessment().eligible);
    }

    #[test]
    fn rejects_disk_when_recursive_holder_topology_is_mounted() {
        let fixture = Fixture::new();
        create_test_disk(&fixture);
        fixture.write("sys/block/dm-0/dev", "253:0\n");
        fixture.write("sys/block/md0/dev", "9:0\n");
        fixture.symlink("../../../dm-0", "sys/block/sdz/sdz1/holders/dm-0");
        fixture.symlink("../../md0", "sys/block/dm-0/holders/md0");
        fixture.write(
            "proc/self/mountinfo",
            "36 25 259:2 / / rw,relatime - ext4 /dev/nvme0n1p2 rw\n\
37 25 9:0 / /srv/storage rw,relatime - ext4 /dev/md0 rw\n",
        );

        let report = discover_linux_block_devices(&fixture.paths()).expect("fixture must discover");
        let device = test_disk(&report);
        assert_eq!(
            device.topology_device_numbers,
            vec![
                DeviceNumber { major: 8, minor: 0 },
                DeviceNumber { major: 8, minor: 1 },
                DeviceNumber { major: 9, minor: 0 },
                DeviceNumber {
                    major: 253,
                    minor: 0
                }
            ]
        );
        assert_eq!(
            device.mounted_topology_device_numbers,
            vec![DeviceNumber { major: 9, minor: 0 }]
        );
        assert!(device.contains_mounted_filesystem);
        assert!(!device.assessment().eligible);
    }

    #[test]
    fn rejects_disk_when_partition_is_active_swap() {
        let fixture = Fixture::new();
        create_test_disk(&fixture);
        write_root_mount(&fixture);
        write_swap_partition(&fixture, &fixture.root.join("dev/sdz1"));

        let report = discover_linux_block_devices(&fixture.paths()).expect("fixture must discover");
        let device = test_disk(&report);
        assert!(device.contains_active_swap);
        assert_eq!(
            device.active_swap_topology_device_numbers,
            vec![DeviceNumber { major: 8, minor: 1 }]
        );
        assert!(!device.assessment().eligible);
    }

    #[test]
    fn rejects_disk_when_holder_is_active_swap() {
        let fixture = Fixture::new();
        create_test_disk(&fixture);
        write_root_mount(&fixture);
        fixture.write("sys/block/dm-0/dev", "253:0\n");
        fixture.write("dev/dm-0", "fixture mapper node placeholder\n");
        fixture.symlink("../../../dm-0", "sys/block/sdz/sdz1/holders/dm-0");
        write_swap_partition(&fixture, &fixture.root.join("dev/dm-0"));

        let report = discover_linux_block_devices(&fixture.paths()).expect("fixture must discover");
        let device = test_disk(&report);
        assert!(device.contains_active_swap);
        assert_eq!(
            device.active_swap_topology_device_numbers,
            vec![DeviceNumber {
                major: 253,
                minor: 0
            }]
        );
        assert!(!device.assessment().eligible);
    }

    #[test]
    fn associates_active_swap_file_with_its_mounted_filesystem() {
        let fixture = Fixture::new();
        create_test_disk(&fixture);
        let mount_point = fixture.root.join("mnt/test");
        let swap_file = mount_point.join("swapfile");
        fixture.write("mnt/test/swapfile", "fixture swap file\n");
        fixture.write(
            "proc/self/mountinfo",
            &format!(
                "36 25 259:2 / / rw,relatime - ext4 /dev/nvme0n1p2 rw\n\
37 25 8:1 / {} rw,relatime - ext4 /dev/sdz1 rw\n",
                mount_point.display()
            ),
        );
        fixture.write(
            "proc/swaps",
            &format!(
                "{SWAP_HEADER}{}\tfile\t1048572\t0\t-2\n",
                swap_file.display()
            ),
        );

        let report = discover_linux_block_devices(&fixture.paths()).expect("fixture must discover");
        let device = test_disk(&report);
        assert!(device.contains_mounted_filesystem);
        assert!(device.contains_active_swap);
        assert_eq!(
            device.active_swap_topology_device_numbers,
            vec![DeviceNumber { major: 8, minor: 1 }]
        );
        assert!(!device.assessment().eligible);
    }

    #[test]
    fn unresolved_active_swap_fails_discovery_closed() {
        let fixture = Fixture::new();
        create_test_disk(&fixture);
        write_root_mount(&fixture);
        write_swap_partition(&fixture, &fixture.root.join("dev/missing-swap"));

        let error = discover_linux_block_devices(&fixture.paths())
            .expect_err("unresolved active swap must fail discovery");
        assert!(error.message.contains("No such file") || error.message.contains("not found"));
    }

    #[test]
    fn revalidation_token_changes_when_mount_state_changes() {
        let fixture = Fixture::new();
        create_test_disk(&fixture);
        write_root_mount(&fixture);

        let first_report = discover_linux_block_devices(&fixture.paths())
            .expect("first fixture probe must discover");
        let token = test_disk(&first_report).revalidation_token();

        fixture.write(
            "proc/self/mountinfo",
            "36 25 259:2 / / rw,relatime - ext4 /dev/nvme0n1p2 rw\n\
37 25 8:1 / /media/test rw,relatime - ext4 /dev/sdz1 rw\n",
        );
        let second_report = discover_linux_block_devices(&fixture.paths())
            .expect("second fixture probe must discover");
        let device = test_disk(&second_report);

        assert!(!token.matches(device));
        assert!(device.contains_mounted_filesystem);
    }

    #[test]
    fn revalidation_token_changes_when_swap_state_changes() {
        let fixture = Fixture::new();
        create_test_disk(&fixture);
        write_root_mount(&fixture);

        let first_report = discover_linux_block_devices(&fixture.paths())
            .expect("first fixture probe must discover");
        let token = test_disk(&first_report).revalidation_token();

        write_swap_partition(&fixture, &fixture.root.join("dev/sdz1"));
        let second_report = discover_linux_block_devices(&fixture.paths())
            .expect("second fixture probe must discover");
        let device = test_disk(&second_report);

        assert!(!token.matches(device));
        assert!(device.contains_active_swap);
    }

    #[test]
    fn decodes_proc_path_escape_sequences() {
        assert_eq!(
            decode_proc_path_field("/media/GoreeCloud\\040Boot"),
            "/media/GoreeCloud Boot"
        );
    }
}
