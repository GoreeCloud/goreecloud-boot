# GoreeCloud Boot Specifications

## 1. Repository state

- **Product:** GoreeCloud Boot
- **Development model:** Original GoreeCloud-native software
- **Lifecycle:** Development foundation; not Stable or production-approved
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

The default layout is planned as GPT with a protective MBR where compatible.

### GCBOOT

- purpose: boot-system partition;
- planned filesystem: FAT32;
- current planned size: 512 MiB;
- contains future UEFI boot chain, runtime, trusted configuration, verification material, and recovery metadata;
- must be repairable or replaceable without erasing `GCDATA` where practical.

### GCDATA

- purpose: large user-facing data partition;
- planned filesystem: exFAT, subject to validation before release;
- occupies remaining usable device space after alignment and GPT-reserve space;
- may later contain supported boot images, checksums, signatures, persistence data, and sidecar metadata.

The current Rust foundation implements layout calculation only. It does not partition devices.

## 5. Current implemented foundation

The repository currently implements only the development foundation:

- deterministic `GCBOOT`/`GCDATA` layout planning;
- conservative target-safety assessment from explicit device evidence;
- initial catalog-entry validation;
- development-only `bootctl plan-device` output;
- unit tests for those rules.

No device discovery, partition-table write, filesystem formatting, bootloader installation, image booting, signature verification, or recovery write path is currently implemented.

## 6. Target-safety requirements

A future destructive provisioning path must not run merely because a path such as `/dev/sdb` was supplied.

Before writes are permitted, the implementation must:

1. discover and record stable target identity;
2. positively establish removable-media eligibility;
3. reject known root, boot, or system-disk targets;
4. reject read-only or undersized media;
5. show the target identity and capacity to the user;
6. require explicit destructive authorization;
7. re-read and compare target identity immediately before writes;
8. abort on target disappearance, replacement, identity change, or contradictory evidence;
9. perform interruption-aware partition/boot updates where practical; and
10. preserve `GCDATA` during routine boot-runtime update and repair operations.

The current target evaluator is only an early guardrail and is not sufficient authorization for destructive writes.

## 7. Catalog model

The first catalog model supports explicit entries with:

- stable entry identifier;
- display name;
- relative asset path;
- boot kind;
- architecture;
- optional SHA-256 metadata.

Current validated boot kinds are schema-level values only. A valid catalog record does not imply that its boot method is implemented.

Catalog paths must be relative and must reject parent traversal. SHA-256 metadata, when present, must be exactly 64 hexadecimal characters.

## 8. Boot compatibility policy

GoreeCloud Boot must not claim universal arbitrary-image compatibility.

Each boot method and image family must have explicit support state and evidence. Planned implementation order is:

1. direct UEFI application launch;
2. selected Linux installation/live image paths;
3. dedicated Windows installation-media support;
4. persistence and stronger recovery workflows;
5. validated Secure Boot, ARM64, optional network boot, and broader hardware support;
6. legacy BIOS only if justified by actual requirements.

None of these boot methods are implemented by the current foundation unless `CAPABILITIES.md` later records verified implementation.

## 9. Technology selection

Rust is the primary language for the native safety-critical foundation because GoreeCloud prefers memory-safe systems languages for low-level and security-sensitive work.

C or C++ may be used only when required by firmware, hardware, upstream compatibility, or another documented technical reason. Shell is limited to appropriate build, test, packaging, and operational automation.

## 10. Third-party foundations

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

## 11. GoreeCloud platform-system responsibilities

The following are required product responsibilities, but documentation must distinguish planned from implemented integration.

### Glaze UI

The future preboot interface must adapt Glaze UI principles to firmware constraints: clear hierarchy, visible focus, keyboard navigation, readable typography, high contrast, restrained motion, and explicit destructive-action language.

### Wardveil Security

Planned responsibilities include release provenance, runtime integrity, image verification, tamper handling, malicious metadata resistance, safe update policy, and trust-state communication.

### Privacy Shield

Core operation must remain local and offline-first. No image names, checksums, device identifiers, boot history, or local configuration may be transmitted merely to use the product.

### Everkeep

The product must support reconstructable boot configuration, repair paths, exportable configuration, preserved user data, and documented restoration testing.

### GoreeCloud Mesh

Future Mesh integration may expose host-side lifecycle events, inventory, or management capabilities. Mesh must not become a boot-time dependency.

### GoreeCloud Identity

Identity is not required for the bare offline boot menu. It becomes applicable only to privileged host-side administration, delegated management, centrally managed policy, or authenticated network functions.

## 12. Testing and release qualification

Required validation will grow with implementation and may include:

- unit tests for safety, catalog, and layout rules;
- property/boundary testing for arithmetic and malformed input;
- virtual UEFI testing with QEMU/OVMF;
- physical removable-media testing;
- representative hardware validation;
- per-image-family compatibility records;
- interrupted-update and recovery tests;
- dependency and license review;
- provenance, checksum, and signature validation;
- security review before any destructive path is enabled.

A successful source build is not Stable qualification.

## 13. Initial engineering milestone

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

The current repository has begun only the safety/catalog/layout foundation for this milestone.
