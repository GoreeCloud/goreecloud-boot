# Capabilities

## Overview

GoreeCloud Boot is currently a **development foundation**, not a bootable multiboot product release. The verified implementation now includes read-only Linux device discovery with bounded recursive topology analysis, conservative target assessment, byte- and sector-aware layout planning, GPT metadata generation for regular-file test images, catalog validation, and development tooling.

No current capability opens a physical block device for writing.

## Core Capabilities

- Calculates a deterministic proposed removable-media layout with:
  - 1 MiB initial alignment;
  - a 512 MiB `GCBOOT` FAT32-designated region;
  - an aligned `GCDATA` remainder;
  - a 1 MiB final reserve.
- Converts the byte layout into checked logical-block ranges when the supplied logical block size is a supported power-of-two value that divides the 1 MiB alignment.
- Performs read-only Linux whole-device discovery from kernel/system metadata, including:
  - `/sys/block` device identity and capacity;
  - major/minor device numbers;
  - logical and physical block sizes;
  - removable and read-only state;
  - Linux `diskseq` when available;
  - vendor, model, serial, and WWID metadata when exposed;
  - `/dev/disk/by-id` aliases when resolvable;
  - direct child-partition major/minor identities;
  - recursive upward traversal of sysfs `holders` relationships starting from the whole device and its direct partitions;
  - all mounted major/minor identities from `/proc/self/mountinfo`, with specific root and `/boot`/`/boot/*` evidence retained.
- Produces a revalidation token from current Linux evidence so a later probe can detect relevant device replacement, state, topology, or mounted-topology changes. The token includes current removable/read-only state, topology device numbers, and the topology intersection with mountinfo in addition to identity/geometry evidence. Matching tokens remain evidence only and are not destructive authorization.
- Conservatively rejects candidate evidence that is:
  - non-removable;
  - read-only;
  - below the minimum supported planning size;
  - known to contain the mounted root filesystem;
  - known to contain a mounted boot filesystem;
  - known to have any mounted filesystem in the discovered device topology.
- Omits a Linux candidate rather than assuming eligibility when mandatory per-device metadata or recursive holder-topology evidence cannot be read safely.
- Generates standards-shaped GPT metadata in memory for the planned layout, including:
  - protective MBR;
  - primary and backup GPT headers;
  - primary and backup partition-entry arrays;
  - GPT CRC32 values;
  - EFI System Partition type for `GCBOOT`;
  - Basic Data partition type for `GCDATA`;
  - development-only fixed GUID identities for test images.
- Validates initial catalog-entry metadata, including safe relative paths and optional SHA-256 string formatting.

## User Capabilities

A developer can currently use `bootctl` to:

- run `plan-device` with explicit development evidence, including whether any filesystem is mounted on the supplied target evidence, and receive a non-destructive byte-layout proposal;
- run `list-linux-devices` to inspect read-only Linux block-device metadata, bounded recursive topology evidence, mounted-topology state, and current target assessment;
- run `plan-linux-device --device PATH` to select a discovered whole device by device node or discovered `by-id` alias and receive a sector-aware plan without opening the target for writing;
- run `create-test-gpt-image` to create a **new** sparse regular file containing the generated protective-MBR/GPT metadata and verify that metadata by reading it back.

`create-test-gpt-image` uses no-overwrite file creation and refuses output under `/dev`, `/sys`, or `/proc`. It does not create FAT32 or exFAT filesystems and does not create a bootable USB image.

## Administrative Capabilities

There is no production management interface, privileged physical-write path, account system, centrally managed trust store, production device inventory, or fleet-management capability.

The read-only Linux discovery command provides development inspection only.

## Platform Integrations

### Glaze UI

**Not implemented.** Firmware-adapted Glaze UI requirements are documented for the future preboot interface.

### Wardveil Security

**Not implemented as a platform integration.** The repository now has stronger native safety controls—read-only device discovery, bounded recursive holder-topology analysis, mounted-filesystem exclusion, topology-aware revalidation tokens, checked sector arithmetic, GPT CRC generation, and no-overwrite regular-file test-image creation—but it does not yet provide Wardveil release provenance, cryptographic image verification, signed trust policy, or tamper response.

### Privacy Shield

**Not implemented as a platform integration.** Current code has no network or telemetry capability and therefore does not transmit discovered device or image information. No Privacy Shield contract or control interface is implemented yet.

### Everkeep

**Not implemented.** No physical-device repair, backup, restore, or reconstruction workflow exists yet.

### GoreeCloud Mesh

**Not implemented.** There is no Mesh capability or runtime dependency.

### GoreeCloud Identity

**Not implemented and not currently required for the offline development foundation.** No authentication or delegated administration capability exists.

## Data and Interoperability

- Catalog paths are represented as relative paths.
- Optional SHA-256 metadata is validated syntactically.
- Linux discovery reads standard kernel/sysfs and mount metadata without a third-party runtime service.
- Recursive topology evidence follows sysfs `holders` links upward from the candidate whole disk and direct partitions.
- GPT metadata follows a protective-MBR plus redundant primary/backup GPT structure and supports the current `GCBOOT`/`GCDATA` partition plan.
- Generated test images are sparse regular files and contain partition-table metadata only.
- Current code uses only Rust standard-library functionality and introduces no external Rust runtime dependency.

FAT32 creation, exFAT creation, ISO/IMG/EFI/WIM/VHD/VHDX boot interoperability, GNU GRUB, EDK II, iPXE, and filesystem-level image management are not implemented yet.

## Supported Platforms and Interfaces

- Source/build interface: Rust/Cargo.
- Development CLI: `bootctl`.
- Read-only host-device discovery: Linux.
- Sector planning and GPT metadata generation: platform-independent Rust core logic.
- Future first firmware target: UEFI x86-64.

No physical hardware platform is production-validated.

## Security and Privacy Capabilities

Current implemented safeguards include:

- conservative target rejection;
- read-only Linux discovery rather than trusting a device path alone;
- recursive sysfs `holders` traversal from a whole device and its direct partitions;
- rejection when any mountinfo-reported filesystem intersects the discovered topology;
- specific mounted root/boot detection for clearer rejection evidence;
- current-instance `diskseq`, major/minor, capacity, logical-block-size, removable/read-only state, persistent-alias/WWID, serial, topology, and mounted-topology evidence in a revalidation token;
- fail-closed omission of a candidate when mandatory metadata/topology evidence cannot be read;
- checked byte and sector arithmetic;
- GPT usable-range and overlap validation;
- traversal-resistant catalog-path validation;
- strict optional SHA-256 metadata formatting;
- no block-device write path;
- no-overwrite sparse test-image creation outside protected pseudo-filesystem paths;
- read-back verification of generated GPT test-image metadata;
- absence of telemetry or network code.

These safeguards are development controls, not a complete security certification or destructive-operation authorization model.

## Resilience, Backup, and Recovery Capabilities

No production recovery operation is implemented. The separate `GCBOOT`/`GCDATA` model and redundant GPT metadata generation establish testable foundations for future non-destructive repair, but current code does not repair physical media.

## Accessibility Capabilities

No graphical or firmware user interface is implemented yet. Accessibility requirements for the future preboot interface remain planned.

## Automation and API Capabilities

- Cargo tests validate current safety, discovery/topology, layout, GPT, and catalog rules.
- Linux discovery tests use synthetic filesystem/sysfs fixtures rather than CI-runner block devices.
- Synthetic topology tests cover a mounted direct partition, recursive holder relationships, and revalidation-token changes when mount state changes.
- GPT tests validate CRC32, protective-MBR structure, redundant headers, partition types, and 512/4096-byte logical-block planning.
- The repository includes CI configuration for formatting, linting, and tests.
- No external API is exposed.

## Current Limitations

The repository cannot currently:

- authorize or perform physical block-device writes;
- safely account for every Linux system-storage relationship: recursive `holders` traversal is implemented, but swap is not represented by mountinfo and exhaustive device-mapper, multipath, RAID, encryption, hotplug, namespace, and other topology cases are not yet validated for destructive use;
- partition or format removable media;
- create FAT32 or exFAT filesystems;
- install a bootloader or UEFI runtime;
- create a bootable USB;
- boot any ISO, IMG, EFI application, Windows media, or Linux image;
- generate production-unique GPT disk/partition GUIDs for physical provisioning;
- verify file contents cryptographically;
- provide Secure Boot;
- provide persistence;
- provide network boot;
- provide ARM64 or legacy BIOS support;
- provide a graphical preboot or host-side management interface;
- provide production recovery or repair.

## Capability Validation

Current capabilities are validated by Rust unit tests, synthetic Linux discovery/topology fixtures, generated GPT test metadata, and source review. The regular-file GPT path validates generated metadata without physical media.

No QEMU/OVMF boot evidence, physical removable-media provisioning evidence, filesystem-formatting evidence, hardware compatibility evidence, Stable release qualification, or production acceptance exists yet.
