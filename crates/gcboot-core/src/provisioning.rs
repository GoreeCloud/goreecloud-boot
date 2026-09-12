// SPDX-License-Identifier: GPL-3.0-or-later

//! Non-destructive provisioning-plan evidence.
//!
//! This module deliberately contains no block-device write operation. It binds a
//! sector layout to one exact read-only Linux discovery snapshot so a future
//! write-capable milestone can prove it revalidated the same target immediately
//! before seeking separate destructive-operation authorization.

use std::error::Error;
use std::fmt::{Display, Formatter};
use std::path::PathBuf;

use crate::layout::{LayoutError, SectorDeviceLayout, plan_sector_layout};
use crate::linux::{LinuxBlockDevice, LinuxRevalidationToken};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinuxProvisioningPlan {
    pub target_devnode: PathBuf,
    pub revalidation_token: LinuxRevalidationToken,
    pub layout: SectorDeviceLayout,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinuxProvisioningRevalidation {
    pub target_path_matches: bool,
    pub token_matches: bool,
    pub target_still_eligible: bool,
    pub layout_matches_current_geometry: bool,
    pub planning_evidence_current: bool,
    pub destructive_write_authorized: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinuxProvisioningPlanError {
    TargetIneligible(Vec<&'static str>),
    Layout(LayoutError),
}

impl Display for LinuxProvisioningPlanError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TargetIneligible(reasons) => {
                write!(
                    formatter,
                    "target is ineligible for provisioning planning: {}",
                    reasons.join(", ")
                )
            }
            Self::Layout(error) => write!(formatter, "cannot plan target layout: {error}"),
        }
    }
}

impl Error for LinuxProvisioningPlanError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Layout(error) => Some(error),
            Self::TargetIneligible(_) => None,
        }
    }
}

impl From<LayoutError> for LinuxProvisioningPlanError {
    fn from(value: LayoutError) -> Self {
        Self::Layout(value)
    }
}

impl LinuxProvisioningPlan {
    pub fn from_discovered_device(
        device: &LinuxBlockDevice,
    ) -> Result<Self, LinuxProvisioningPlanError> {
        let assessment = device.assessment();
        if !assessment.eligible {
            return Err(LinuxProvisioningPlanError::TargetIneligible(
                assessment.reasons,
            ));
        }
        let layout = plan_sector_layout(device.size_bytes, device.logical_block_size)?;
        Ok(Self {
            target_devnode: device.devnode.clone(),
            revalidation_token: device.revalidation_token(),
            layout,
        })
    }

    /// Compare a fresh read-only discovery result with the exact evidence that
    /// produced this plan. Even a complete match is planning evidence only.
    #[must_use]
    pub fn revalidate(&self, current: &LinuxBlockDevice) -> LinuxProvisioningRevalidation {
        let target_path_matches = self.target_devnode == current.devnode;
        let token_matches = self.revalidation_token.matches(current);
        let target_still_eligible = current.assessment().eligible;
        let current_layout =
            plan_sector_layout(current.size_bytes, current.logical_block_size).ok();
        let layout_matches_current_geometry = current_layout.as_ref() == Some(&self.layout);
        let planning_evidence_current = target_path_matches
            && token_matches
            && target_still_eligible
            && layout_matches_current_geometry;

        LinuxProvisioningRevalidation {
            target_path_matches,
            token_matches,
            target_still_eligible,
            layout_matches_current_geometry,
            planning_evidence_current,
            destructive_write_authorized: false,
        }
    }

    /// Physical writes remain impossible to authorize in the current milestone.
    #[must_use]
    pub const fn destructive_write_authorized(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::MIN_DEVICE_BYTES;
    use crate::linux::DeviceNumber;

    fn eligible_device() -> LinuxBlockDevice {
        let number = DeviceNumber {
            major: 8,
            minor: 240,
        };
        LinuxBlockDevice {
            kernel_name: "sdz".to_owned(),
            devnode: PathBuf::from("/dev/sdz"),
            device_number: number,
            partition_device_numbers: Vec::new(),
            topology_device_numbers: vec![number],
            mounted_topology_device_numbers: Vec::new(),
            active_swap_topology_device_numbers: Vec::new(),
            mount_namespace_ids: vec!["mnt:[100]".to_owned()],
            mount_namespace_coverage_complete: true,
            size_bytes: MIN_DEVICE_BYTES,
            logical_block_size: 512,
            physical_block_size: 4096,
            removable: true,
            read_only: false,
            contains_mounted_root: false,
            contains_mounted_boot: false,
            contains_mounted_filesystem: false,
            contains_active_swap: false,
            diskseq: Some(42),
            vendor: Some("Test".to_owned()),
            model: Some("Disposable".to_owned()),
            serial: Some("serial-1".to_owned()),
            wwid: Some("wwid-1".to_owned()),
            persistent_aliases: Vec::new(),
        }
    }

    #[test]
    fn exact_fresh_evidence_can_only_make_planning_evidence_current() {
        let device = eligible_device();
        let plan = LinuxProvisioningPlan::from_discovered_device(&device)
            .expect("eligible device should plan");
        let revalidation = plan.revalidate(&device);
        assert!(revalidation.planning_evidence_current);
        assert!(!revalidation.destructive_write_authorized);
        assert!(!plan.destructive_write_authorized());
    }

    #[test]
    fn device_replacement_or_runtime_safety_change_invalidates_plan() {
        let device = eligible_device();
        let plan = LinuxProvisioningPlan::from_discovered_device(&device)
            .expect("eligible device should plan");

        let mut replacement = device.clone();
        replacement.diskseq = Some(43);
        let replaced = plan.revalidate(&replacement);
        assert!(!replaced.token_matches);
        assert!(!replaced.planning_evidence_current);
        assert!(!replaced.destructive_write_authorized);

        let mut mounted = device.clone();
        mounted.contains_mounted_filesystem = true;
        mounted.mounted_topology_device_numbers = vec![mounted.device_number];
        let active = plan.revalidate(&mounted);
        assert!(!active.target_still_eligible);
        assert!(!active.planning_evidence_current);
        assert!(!active.destructive_write_authorized);
    }

    #[test]
    fn ineligible_target_cannot_produce_a_plan() {
        let mut device = eligible_device();
        device.removable = false;
        let error = LinuxProvisioningPlan::from_discovered_device(&device)
            .expect_err("non-removable target must not produce a plan");
        assert!(matches!(
            error,
            LinuxProvisioningPlanError::TargetIneligible(_)
        ));
    }
}
