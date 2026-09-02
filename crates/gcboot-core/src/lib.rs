// SPDX-License-Identifier: GPL-3.0-or-later
//
// GoreeCloud Boot core primitives. The current crate is intentionally limited
// to non-destructive planning and validation. It contains no block-device
// write path.

#![forbid(unsafe_code)]

pub mod catalog;
pub mod device;
pub mod layout;

pub use catalog::{Architecture, BootKind, CatalogEntry, CatalogError};
pub use device::{DeviceEvidence, TargetAssessment};
pub use layout::{DeviceLayout, LayoutError, PartitionPlan, plan_layout};
