# GoreeCloud Boot Specifications

## 1. Repository state

- **Product:** GoreeCloud Boot
- **Development model:** Original GoreeCloud-native software
- **Lifecycle:** Development foundation; not Stable or production-approved
- **Current development version:** `0.1.0-dev.2`
- **Initial host platform:** Linux
- **Initial firmware target:** UEFI x86-64
- **GoreeCloud-owned source license:** GPL-3.0-or-later
- **Ventoy relationship:** Capability reference only; no Ventoy source is included

This file is the repository-local, version-coupled specification. The canonical GoreeCloud project record remains `Project Specification — Boot` under `GoreeCloud/Projects`.

## 2. Product purpose

GoreeCloud Boot is intended to provide a removable-media platform that can hold multiple explicitly supported boot assets while preserving a stable data partition between boot-runtime updates.

Core use cases include operating-system installation, live environments, recovery, diagnostics, GoreeCloud maintenance, and future trusted recovery workflows.

## 3. Required principles

The product must remain:

- open source and independently maintainable;
- offline-first for core boot and provisioning functions;
- free of mandatory vendor accounts, subscriptions, hosted control planes, advertising, and telemetry;
- conservative around destructive device operations;
- explicit about supported and unsupported image types;
- recoverable without requiring a proprietary service;
- truthful about implementation and validation state.

## 4. Initial device layout

The intended default physical layout is GPT with a protective MBR where compatible.

### GCBOOT

- purpose: boot-system partition;
- planned filesystem: FAT32;
- current planned size: 512 MiB;
- current GPT type: EFI System Partition type GUID;
- contains future UEFI boot chain, runtime, trusted configuration, verification material, and recovery metadata;
- must be repairable or replaceable without erasing `GCDATA` where practical.

### GCDATA

- purpose: large user-facing data partition;
- planned filesystem: exFAT, subject to validation before release;
- current GPT type: Microsoft Basic Data partition type GUID;
- occupies remaining usable device space after alignment and end reserve;
- may later contain supported boot images, checksums, signatures, persistence data, and sidecar metadata.

The current Rust foundation implements byte/sector layout calculation and protective-MBR/GPT metadata generation for development regular-file images only. It does not partition physical devices or create filesystems.

## 5. Current implemented foundation

The repository currently implements these development capabilities:

- deterministic `GCBOOT`/`GCDATA` byte-layout planning;
- checked conversion to logical-block ranges for supported sector sizes, including 512-byte and 4096-byte logical blocks;
- conservative target-safety assessment;
- read-only Linux whole-block-device discovery from sysfs, mountinfo, active-swap metadata, and available persistent aliases;
- collection of current device identity evidence including major/minor number, capacity, logical/physical block size, `diskseq` when available, and available WWID/serial/`by-id` identity;
- directly enumerated child-partition identities plus recursive upward sysfs `holders` traversal to build a bounded candidate topology;
- association of the discovered topology with every mountinfo-reported major/minor identity, with explicit root/boot state retained and any mounted topology rejected;
- active-swap discovery from `/proc/swaps`, including sysfs resolution of swap partitions and backing-filesystem resolution of swap files, with rejection when active swap intersects the discovered topology;
- fail-closed omission of a candidate when mandatory per-device or holder-topology evidence cannot be read safely;
- fail-closed Linux discovery when mandatory global active-swap evidence cannot be read, parsed, or resolved safely;
- revalidation tokens that bind identity/geometry, removable/read-only state, discovered topology, mounted-topology intersection, and active-swap-topology intersection for later comparison;
- in-memory protective-MBR and redundant GPT header/entry-array generation with CRC32;
- development-only sparse regular-file GPT image creation with no-overwrite behavior and read-back metadata verification;
- initial catalog-entry validation;
- development `bootctl` inspection/planning/test-image commands;
- unit and synthetic-fixture tests for those rules, including recursive holder topology, mounted-topology changes, active swap partitions/files/holder devices, and swap-state revalidation changes.

No physical block-device write, filesystem formatting, bootloader installation, image booting, cryptographic content verification, or production recovery write path is currently implemented.

## 6. Linux device-discovery requirements

The Linux discovery layer is read-only and must remain separable from future destructive authorization.

Current discovery reads standard Linux metadata sufficient to identify whole block devices, directly enumerated child partitions, a bounded upward holder topology, mounted filesystems, and current active swap areas. It must not treat a mutable path such as `/dev/sdb` as stable identity by itself.

Current discovery evidence includes, where available:

- whole-device major/minor identity;
- child-partition major/minor identities;
- recursively discovered sysfs `holders` device identities;
- kernel device name and current `/dev` node;
- capacity;
- logical and physical block sizes;
- removable and read-only state;
- `diskseq` current-instance identity;
- vendor, model, serial, and WWID;
- resolved `/dev/disk/by-id` aliases;
- all mounted major/minor identities and mount points from mountinfo;
- explicit mounted root and boot associations;
- active swap entries from `/proc/swaps`, resolved to major/minor identity either directly through sysfs for swap partitions or through the deepest containing mountinfo filesystem for swap files.

A candidate topology currently starts with the whole device plus its direct partitions and recursively follows `holders` links upward. Discovery failure or missing mandatory per-device device/topology data must fail conservatively for the affected device rather than fabricating eligibility.

Any mounted filesystem or resolved active swap area whose major/minor identity intersects the discovered topology makes the target ineligible. Global active-swap metadata is mandatory: if `/proc/swaps` cannot be read or an active entry cannot be parsed/resolved safely, Linux discovery must fail rather than return candidates based on incomplete host-storage evidence.

This is a stronger development safeguard than root/boot-only matching, but it is not complete destructive-target qualification. Before any physical destructive write path can be enabled, Linux topology/state analysis must be extended or independently validated across relevant device-mapper, encrypted mapping, software RAID, multipath, hotplug/reconfiguration, mount/swap namespace, unusual swap/storage, and other relationships that could hide or introduce a system-disk dependency.

## 7. Target-safety and authorization requirements

A future destructive provisioning path must not run merely because a path or a previously successful assessment was supplied.

Before writes are permitted, the implementation must:

1. discover and record current target identity;
2. positively establish removable-media eligibility using sufficient evidence;
3. reject known mounted filesystems, root, boot, active swap, system-disk, or contradictory storage topology;
4. reject read-only or undersized media;
5. show stable/current identity, geometry, topology, and capacity to the user;
6. require explicit destructive authorization;
7. re-read and compare target identity and safety-relevant topology/active-use state immediately before writes;
8. abort on target disappearance, replacement, identity change, mount/swap/topology change, or contradictory evidence;
9. bind writes to the revalidated target as strongly as platform APIs reasonably permit;
10. perform interruption-aware partition/boot updates where practical; and
11. preserve `GCDATA` during routine boot-runtime update and repair operations.

The current Linux revalidation token and target assessment are early controls only and are not sufficient destructive authorization.

## 8. Sector and GPT model

The current sector planner requires a logical block size that:

- is at least 512 bytes;
- is a power of two;
- does not exceed 1 MiB; and
- divides the 1 MiB layout alignment exactly.

Device capacity must be divisible by the logical block size.

Current GPT metadata generation includes:

- a protective MBR in logical block 0;
- primary GPT header at LBA 1;
- a 128-entry, 128-byte-per-entry primary partition-entry array beginning at LBA 2;
- an equivalent backup partition-entry array before the final logical block;
- backup GPT header in the final logical block;
- CRC32 over the active partition-entry bytes and GPT header bytes;
- validation that both planned partitions fall within GPT usable LBAs and do not overlap.

Fixed GUIDs are used only by the development test-image helper. Physical provisioning must generate unique disk and partition GUIDs using an approved random/identity source and must never reuse the development constants.

The regular-file GPT test path must create a new file without overwriting an existing path, must not create output beneath protected pseudo-filesystem/device roots such as `/dev`, `/sys`, or `/proc`, and must verify generated metadata by reading it back.

## 9. Catalog model

The first catalog model supports explicit entries with:

- stable entry identifier;
- display name;
- relative asset path;
- boot kind;
- architecture;
- optional SHA-256 metadata.

Current validated boot kinds are schema-level values only. A valid catalog record does not imply that its boot method is implemented.

Catalog paths must be relative and must reject parent traversal. SHA-256 metadata, when present, must be exactly 64 hexadecimal characters.

## 10. Boot compatibility policy

GoreeCloud Boot must not claim universal arbitrary-image compatibility.

Each boot method and image family must have explicit support state and evidence. Planned implementation order is:

1. direct UEFI application launch;
2. selected Linux installation/live image paths;
3. dedicated Windows installation-media support;
4. persistence and stronger recovery workflows;
5. validated Secure Boot, ARM64, optional network boot, and broader hardware support;
6. legacy BIOS only if justified by actual requirements.

None of these boot methods are implemented by the current foundation unless `CAPABILITIES.md` later records verified implementation.

## 11. Technology selection

Rust is the primary language for the native safety-critical foundation because GoreeCloud prefers memory-safe systems languages for low-level and security-sensitive work.

The current Rust workspace intentionally has no external runtime crates. New dependencies require a documented engineering benefit plus provenance, maintenance, security, and licensing review.

C or C++ may be used only when required by firmware, hardware, upstream compatibility, or another documented technical reason. Shell is limited to appropriate build, test, packaging, and operational automation.

## 12. Third-party foundations

Potential bounded foundations include GNU GRUB, EDK II, and optional iPXE. They are not part of the current repository implementation.

Before any third-party component is distributed, the project must record:

- authoritative upstream source;
- exact version or revision;
- license and notices;
- local modifications;
- build provenance;
- source-availability obligations;
- compatibility with the aggregate distribution model;
- update and vulnerability-review process.

GPLv2-only and GPLv3-only code must not be combined into an incompatible derivative work.

## 13. GoreeCloud platform-system responsibilities

The following are required product responsibilities, but documentation must distinguish planned from implemented integration.

### Glaze UI

The future preboot interface must adapt Glaze UI principles to firmware constraints: clear hierarchy, visible focus, keyboard navigation, readable typography, high contrast, restrained motion, and explicit destructive-action language.

### Wardveil Security

Planned responsibilities include release provenance, runtime integrity, image verification, tamper handling, malicious metadata resistance, safe update policy, and trust-state communication. The current native safety controls do not by themselves constitute substantive Wardveil integration.

### Privacy Shield

Core operation must remain local and offline-first. No image names, checksums, device identifiers, boot history, or local configuration may be transmitted merely to use the product. Current discovery and GPT planning are local-only but no substantive Privacy Shield integration contract is claimed yet.

### Everkeep

The product must support reconstructable boot configuration, repair paths, exportable configuration, preserved user data, and documented restoration testing. Current redundant GPT metadata generation is only a development foundation and not an Everkeep repair capability.

### GoreeCloud Mesh

Future Mesh integration may expose host-side lifecycle events, inventory, or management capabilities. Mesh must not become a boot-time dependency.

### GoreeCloud Identity

Identity is not required for the bare offline boot menu. It becomes applicable only to privileged host-side administration, delegated management, centrally managed policy, or authenticated network functions.

## 14. Testing and release qualification

Required validation grows with implementation and includes, where applicable:

- unit tests for safety, discovery/topology/active-swap, catalog, layout, and GPT rules;
- synthetic Linux sysfs/mount/swap fixtures rather than reliance on CI-runner disk topology;
- recursive holder-chain, mounted-topology, active-swap, topology/state-change, and fail-closed discovery cases;
- property/boundary testing for arithmetic and malformed input;
- regular-file destructive metadata tests before block-device writes;
- virtual UEFI testing with QEMU/OVMF;
- physical removable-media testing;
- representative hardware validation;
- per-image-family compatibility records;
- interrupted-update and recovery tests;
- dependency and license review;
- provenance, checksum, and signature validation;
- security review before any physical destructive path is enabled.

A successful source build or unit-test run is not Stable qualification.

## 15. Initial engineering milestone

The first bootable milestone is a non-Stable UEFI x86-64 prototype that can eventually:

- provision a validated removable test device;
- create `GCBOOT` and `GCDATA`;
- present a GoreeCloud Boot preboot menu;
- discover explicitly supported entries;
- directly launch EFI applications;
- boot at least one validated Linux image path;
- verify catalog checksums;
- update or repair `GCBOOT` without erasing `GCDATA`;
- run under QEMU/OVMF before physical-device acceptance.

The current repository has advanced the safety foundation through read-only Linux discovery with bounded recursive holder topology, mounted-filesystem exclusion, active-swap exclusion, topology/active-use-aware revalidation evidence, sector-aware planning, and regular-file GPT metadata generation. Physical provisioning and boot execution remain outside the implemented boundary.