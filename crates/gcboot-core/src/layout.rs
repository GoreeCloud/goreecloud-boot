// SPDX-License-Identifier: GPL-3.0-or-later

use std::error::Error;
use std::fmt::{Display, Formatter};

pub const MIB: u64 = 1_048_576;
pub const GIB: u64 = 1_073_741_824;
pub const ALIGNMENT_BYTES: u64 = MIB;
pub const GCBOOT_SIZE_BYTES: u64 = 512 * MIB;
pub const MIN_DEVICE_BYTES: u64 = 8 * GIB;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutError {
    DeviceTooSmall,
    ArithmeticOverflow,
    NoUsableDataSpace,
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
///
/// This function deliberately stays above sector/GPT details. A future
/// write-capable layer must add sector-aware validation and re-check every
/// invariant before authorizing writes.
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
}
