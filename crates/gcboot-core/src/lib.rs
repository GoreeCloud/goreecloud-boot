// SPDX-License-Identifier: GPL-3.0-or-later
//
// GoreeCloud Boot core primitives. The current crate is intentionally limited
// to non-destructive planning, read-only Linux discovery, validation, and GPT
// metadata generation for development test images. It contains no block-device
// write path.

#![forbid(unsafe_code)]

pub mod catalog;
pub mod device;
pub mod gpt;
pub mod layout;
#[cfg(target_os = "linux")]
pub mod linux;

pub use catalog::{Architecture, BootKind, CatalogEntry, CatalogError};
pub use device::{DeviceEvidence, TargetAssessment};
pub use gpt::{
    BASIC_DATA_PARTITION_TYPE, EFI_SYSTEM_PARTITION_TYPE, GPT_HEADER_SIZE,
    GPT_PARTITION_ENTRY_COUNT, GPT_PARTITION_ENTRY_SIZE, GptError, GptGuid, GptIdentity,
    GptMetadataImage, GptMetadataWrite, build_gpt_metadata, development_test_identity,
};
pub use layout::{
    DeviceLayout, LayoutError, PartitionPlan, SectorDeviceLayout, SectorPartitionPlan, plan_layout,
    plan_sector_layout,
};
#[cfg(target_os = "linux")]
pub use linux::{
    DeviceNumber, DeviceNumberParseError, DiscoveryWarning, LinuxBlockDevice, LinuxDiscoveryError,
    LinuxDiscoveryReport, LinuxProbePaths, LinuxRevalidationToken, discover_linux_block_devices,
};
