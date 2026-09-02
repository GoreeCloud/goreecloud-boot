// SPDX-License-Identifier: GPL-3.0-or-later

use std::error::Error;
use std::fmt::{Display, Formatter};

pub const MIB: u64 = 1_048_576;
pub const GIB: u64 = 1_073_741_824;
pub const ALIGNMENT_BYTES: u64 = MIB;
pub const GCBOOT_SIZE_BYTES: u64 = 512 * MIB;
pub const MIN_DEVICE_BYTES: u64 = 8 * GIB;
pub const MIN_LOGICAL_BLOCK_SIZE: u64 = 512;
const END_RESERVE_BYTES: u64 = MIB;

/// A planned partition byte range.
///
/// Ranges are half-open: `start_bytes` is inclusive and `end_bytes` is
/// exclusive. The current implementation is planning-only and does not write a
/// partition table or filesystem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartitionPlan {
    pub label: &'static str,
    pub intended_filesystem: &'static str,
    pub start_bytes: u64,
    pub end_bytes: u64,
}

impl PartitionPlan {
    #[must_use]
    pub fn size_bytes(&self) -> u64 {
        self.end_bytes - self.start_bytes
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceLayout {
    pub total_bytes: u64,
    pub gcboot: PartitionPlan,
    pub gcdata: PartitionPlan,
}

/// Inclusive logical-block range for a planned partition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SectorPartitionPlan {
    pub label: &'static str,
    pub intended_filesystem: &'static str,
    pub first_lba: u64,
    pub last_lba: u64,
}

impl SectorPartitionPlan {
    #[must_use]
    pub fn block_count(&self) -> u64 {
        self.last_lba - self.first_lba + 1
    }
}

/// Sector-aware representation of the planned GoreeCloud Boot layout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SectorDeviceLayout {
    pub total_bytes: u64,
    pub logical_block_size: u64,
    pub total_lbas: u64,
    pub gcboot: SectorPartitionPlan,
    pub gcdata: SectorPartitionPlan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutError {
    DeviceTooSmall,
    ArithmeticOverflow,
    NoUsableDataSpace,
    UnsupportedLogicalBlockSize,
    DeviceSizeNotBlockAligned,
    PartitionBoundaryNotBlockAligned,
}

impl Display for LayoutError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DeviceTooSmall => write!(
                formatter,
                "device is smaller than the minimum planning size"
            ),
            Self::ArithmeticOverflow => write!(formatter, "layout arithmetic overflowed"),
            Self::NoUsableDataSpace => {
                write!(formatter, "no usable GCDATA space remains after reserves")
            }
            Self::UnsupportedLogicalBlockSize => write!(
                formatter,
                "logical block size must be a power of two between 512 bytes and 1 MiB and divide the 1 MiB alignment"
            ),
            Self::DeviceSizeNotBlockAligned => {
                write!(
                    formatter,
                    "device size is not aligned to its logical block size"
                )
            }
            Self::PartitionBoundaryNotBlockAligned => write!(
                formatter,
                "a planned partition boundary is not aligned to the logical block size"
            ),
        }
    }
}

impl Error for LayoutError {}

fn align_up(value: u64, alignment: u64) -> Result<u64, LayoutError> {
    let remainder = value % alignment;
    if remainder == 0 {
        return Ok(value);
    }

    value
        .checked_add(alignment - remainder)
        .ok_or(LayoutError::ArithmeticOverflow)
}

/// Plan the initial GoreeCloud Boot byte layout without modifying a device.
pub fn plan_layout(total_bytes: u64) -> Result<DeviceLayout, LayoutError> {
    if total_bytes < MIN_DEVICE_BYTES {
        return Err(LayoutError::DeviceTooSmall);
    }

    let gcboot_start = ALIGNMENT_BYTES;
    let gcboot_end = gcboot_start
        .checked_add(GCBOOT_SIZE_BYTES)
        .ok_or(LayoutError::ArithmeticOverflow)?;
    let gcdata_start = align_up(gcboot_end, ALIGNMENT_BYTES)?;
    let gcdata_end = total_bytes
        .checked_sub(END_RESERVE_BYTES)
        .ok_or(LayoutError::ArithmeticOverflow)?;

    if gcdata_end <= gcdata_start {
        return Err(LayoutError::NoUsableDataSpace);
    }

    Ok(DeviceLayout {
        total_bytes,
        gcboot: PartitionPlan {
            label: "GCBOOT",
            intended_filesystem: "FAT32",
            start_bytes: gcboot_start,
            end_bytes: gcboot_end,
        },
        gcdata: PartitionPlan {
            label: "GCDATA",
            intended_filesystem: "exFAT",
            start_bytes: gcdata_start,
            end_bytes: gcdata_end,
        },
    })
}

/// Convert the byte planner into checked logical-block ranges.
///
/// This is still planning-only. It does not create GPT structures or write to
/// a block device.
pub fn plan_sector_layout(
    total_bytes: u64,
    logical_block_size: u64,
) -> Result<SectorDeviceLayout, LayoutError> {
    if logical_block_size < MIN_LOGICAL_BLOCK_SIZE
        || logical_block_size > ALIGNMENT_BYTES
        || !logical_block_size.is_power_of_two()
        || ALIGNMENT_BYTES % logical_block_size != 0
    {
        return Err(LayoutError::UnsupportedLogicalBlockSize);
    }

    if total_bytes % logical_block_size != 0 {
        return Err(LayoutError::DeviceSizeNotBlockAligned);
    }

    let layout = plan_layout(total_bytes)?;
    let gcboot = sector_partition(&layout.gcboot, logical_block_size)?;
    let gcdata = sector_partition(&layout.gcdata, logical_block_size)?;
    let total_lbas = total_bytes / logical_block_size;

    Ok(SectorDeviceLayout {
        total_bytes,
        logical_block_size,
        total_lbas,
        gcboot,
        gcdata,
    })
}

fn sector_partition(
    partition: &PartitionPlan,
    logical_block_size: u64,
) -> Result<SectorPartitionPlan, LayoutError> {
    if partition.start_bytes % logical_block_size != 0
        || partition.end_bytes % logical_block_size != 0
    {
        return Err(LayoutError::PartitionBoundaryNotBlockAligned);
    }

    let end_exclusive = partition.end_bytes / logical_block_size;
    let last_lba = end_exclusive
        .checked_sub(1)
        .ok_or(LayoutError::ArithmeticOverflow)?;

    Ok(SectorPartitionPlan {
        label: partition.label,
        intended_filesystem: partition.intended_filesystem,
        first_lba: partition.start_bytes / logical_block_size,
        last_lba,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_devices_below_minimum() {
        assert_eq!(
            plan_layout(MIN_DEVICE_BYTES - 1),
            Err(LayoutError::DeviceTooSmall)
        );
    }

    #[test]
    fn plans_expected_partition_boundaries() {
        let total = 64 * GIB;
        let layout = plan_layout(total).expect("64 GiB must be large enough for planning");

        assert_eq!(layout.gcboot.start_bytes, MIB);
        assert_eq!(layout.gcboot.size_bytes(), GCBOOT_SIZE_BYTES);
        assert_eq!(layout.gcboot.end_bytes, layout.gcdata.start_bytes);
        assert_eq!(layout.gcdata.end_bytes, total - MIB);
        assert!(layout.gcdata.end_bytes > layout.gcdata.start_bytes);
    }

    #[test]
    fn partition_ranges_do_not_overlap() {
        let layout = plan_layout(MIN_DEVICE_BYTES).expect("minimum size must produce a layout");
        assert!(layout.gcboot.end_bytes <= layout.gcdata.start_bytes);
        assert!(layout.gcdata.end_bytes <= layout.total_bytes);
    }

    #[test]
    fn creates_sector_layout_for_512_byte_blocks() {
        let layout = plan_sector_layout(64 * GIB, 512).expect("512-byte sectors must be supported");

        assert_eq!(layout.total_lbas, (64 * GIB) / 512);
        assert_eq!(layout.gcboot.first_lba, MIB / 512);
        assert_eq!(layout.gcboot.block_count(), GCBOOT_SIZE_BYTES / 512);
        assert_eq!(layout.gcboot.last_lba + 1, layout.gcdata.first_lba);
        assert_eq!(layout.gcdata.last_lba + 1, (64 * GIB - MIB) / 512);
    }

    #[test]
    fn creates_sector_layout_for_4096_byte_blocks() {
        let layout = plan_sector_layout(MIN_DEVICE_BYTES, 4096)
            .expect("4096-byte sectors must be supported");

        assert_eq!(layout.gcboot.first_lba, MIB / 4096);
        assert_eq!(layout.gcboot.block_count(), GCBOOT_SIZE_BYTES / 4096);
        assert!(layout.gcdata.last_lba < layout.total_lbas);
    }

    #[test]
    fn rejects_invalid_logical_block_size() {
        assert_eq!(
            plan_sector_layout(MIN_DEVICE_BYTES, 1000),
            Err(LayoutError::UnsupportedLogicalBlockSize)
        );
    }

    #[test]
    fn rejects_unaligned_device_capacity() {
        assert_eq!(
            plan_sector_layout(MIN_DEVICE_BYTES + 1, 4096),
            Err(LayoutError::DeviceSizeNotBlockAligned)
        );
    }
}
