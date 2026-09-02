// SPDX-License-Identifier: GPL-3.0-or-later

use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootKind {
    EfiApplication,
    LinuxImage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Architecture {
    X86_64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogEntry {
    pub id: String,
    pub display_name: String,
    pub relative_path: String,
    pub boot_kind: BootKind,
    pub architecture: Architecture,
    pub sha256: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CatalogError {
    EmptyId,
    InvalidId,
    EmptyDisplayName,
    UnsafePath,
    InvalidSha256,
}

impl Display for CatalogError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyId => write!(formatter, "catalog entry id is empty"),
            Self::InvalidId => write!(formatter, "catalog entry id contains unsupported characters"),
            Self::EmptyDisplayName => write!(formatter, "catalog display name is empty"),
            Self::UnsafePath => write!(formatter, "catalog path is not a safe relative path"),
            Self::InvalidSha256 => write!(formatter, "SHA-256 metadata must be 64 hexadecimal characters"),
        }
    }
}

impl Error for CatalogError {}

impl CatalogEntry {
    pub fn validate(&self) -> Result<(), CatalogError> {
        validate_id(&self.id)?;

        if self.display_name.trim().is_empty() {
            return Err(CatalogError::EmptyDisplayName);
        }

        validate_relative_path(&self.relative_path)?;

        if let Some(hash) = &self.sha256 {
            validate_sha256(hash)?;
        }

        Ok(())
    }
}

fn validate_id(id: &str) -> Result<(), CatalogError> {
    if id.is_empty() {
        return Err(CatalogError::EmptyId);
    }

    if id.len() > 128
        || !id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        return Err(CatalogError::InvalidId);
    }

    Ok(())
}

fn validate_relative_path(path: &str) -> Result<(), CatalogError> {
    if path.is_empty() || path.starts_with('/') || path.contains('\\') || path.contains('\0') {
        return Err(CatalogError::UnsafePath);
    }

    for segment in path.split('/') {
        if segment.is_empty() || matches!(segment, "." | "..") {
            return Err(CatalogError::UnsafePath);
        }
    }

    Ok(())
}

fn validate_sha256(hash: &str) -> Result<(), CatalogError> {
    if hash.len() != 64 || !hash.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(CatalogError::InvalidSha256);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_entry() -> CatalogEntry {
        CatalogEntry {
            id: "linux-rescue-1".to_owned(),
            display_name: "Linux Rescue".to_owned(),
            relative_path: "images/linux-rescue.iso".to_owned(),
            boot_kind: BootKind::LinuxImage,
            architecture: Architecture::X86_64,
            sha256: Some("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_owned()),
        }
    }

    #[test]
    fn accepts_valid_metadata() {
        assert_eq!(valid_entry().validate(), Ok(()));
    }

    #[test]
    fn rejects_parent_traversal() {
        let mut entry = valid_entry();
        entry.relative_path = "images/../outside.iso".to_owned();
        assert_eq!(entry.validate(), Err(CatalogError::UnsafePath));
    }

    #[test]
    fn rejects_absolute_and_backslash_paths() {
        let mut entry = valid_entry();
        entry.relative_path = "/etc/passwd".to_owned();
        assert_eq!(entry.validate(), Err(CatalogError::UnsafePath));

        entry.relative_path = "images\\escape.iso".to_owned();
        assert_eq!(entry.validate(), Err(CatalogError::UnsafePath));
    }

    #[test]
    fn rejects_malformed_hash() {
        let mut entry = valid_entry();
        entry.sha256 = Some("not-a-hash".to_owned());
        assert_eq!(entry.validate(), Err(CatalogError::InvalidSha256));
    }
}
