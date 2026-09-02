// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::BTreeSet;
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
}

impl Default for LinuxProbePaths {
    fn default() -> Self {
        Self {
            sys_block_root: PathBuf::from("/sys/block"),
            dev_root: PathBuf::from("/dev"),
            dev_disk_by_id: PathBuf::from("/dev/disk/by-id"),
            mountinfo: PathBuf::from("/proc/self/mountinfo"),
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
    pub size_bytes: u64,
    pub logical_block_size: u64,
    pub physical_block_size: u64,
    pub removable: bool,
    pub read_only: bool,
    pub contains_mounted_root: bool,
    pub contains_mounted_boot: bool,
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
            persistent_identity: self.persistent_identity(),
            serial: self.serial.clone(),
        }
    }
}

/// A snapshot used to detect path reuse or device replacement between probes.
///
/// A matching token is necessary evidence for a future destructive workflow,
/// but the current project does not treat it as sufficient authorization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinuxRevalidationToken {
    pub device_number: DeviceNumber,
    pub diskseq: Option<u64>,
    pub size_bytes: u64,
    pub logical_block_size: u64,
    pub persistent_identity: Option<String>,
    pub serial: Option<String>,
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
struct SystemMounts {
    root_device: DeviceNumber,
    boot_devices: BTreeSet<DeviceNumber>,
}

/// Discover whole Linux block devices using read-only sysfs, mountinfo, and
/// persistent-device alias metadata.
///
/// The function never opens a block-device node. A per-device metadata failure
/// causes that device to be omitted and reported as a warning so incomplete
/// evidence cannot accidentally become an eligible target.
pub fn discover_linux_block_devices(
    paths: &LinuxProbePaths,
) -> Result<LinuxDiscoveryReport, LinuxDiscoveryError> {
    let mount_text = fs::read_to_string(&paths.mountinfo)
        .map_err(|error| LinuxDiscoveryError::io(&paths.mountinfo, &error))?;
    let mounts = parse_system_mounts(&mount_text)
        .map_err(|message| LinuxDiscoveryError::new(&paths.mountinfo, message))?;

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

        match discover_one(paths, &mounts, &name, &sys_device, &mut warnings) {
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
    let partition_device_numbers = discover_partition_device_numbers(sys_device)?;

    let mut family = BTreeSet::new();
    family.insert(device_number);
    family.extend(partition_device_numbers.iter().copied());

    let contains_mounted_root = family.contains(&mounts.root_device);
    let contains_mounted_boot = mounts
        .boot_devices
        .iter()
        .any(|device| family.contains(device));
    let devnode = paths.dev_root.join(kernel_name);
    let persistent_aliases = collect_persistent_aliases(paths, &devnode, warnings);

    Ok(LinuxBlockDevice {
        kernel_name: kernel_name.to_owned(),
        devnode,
        device_number,
        partition_device_numbers,
        size_bytes,
        logical_block_size,
        physical_block_size,
        removable,
        read_only,
        contains_mounted_root,
        contains_mounted_boot,
        diskseq: read_optional_u64(&sys_device.join("diskseq"), warnings),
        vendor: read_optional_text(&sys_device.join("device/vendor"), warnings),
        model: read_optional_text(&sys_device.join("device/model"), warnings),
        serial: read_optional_text(&sys_device.join("device/serial"), warnings),
        wwid: read_optional_text(&sys_device.join("device/wwid"), warnings),
        persistent_aliases,
    })
}

fn discover_partition_device_numbers(
    sys_device: &Path,
) -> Result<Vec<DeviceNumber>, LinuxDiscoveryError> {
    let entries =
        fs::read_dir(sys_device).map_err(|error| LinuxDiscoveryError::io(sys_device, &error))?;
    let mut partitions = Vec::new();

    for entry in entries {
        let entry = entry.map_err(|error| LinuxDiscoveryError::io(sys_device, &error))?;
        let path = entry.path();
        if !path.join("partition").exists() {
            continue;
        }

        partitions.push(read_device_number(&path.join("dev"))?);
    }

    partitions.sort();
    partitions.dedup();
    Ok(partitions)
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
    let mut root_device = None;
    let mut boot_devices = BTreeSet::new();

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
        let mount_point = decode_mountinfo_field(fields[4]);

        if mount_point == "/" {
            root_device = Some(device);
        }
        if mount_point == "/boot" || mount_point.starts_with("/boot/") {
            boot_devices.insert(device);
        }
    }

    Ok(SystemMounts {
        root_device: root_device
            .ok_or_else(|| "mountinfo does not identify the mounted root filesystem".to_owned())?,
        boot_devices,
    })
}

fn decode_mountinfo_field(value: &str) -> String {
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
            }
        }

        fn write(&self, relative: &str, value: &str) {
            let path = self.root.join(relative);
            fs::create_dir_all(path.parent().expect("fixture path must have a parent"))
                .expect("fixture directory must be creatable");
            fs::write(path, value).expect("fixture file must be writable");
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

        let by_id = fixture.root.join("dev/disk/by-id");
        fs::create_dir_all(&by_id).expect("by-id directory must be creatable");
        symlink(
            "../../sdz",
            by_id.join("usb-GoreeCloud_Test_USB_GCBOOT-TEST-1"),
        )
        .expect("fixture symlink must be creatable");
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
        fixture.write(
            "proc/self/mountinfo",
            "36 25 259:2 / / rw,relatime - ext4 /dev/nvme0n1p2 rw\n",
        );

        let report = discover_linux_block_devices(&fixture.paths()).expect("fixture must discover");
        assert!(
            report.warnings.is_empty(),
            "warnings: {:?}",
            report.warnings
        );
        assert_eq!(report.devices.len(), 1);

        let device = &report.devices[0];
        assert_eq!(device.kernel_name, "sdz");
        assert_eq!(device.size_bytes, 16 * GIB);
        assert_eq!(device.logical_block_size, 512);
        assert_eq!(device.physical_block_size, 4096);
        assert!(device.removable);
        assert!(!device.read_only);
        assert!(!device.contains_mounted_root);
        assert!(!device.contains_mounted_boot);
        assert_eq!(device.diskseq, Some(42));
        assert_eq!(device.wwid.as_deref(), Some("test-wwid-1"));
        assert_eq!(device.persistent_aliases.len(), 1);
        assert_eq!(
            device.persistent_identity().as_deref(),
            Some("wwid:test-wwid-1")
        );
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
        let device = &report.devices[0];
        assert!(device.contains_mounted_root);
        assert!(!device.assessment().eligible);
    }

    #[test]
    fn decodes_mountinfo_escape_sequences() {
        assert_eq!(
            decode_mountinfo_field("/media/GoreeCloud\\040Boot"),
            "/media/GoreeCloud Boot"
        );
    }
}
