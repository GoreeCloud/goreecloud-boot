// SPDX-License-Identifier: GPL-3.0-or-later

use std::io::{self, Read, Seek, SeekFrom};

use crate::layout::{SectorDeviceLayout, SectorPartitionPlan};

pub const GPT_HEADER_SIZE: usize = 92;
pub const GPT_PARTITION_ENTRY_COUNT: u32 = 128;
pub const GPT_PARTITION_ENTRY_SIZE: u32 = 128;
const GPT_REVISION_1_0: u32 = 0x0001_0000;
const GPT_SIGNATURE: &[u8; 8] = b"EFI PART";

/// GUID represented in UEFI field form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GptGuid {
    data1: u32,
    data2: u16,
    data3: u16,
    data4: [u8; 8],
}

impl GptGuid {
    pub const fn new(data1: u32, data2: u16, data3: u16, data4: [u8; 8]) -> Self {
        Self {
            data1,
            data2,
            data3,
            data4,
        }
    }

    #[must_use]
    pub fn to_disk_bytes(self) -> [u8; 16] {
        let mut bytes = [0_u8; 16];
        bytes[0..4].copy_from_slice(&self.data1.to_le_bytes());
        bytes[4..6].copy_from_slice(&self.data2.to_le_bytes());
        bytes[6..8].copy_from_slice(&self.data3.to_le_bytes());
        bytes[8..16].copy_from_slice(&self.data4);
        bytes
    }
}

/// UEFI-defined EFI System Partition type GUID.
pub const EFI_SYSTEM_PARTITION_TYPE: GptGuid = GptGuid::new(
    0xC12A_7328,
    0xF81F,
    0x11D2,
    [0xBA, 0x4B, 0x00, 0xA0, 0xC9, 0x3E, 0xC9, 0x3B],
);

/// Microsoft Basic Data partition type GUID, used for the planned user-facing
/// cross-platform GCDATA partition.
pub const BASIC_DATA_PARTITION_TYPE: GptGuid = GptGuid::new(
    0xEBD0_A0A2,
    0xB9E5,
    0x4433,
    [0x87, 0xC0, 0x68, 0xB6, 0xB7, 0x26, 0x99, 0xC7],
);

/// GUIDs used by a generated GPT metadata set.
///
/// Production provisioning must generate genuinely unique disk and partition
/// GUIDs. The fixed helper identity in this module is only for development
/// sparse-image tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GptIdentity {
    pub disk_guid: GptGuid,
    pub gcboot_guid: GptGuid,
    pub gcdata_guid: GptGuid,
}

#[must_use]
pub const fn development_test_identity() -> GptIdentity {
    GptIdentity {
        disk_guid: GptGuid::new(
            0x4743_4254,
            0x0001,
            0x4000,
            [0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01],
        ),
        gcboot_guid: GptGuid::new(
            0x4743_4254,
            0x0001,
            0x4000,
            [0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02],
        ),
        gcdata_guid: GptGuid::new(
            0x4743_4254,
            0x0001,
            0x4000,
            [0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03],
        ),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GptMetadataWrite {
    pub offset_bytes: u64,
    pub data: Vec<u8>,
}

/// Complete GPT metadata writes for a planned device geometry.
///
/// This object contains only metadata buffers and offsets. It does not open or
/// write a block device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GptMetadataImage {
    pub total_bytes: u64,
    pub logical_block_size: u64,
    pub primary_header_lba: u64,
    pub backup_header_lba: u64,
    pub primary_entry_lba: u64,
    pub backup_entry_lba: u64,
    pub first_usable_lba: u64,
    pub last_usable_lba: u64,
    pub writes: Vec<GptMetadataWrite>,
}

impl GptMetadataImage {
    /// Compare all generated metadata ranges against an already opened reader.
    ///
    /// The method performs reads only and does not modify the reader.
    pub fn matches_reader<R: Read + Seek>(&self, reader: &mut R) -> io::Result<bool> {
        for write in &self.writes {
            reader.seek(SeekFrom::Start(write.offset_bytes))?;
            let mut observed = vec![0_u8; write.data.len()];
            reader.read_exact(&mut observed)?;
            if observed != write.data {
                return Ok(false);
            }
        }

        Ok(true)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GptError {
    LogicalBlockTooSmall,
    DeviceTooSmallForGpt,
    ArithmeticOverflow,
    PartitionOutsideUsableRange,
    PartitionOverlap,
    PartitionNameTooLong,
}

impl std::fmt::Display for GptError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LogicalBlockTooSmall => {
                write!(
                    formatter,
                    "GPT logical block size must be at least 512 bytes"
                )
            }
            Self::DeviceTooSmallForGpt => {
                write!(
                    formatter,
                    "device geometry is too small for redundant GPT metadata"
                )
            }
            Self::ArithmeticOverflow => write!(formatter, "GPT arithmetic overflowed"),
            Self::PartitionOutsideUsableRange => {
                write!(formatter, "planned partition falls outside GPT usable LBAs")
            }
            Self::PartitionOverlap => write!(formatter, "planned GPT partitions overlap"),
            Self::PartitionNameTooLong => {
                write!(formatter, "GPT partition name exceeds 36 UTF-16 code units")
            }
        }
    }
}

impl std::error::Error for GptError {}

/// Build a standards-shaped primary/backup GPT metadata set for the sector
/// layout without touching a block device.
pub fn build_gpt_metadata(
    layout: &SectorDeviceLayout,
    identity: GptIdentity,
) -> Result<GptMetadataImage, GptError> {
    if layout.logical_block_size < 512 {
        return Err(GptError::LogicalBlockTooSmall);
    }

    let block_size =
        usize::try_from(layout.logical_block_size).map_err(|_| GptError::ArithmeticOverflow)?;
    let entry_bytes = u64::from(GPT_PARTITION_ENTRY_COUNT)
        .checked_mul(u64::from(GPT_PARTITION_ENTRY_SIZE))
        .ok_or(GptError::ArithmeticOverflow)?;
    let entry_lbas = entry_bytes.div_ceil(layout.logical_block_size);

    let primary_header_lba = 1_u64;
    let primary_entry_lba = 2_u64;
    let first_usable_lba = primary_entry_lba
        .checked_add(entry_lbas)
        .ok_or(GptError::ArithmeticOverflow)?;
    let backup_header_lba = layout
        .total_lbas
        .checked_sub(1)
        .ok_or(GptError::DeviceTooSmallForGpt)?;
    let backup_entry_lba = backup_header_lba
        .checked_sub(entry_lbas)
        .ok_or(GptError::DeviceTooSmallForGpt)?;
    let last_usable_lba = backup_entry_lba
        .checked_sub(1)
        .ok_or(GptError::DeviceTooSmallForGpt)?;

    validate_partition(&layout.gcboot, first_usable_lba, last_usable_lba)?;
    validate_partition(&layout.gcdata, first_usable_lba, last_usable_lba)?;
    if layout.gcboot.last_lba >= layout.gcdata.first_lba {
        return Err(GptError::PartitionOverlap);
    }

    let entry_bytes_usize =
        usize::try_from(entry_bytes).map_err(|_| GptError::ArithmeticOverflow)?;
    let padded_entry_bytes = entry_lbas
        .checked_mul(layout.logical_block_size)
        .ok_or(GptError::ArithmeticOverflow)?;
    let padded_entry_bytes =
        usize::try_from(padded_entry_bytes).map_err(|_| GptError::ArithmeticOverflow)?;
    let mut entries = vec![0_u8; entry_bytes_usize];

    write_partition_entry(
        &mut entries,
        0,
        &layout.gcboot,
        EFI_SYSTEM_PARTITION_TYPE,
        identity.gcboot_guid,
    )?;
    write_partition_entry(
        &mut entries,
        1,
        &layout.gcdata,
        BASIC_DATA_PARTITION_TYPE,
        identity.gcdata_guid,
    )?;

    let entry_crc = crc32(&entries);
    let mut padded_entries = vec![0_u8; padded_entry_bytes];
    padded_entries[..entries.len()].copy_from_slice(&entries);

    let primary_header = build_header(
        block_size,
        primary_header_lba,
        backup_header_lba,
        first_usable_lba,
        last_usable_lba,
        identity.disk_guid,
        primary_entry_lba,
        entry_crc,
    )?;
    let backup_header = build_header(
        block_size,
        backup_header_lba,
        primary_header_lba,
        first_usable_lba,
        last_usable_lba,
        identity.disk_guid,
        backup_entry_lba,
        entry_crc,
    )?;
    let protective_mbr = build_protective_mbr(block_size, layout.total_lbas)?;

    let mut writes = vec![
        GptMetadataWrite {
            offset_bytes: 0,
            data: protective_mbr,
        },
        GptMetadataWrite {
            offset_bytes: lba_offset(primary_header_lba, layout.logical_block_size)?,
            data: primary_header,
        },
        GptMetadataWrite {
            offset_bytes: lba_offset(primary_entry_lba, layout.logical_block_size)?,
            data: padded_entries.clone(),
        },
        GptMetadataWrite {
            offset_bytes: lba_offset(backup_entry_lba, layout.logical_block_size)?,
            data: padded_entries,
        },
        GptMetadataWrite {
            offset_bytes: lba_offset(backup_header_lba, layout.logical_block_size)?,
            data: backup_header,
        },
    ];
    writes.sort_by_key(|write| write.offset_bytes);

    Ok(GptMetadataImage {
        total_bytes: layout.total_bytes,
        logical_block_size: layout.logical_block_size,
        primary_header_lba,
        backup_header_lba,
        primary_entry_lba,
        backup_entry_lba,
        first_usable_lba,
        last_usable_lba,
        writes,
    })
}

fn validate_partition(
    partition: &SectorPartitionPlan,
    first_usable_lba: u64,
    last_usable_lba: u64,
) -> Result<(), GptError> {
    if partition.first_lba > partition.last_lba
        || partition.first_lba < first_usable_lba
        || partition.last_lba > last_usable_lba
    {
        return Err(GptError::PartitionOutsideUsableRange);
    }

    Ok(())
}

fn write_partition_entry(
    entries: &mut [u8],
    index: usize,
    partition: &SectorPartitionPlan,
    partition_type: GptGuid,
    unique_guid: GptGuid,
) -> Result<(), GptError> {
    let entry_size =
        usize::try_from(GPT_PARTITION_ENTRY_SIZE).map_err(|_| GptError::ArithmeticOverflow)?;
    let offset = index
        .checked_mul(entry_size)
        .ok_or(GptError::ArithmeticOverflow)?;
    let end = offset
        .checked_add(entry_size)
        .ok_or(GptError::ArithmeticOverflow)?;
    let entry = entries
        .get_mut(offset..end)
        .ok_or(GptError::ArithmeticOverflow)?;

    entry[0..16].copy_from_slice(&partition_type.to_disk_bytes());
    entry[16..32].copy_from_slice(&unique_guid.to_disk_bytes());
    put_u64(entry, 32, partition.first_lba);
    put_u64(entry, 40, partition.last_lba);
    put_u64(entry, 48, 0);
    write_partition_name(entry, partition.label)?;
    Ok(())
}

fn write_partition_name(entry: &mut [u8], name: &str) -> Result<(), GptError> {
    let units: Vec<u16> = name.encode_utf16().collect();
    if units.len() > 36 {
        return Err(GptError::PartitionNameTooLong);
    }

    for (index, unit) in units.into_iter().enumerate() {
        let offset = 56 + index * 2;
        entry[offset..offset + 2].copy_from_slice(&unit.to_le_bytes());
    }

    Ok(())
}

fn build_header(
    block_size: usize,
    my_lba: u64,
    alternate_lba: u64,
    first_usable_lba: u64,
    last_usable_lba: u64,
    disk_guid: GptGuid,
    partition_entry_lba: u64,
    entry_crc: u32,
) -> Result<Vec<u8>, GptError> {
    if block_size < GPT_HEADER_SIZE {
        return Err(GptError::LogicalBlockTooSmall);
    }

    let mut header = vec![0_u8; block_size];
    header[0..8].copy_from_slice(GPT_SIGNATURE);
    put_u32(&mut header, 8, GPT_REVISION_1_0);
    put_u32(
        &mut header,
        12,
        u32::try_from(GPT_HEADER_SIZE).map_err(|_| GptError::ArithmeticOverflow)?,
    );
    put_u32(&mut header, 16, 0);
    put_u32(&mut header, 20, 0);
    put_u64(&mut header, 24, my_lba);
    put_u64(&mut header, 32, alternate_lba);
    put_u64(&mut header, 40, first_usable_lba);
    put_u64(&mut header, 48, last_usable_lba);
    header[56..72].copy_from_slice(&disk_guid.to_disk_bytes());
    put_u64(&mut header, 72, partition_entry_lba);
    put_u32(&mut header, 80, GPT_PARTITION_ENTRY_COUNT);
    put_u32(&mut header, 84, GPT_PARTITION_ENTRY_SIZE);
    put_u32(&mut header, 88, entry_crc);

    let header_crc = crc32(&header[..GPT_HEADER_SIZE]);
    put_u32(&mut header, 16, header_crc);
    Ok(header)
}

fn build_protective_mbr(block_size: usize, total_lbas: u64) -> Result<Vec<u8>, GptError> {
    if block_size < 512 || total_lbas < 2 {
        return Err(GptError::DeviceTooSmallForGpt);
    }

    let mut mbr = vec![0_u8; block_size];
    let record = &mut mbr[446..462];
    record[0] = 0x00;
    record[1..4].copy_from_slice(&[0x00, 0x02, 0x00]);
    record[4] = 0xEE;
    record[5..8].copy_from_slice(&[0xFF, 0xFF, 0xFF]);
    record[8..12].copy_from_slice(&1_u32.to_le_bytes());

    let protected_lbas = total_lbas.saturating_sub(1).min(u64::from(u32::MAX));
    let protected_lbas = u32::try_from(protected_lbas).map_err(|_| GptError::ArithmeticOverflow)?;
    record[12..16].copy_from_slice(&protected_lbas.to_le_bytes());
    mbr[510] = 0x55;
    mbr[511] = 0xAA;
    Ok(mbr)
}

fn lba_offset(lba: u64, logical_block_size: u64) -> Result<u64, GptError> {
    lba.checked_mul(logical_block_size)
        .ok_or(GptError::ArithmeticOverflow)
}

fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn put_u64(bytes: &mut [u8], offset: usize, value: u64) {
    bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFF_u32;

    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            let mask = 0_u32.wrapping_sub(crc & 1);
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }

    !crc
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{GIB, MIN_DEVICE_BYTES, plan_sector_layout};

    fn read_u32(bytes: &[u8], offset: usize) -> u32 {
        u32::from_le_bytes(
            bytes[offset..offset + 4]
                .try_into()
                .expect("four bytes must be available"),
        )
    }

    fn read_u64(bytes: &[u8], offset: usize) -> u64 {
        u64::from_le_bytes(
            bytes[offset..offset + 8]
                .try_into()
                .expect("eight bytes must be available"),
        )
    }

    fn write_at(image: &GptMetadataImage, offset: u64) -> &GptMetadataWrite {
        image
            .writes
            .iter()
            .find(|write| write.offset_bytes == offset)
            .expect("expected GPT metadata write must exist")
    }

    #[test]
    fn crc32_matches_standard_test_vector() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }

    #[test]
    fn builds_redundant_gpt_metadata_for_512_byte_blocks() {
        let layout = plan_sector_layout(64 * GIB, 512).expect("layout must be valid");
        let image = build_gpt_metadata(&layout, development_test_identity())
            .expect("GPT metadata must be generated");

        let mbr = write_at(&image, 0);
        assert_eq!(mbr.data[450], 0xEE);
        assert_eq!(&mbr.data[510..512], &[0x55, 0xAA]);

        let primary = write_at(&image, 512);
        assert_eq!(&primary.data[0..8], GPT_SIGNATURE);
        assert_eq!(read_u64(&primary.data, 24), 1);
        assert_eq!(read_u64(&primary.data, 32), layout.total_lbas - 1);
        assert_eq!(read_u64(&primary.data, 72), 2);

        let mut header_for_crc = primary.data[..GPT_HEADER_SIZE].to_vec();
        let expected_header_crc = read_u32(&header_for_crc, 16);
        header_for_crc[16..20].fill(0);
        assert_eq!(crc32(&header_for_crc), expected_header_crc);

        let entries = write_at(&image, 2 * 512);
        let entry_bytes = usize::try_from(
            u64::from(GPT_PARTITION_ENTRY_COUNT) * u64::from(GPT_PARTITION_ENTRY_SIZE),
        )
        .expect("entry array must fit in memory");
        assert_eq!(
            read_u32(&primary.data, 88),
            crc32(&entries.data[..entry_bytes])
        );
        assert_eq!(
            &entries.data[0..16],
            &EFI_SYSTEM_PARTITION_TYPE.to_disk_bytes()
        );
        assert_eq!(read_u64(&entries.data, 32), layout.gcboot.first_lba);
        assert_eq!(read_u64(&entries.data, 40), layout.gcboot.last_lba);

        let second_entry = usize::try_from(GPT_PARTITION_ENTRY_SIZE)
            .expect("partition entry size must fit in memory");
        assert_eq!(
            &entries.data[second_entry..second_entry + 16],
            &BASIC_DATA_PARTITION_TYPE.to_disk_bytes()
        );
        assert_eq!(
            read_u64(&entries.data, second_entry + 32),
            layout.gcdata.first_lba
        );
        assert_eq!(
            read_u64(&entries.data, second_entry + 40),
            layout.gcdata.last_lba
        );

        let backup_offset = (layout.total_lbas - 1) * 512;
        let backup = write_at(&image, backup_offset);
        assert_eq!(&backup.data[0..8], GPT_SIGNATURE);
        assert_eq!(read_u64(&backup.data, 24), layout.total_lbas - 1);
        assert_eq!(read_u64(&backup.data, 32), 1);
        assert_eq!(read_u64(&backup.data, 72), image.backup_entry_lba);
    }

    #[test]
    fn supports_4096_byte_logical_blocks() {
        let layout = plan_sector_layout(MIN_DEVICE_BYTES, 4096).expect("layout must be valid");
        let image = build_gpt_metadata(&layout, development_test_identity())
            .expect("GPT metadata must be generated");

        assert_eq!(image.logical_block_size, 4096);
        assert_eq!(image.primary_header_lba, 1);
        assert_eq!(image.primary_entry_lba, 2);
        assert_eq!(image.first_usable_lba, 6);
        assert_eq!(image.backup_header_lba, layout.total_lbas - 1);
        assert_eq!(image.last_usable_lba, image.backup_entry_lba - 1);
    }
}
