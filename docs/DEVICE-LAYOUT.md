# Device Layout Contract

## Status

This is the current **development planning and GPT-metadata contract** for the initial GoreeCloud Boot device layout. The Rust code can calculate byte/sector geometry and generate protective-MBR/GPT metadata for a regular-file test image, but the repository does not yet partition or format physical media.

## Units and interval model

- 1 MiB = 1,048,576 bytes.
- 1 GiB = 1,073,741,824 bytes.
- Partition byte ranges use half-open intervals: start is inclusive and end is exclusive.
- Sector partition ranges use inclusive first/last LBAs.
- Major planned byte boundaries are aligned to 1 MiB.

## Minimum planned device capacity

The current planner requires at least **8 GiB**.

This is a development policy floor, not a final product capacity recommendation. It may change after filesystem, boot-runtime, image, update, and physical-media testing.

## Initial byte layout

```text
byte 0
│
├── initial reserved/alignment region: 1 MiB
│
├── GCBOOT
│   ├── start: 1 MiB
│   ├── size: 512 MiB
│   ├── intended filesystem: FAT32
│   └── intended role: boot-system/runtime/recovery metadata
│
├── alignment to next 1 MiB boundary
│
├── GCDATA
│   ├── start: aligned after GCBOOT
│   ├── size: remaining usable bytes
│   ├── intended filesystem: exFAT, pending validation
│   └── intended role: user boot assets and related metadata
│
└── final reserved region: 1 MiB
```

## Required byte-layout invariants

A valid planned layout must satisfy all of the following:

- `GCBOOT.start < GCBOOT.end`;
- `GCDATA.start < GCDATA.end`;
- `GCBOOT.end <= GCDATA.start`;
- neither partition extends beyond the supplied device capacity;
- arithmetic must fail on overflow rather than wrap;
- a device below the minimum planning size is rejected;
- `GCDATA` must retain non-zero usable capacity after reserves and alignment.

## Logical-block geometry

The current sector planner accepts a logical block size only when it:

- is at least 512 bytes;
- is a power of two;
- is no larger than 1 MiB; and
- divides the 1 MiB alignment exactly.

The total capacity must be exactly divisible by the logical block size. Planned partition boundaries must also be block-aligned.

The test suite explicitly exercises 512-byte and 4096-byte logical-block geometry.

## Current GPT metadata model

For a valid sector plan, the development GPT generator produces:

```text
LBA 0                       protective MBR
LBA 1                       primary GPT header
LBA 2 ...                   primary partition-entry array
...
GCBOOT                      EFI System Partition type
GCDATA                      Microsoft Basic Data type
...
...                          backup partition-entry array
last LBA                    backup GPT header
```

The current generator uses:

- GPT revision 1.0 header encoding;
- 92-byte GPT header content within a full logical block;
- 128 partition entries;
- 128 bytes per partition entry;
- CRC32 for the active partition-entry bytes;
- CRC32 for the GPT header with the checksum field zeroed during calculation;
- redundant primary and backup entry arrays and headers;
- a protective MBR partition record of type `0xEE`.

The generator validates that planned partitions are within GPT usable LBAs and do not overlap.

## Partition type identities

### GCBOOT

The current GPT record uses the UEFI EFI System Partition type GUID:

`C12A7328-F81F-11D2-BA4B-00A0C93EC93B`

This identifies the planned boot-system role. The repository does not yet create a FAT32 filesystem or place UEFI boot files in this partition.

### GCDATA

The current GPT record uses Microsoft Basic Data partition type GUID:

`EBD0A0A2-B9E5-4433-87C0-68B6B72699C7`

This is the planned container type for the future cross-platform user-data filesystem. The repository does not yet create exFAT.

## GUID generation boundary

The development regular-file test helper deliberately uses fixed GoreeCloud Boot test GUIDs so generated metadata is deterministic.

**Those GUIDs are not valid for physical provisioning.** A future physical provisioner must generate unique disk and partition GUIDs using an approved source and must test uniqueness/error handling before destructive support is enabled.

## Regular-file GPT test path

The current `create-test-gpt-image` development command:

1. validates requested size and logical-block geometry;
2. generates GPT metadata in memory;
3. creates a new sparse regular file with no-overwrite semantics;
4. refuses output beneath `/dev`, `/sys`, or `/proc`;
5. sizes the file to the requested logical capacity;
6. writes only the generated MBR/GPT metadata ranges;
7. synchronizes the file;
8. reads all generated metadata ranges back and compares them byte-for-byte.

If generation/write/verification fails after file creation, the command attempts to remove the incomplete file.

This path intentionally does **not** create filesystems, install a bootloader, copy a boot runtime, or produce a bootable GoreeCloud Boot device image.

## Filesystem boundary

The source records intended filesystem identities only. It does not create FAT32 or exFAT filesystems and therefore must not be used as evidence that filesystem compatibility is implemented.

## Physical provisioning boundary

Before the GPT metadata can be written to a physical removable device, the implementation must additionally provide and validate:

- comprehensive target/storage-topology exclusion sufficient to protect the host;
- fresh stable/current-instance device revalidation;
- explicit destructive authorization;
- production-unique disk/partition GUID generation;
- image-backed end-to-end partition-table tests;
- filesystem creation and validation;
- interruption/recovery behavior;
- privilege minimization and secure device-handle/write binding;
- physical removable-media test evidence.

## Standards/reference basis

The GPT structure is implemented against the UEFI specification's GUID Partition Table model, including primary/backup headers, partition-entry arrays, protective MBR behavior, and CRC32 fields. The EFI System Partition type is defined by UEFI. The Basic Data partition type is the Microsoft-defined Basic Data GUID used for general data partitions.

These references establish metadata format facts only; they do not establish GoreeCloud Boot release readiness or physical compatibility.

## Update invariant

Routine GoreeCloud Boot runtime updates and repair are intended to target `GCBOOT` without erasing or recreating `GCDATA` unless the user explicitly invokes a full destructive reprovisioning workflow.
