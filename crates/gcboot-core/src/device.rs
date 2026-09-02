// SPDX-License-Identifier: GPL-3.0-or-later

use crate::layout::MIN_DEVICE_BYTES;

/// Evidence about a candidate block device.
///
/// In the current development foundation these values are supplied explicitly
/// by the caller. A future write-capable implementation must discover and
/// revalidate authoritative device identity instead of trusting user-supplied
/// booleans.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceEvidence {
    pub device_path: String,
    pub size_bytes: u64,
    pub removable: bool,
    pub contains_mounted_root: bool,
    pub contains_mounted_boot: bool,
    pub read_only: bool,
}

/// Conservative eligibility result for a candidate provisioning target.
///
/// `eligible` means only that the supplied evidence passed the current policy.
/// It is not destructive-operation authorization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetAssessment {
    pub eligible: bool,
    pub reasons: Vec<&'static str>,
}

impl TargetAssessment {
    #[must_use]
    pub fn evaluate(evidence: &DeviceEvidence) -> Self {
        let mut reasons = Vec::new();

        if evidence.device_path.trim().is_empty() {
            reasons.push("device path is empty");
        }
        if !evidence.removable {
            reasons.push("target is not positively identified as removable");
        }
        if evidence.read_only {
            reasons.push("target is read-only");
        }
        if evidence.contains_mounted_root {
            reasons.push("target contains the mounted root filesystem");
        }
        if evidence.contains_mounted_boot {
            reasons.push("target contains the mounted boot filesystem");
        }
        if evidence.size_bytes < MIN_DEVICE_BYTES {
            reasons.push("target is smaller than the minimum planning size");
        }

        Self {
            eligible: reasons.is_empty(),
            reasons,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn safe_evidence() -> DeviceEvidence {
        DeviceEvidence {
            device_path: "/dev/sdz".to_owned(),
            size_bytes: MIN_DEVICE_BYTES,
            removable: true,
            contains_mounted_root: false,
            contains_mounted_boot: false,
            read_only: false,
        }
    }

    #[test]
    fn accepts_eligible_supplied_evidence() {
        let result = TargetAssessment::evaluate(&safe_evidence());
        assert!(result.eligible);
        assert!(result.reasons.is_empty());
    }

    #[test]
    fn rejects_system_disk_evidence() {
        let mut evidence = safe_evidence();
        evidence.contains_mounted_root = true;
        evidence.contains_mounted_boot = true;

        let result = TargetAssessment::evaluate(&evidence);
        assert!(!result.eligible);
        assert_eq!(result.reasons.len(), 2);
    }

    #[test]
    fn rejects_non_removable_media() {
        let mut evidence = safe_evidence();
        evidence.removable = false;

        let result = TargetAssessment::evaluate(&evidence);
        assert!(!result.eligible);
        assert!(
            result
                .reasons
                .contains(&"target is not positively identified as removable")
        );
    }
}
