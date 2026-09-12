# Capabilities

## Overview

GoreeCloud Boot is currently a **development foundation**, not a bootable multiboot product release. The verified implementation now includes read-only Linux device discovery with bounded bidirectional storage topology, visible mount-namespace analysis, active-swap analysis, conservative target assessment, byte- and sector-aware layout planning, GPT metadata generation for regular-file test images, catalog validation, and development tooling.

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
  - recursive bidirectional traversal of sysfs `holders` and `slaves` relationships starting from the whole device and its direct partitions;
  - canonical-path cycle protection and sorted/deduplicated topology device identities;
  - distinct caller-visible mount namespace identities discovered through `/proc/<pid>/ns/mnt`;
  - mounted major/minor identities and mount points unioned from readable distinct caller-visible `/proc/<pid>/mountinfo` views, with specific root and `/boot`/`/boot/*` evidence retained;
  - active swap areas from `/proc/swaps`, resolving swap partitions to sysfs major/minor identities and swap files against their deepest containing visible mount evidence.
- The bidirectional topology closure can include sibling backing members reached through a shared holder, allowing mount or active-swap state on those related members to reject the candidate.
- A filesystem mounted only in another readable caller-visible mount namespace can reject the candidate.
- If a caller-visible process or distinct visible mount namespace cannot be safely inspected, the probe records incomplete visible namespace coverage and every candidate assessment becomes ineligible rather than treating incomplete mount evidence as safe.
- Produces a revalidation token from current Linux evidence so a later probe can detect relevant device replacement, state, topology, mounted-topology, active-swap, visible mount-namespace set, or namespace-coverage changes. Matching tokens remain evidence only and are not destructive authorization.
- Conservatively rejects candidate evidence that is:
  - non-removable;
  - read-only;
  - below the minimum supported planning size;
  - known to contain the mounted root filesystem;
  - known to contain a mounted boot filesystem;
  - known to have any mounted filesystem in the discovered device topology across readable visible mount namespaces;
  - known to contain active swap in the discovered device topology;
  - based on incomplete visible mount-namespace coverage.
- Omits a Linux candidate rather than assuming eligibility when mandatory per-device metadata or a required `holders`/`slaves` topology relation cannot be read safely.
- Fails Linux discovery rather than returning potentially eligible targets when mandatory global active-swap metadata is unreadable, malformed, unsupported, or cannot be resolved to a device/filesystem identity.
- Fails Linux discovery when an active swap file has ambiguous deepest backing filesystems across the readable visible mount-namespace evidence rather than selecting one arbitrarily.
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

- run `plan-device` with explicit development evidence, including whether any filesystem is mounted or active swap is present on the supplied target evidence, and receive a non-destructive byte-layout proposal;
- run `list-linux-devices` to inspect read-only Linux block-device metadata, bounded bidirectional topology evidence, mounted-topology state, active-swap topology state, visible mount-namespace identities, namespace-coverage completeness, and current target assessment;
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

**Not implemented as a platform integration.** The repository now has stronger native safety controls—read-only device discovery, bounded bidirectional holder/slave topology closure, visible mount-namespace analysis with conservative incomplete-coverage rejection, mounted-filesystem exclusion, active-swap exclusion, topology/active-use/namespace-aware revalidation tokens, checked sector arithmetic, GPT CRC generation, and no-overwrite regular-file test-image creation—but it does not yet provide Wardveil release provenance, cryptographic image verification, signed trust policy, or tamper response.

### Privacy Shield

**Not implemented as a platform integration.** Current code has no network or telemetry capability and therefore does not transmit discovered device, process-namespace, or image information. Mount-namespace identities are used locally as development safety evidence. No Privacy Shield contract or control interface is implemented yet.

### Everkeep

**Not implemented.** No physical-device repair, backup, restore, or reconstruction workflow exists yet.

### GoreeCloud Mesh

**Not implemented.** There is no Mesh capability or runtime dependency.

### GoreeCloud Identity

**Not implemented and not currently required for the offline development foundation.** No authentication or delegated administration capability exists.

## Data and Interoperability

- Catalog paths are represented as relative paths.
- Optional SHA-256 metadata is validated syntactically.
- Linux discovery reads standard kernel/sysfs, caller-visible procfs mount-namespace/mount, and active-swap metadata without a third-party runtime service.
- Topology evidence recursively follows both sysfs `holders` and `slaves` links, with canonical-path cycle protection.
- Shared stacked devices can connect multiple backing members into one candidate safety topology.
- Visible mount namespaces are deduplicated by namespace identity and their readable mountinfo evidence is unioned for safety assessment.
- Namespace coverage is explicitly bounded to what is visible and readable through the caller’s `/proc`; the implementation does not claim to prove the absence of hidden or inaccessible namespaces.
- Active swap partitions are resolved to sysfs major/minor identities. Swap-file backing resolution fails closed if deepest visible mount evidence is ambiguous across namespaces.
- GPT metadata follows a protective-MBR plus redundant primary/backup GPT structure and supports the current `GCBOOT`/`GCDATA` partition plan.
- Generated test images are sparse regular files and contain partition-table metadata only.
- Current code uses only Rust standard-library functionality and introduces no external Rust runtime dependency.

FAT32 creation, exFAT creation, ISO/IMG/EFI/WIM/VHD/VHDX boot interoperability, GNU GRUB, EDK II, iPXE, and filesystem-level image management are not implemented yet.

## Supported Platforms and Interfaces

- Source/build interface: Rust/Cargo.
- Development CLI: `bootctl`.
- Read-only host-device discovery: Linux.
- Visible mount-namespace analysis: Linux procfs as visible to the calling process.
- Sector planning and GPT metadata generation: platform-independent Rust core logic.
- Future first firmware target: UEFI x86-64.

No physical hardware platform is production-validated.

## Security and Privacy Capabilities

Current implemented safeguards include:

- conservative target rejection;
- read-only Linux discovery rather than trusting a device path alone;
- bidirectional sysfs `holders`/`slaves` topology closure from a whole device and its direct partitions;
- canonical-path cycle protection for reciprocal relation links;
- enumeration and deduplication of caller-visible mount namespaces under `/proc`;
- union of mount evidence from readable distinct visible namespaces;
- rejection when any visible mountinfo-reported filesystem intersects the discovered topology;
- candidate rejection when caller-visible mount-namespace coverage is incomplete;
- active-swap discovery from `/proc/swaps` and rejection when resolved swap state intersects the discovered topology;
- fail-closed ambiguous swap-file backing resolution across visible namespaces;
- specific mounted root/boot detection for clearer rejection evidence;
- current-instance `diskseq`, major/minor, capacity, logical-block-size, removable/read-only state, persistent-alias/WWID, serial, topology, mounted-topology, active-swap-topology, mount-namespace identity, and namespace-coverage evidence in a revalidation token;
- fail-closed omission of a candidate when mandatory per-device metadata/topology evidence cannot be read;
- fail-closed Linux discovery when mandatory global swap evidence cannot be read or resolved safely;
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

- Cargo tests validate current safety, discovery/topology/mount-namespace/swap handling, layout, GPT, and catalog rules.
- Linux discovery tests use synthetic filesystem/sysfs/procfs fixtures rather than CI-runner block devices.
- Synthetic topology tests cover a mounted direct partition, recursive holder relationships, reciprocal holder/slave links, mounted and active-swap sibling backing members reached through a shared holder, fail-closed unresolved swap evidence, and revalidation-token changes when mount, swap, or slave-connected topology state changes.
- Synthetic namespace tests cover a target mounted only in another readable visible mount namespace, incomplete visible namespace coverage, ambiguous swap-file backing across namespaces, and revalidation-token invalidation when the visible namespace set changes.
- GPT tests validate CRC32, protective-MBR structure, redundant headers, partition types, and 512/4096-byte logical-block planning.
- The repository includes CI configuration for formatting, linting, and tests.
- No external API is exposed.

## Current Limitations

The repository cannot currently:

- authorize or perform physical block-device writes;
- safely claim exhaustive Linux destructive-target qualification: bidirectional `holders`/`slaves` closure, visible mount-namespace analysis, mounted-filesystem exclusion, and active-swap exclusion are implemented, but device-mapper, multipath, RAID, encryption, hotplug/reconfiguration, procfs/PID/mount-namespace visibility constraints, unusual storage configurations, and other topology/state cases are not yet comprehensively validated for destructive use;
- prove that mount namespaces hidden from or inaccessible to the calling process do not exist;
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

Current capabilities are validated by Rust unit tests, synthetic Linux discovery/topology/mount-namespace/swap fixtures, generated GPT test metadata, source review, and GitHub Actions formatting/lint/test gates. The regular-file GPT path validates generated metadata without physical media.

Visible mount-namespace behavior is validated synthetically for the implemented caller-visible `/proc` model; it is not production qualification across containers, restrictive procfs configurations, PID namespaces, `hidepid`, privilege boundaries, or representative complex storage stacks.

No QEMU/OVMF boot evidence, physical removable-media provisioning evidence, filesystem-formatting evidence, hardware compatibility evidence, Stable release qualification, or production acceptance exists yet.