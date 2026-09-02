# Capabilities

## Overview

GoreeCloud Boot is currently a **development foundation**, not a bootable multiboot product release. The current verified capabilities are limited to host-side safety, layout-planning, catalog-validation, and development tooling implemented in this repository.

## Core Capabilities

- Calculates a deterministic proposed removable-media layout with:
  - 1 MiB initial alignment;
  - a 512 MiB `GCBOOT` FAT32-designated region;
  - an aligned `GCDATA` remainder;
  - reserved end space for future partition-table requirements.
- Assesses explicit device evidence conservatively and rejects targets that are:
  - non-removable;
  - read-only;
  - below the minimum supported planning size;
  - known to contain the mounted root filesystem;
  - known to contain the mounted boot filesystem.
- Validates initial catalog-entry metadata, including safe relative paths and optional SHA-256 string formatting.

These capabilities operate on supplied metadata only. They do not currently inspect or write block devices.

## User Capabilities

A developer can run `bootctl plan-device` with explicit target evidence and receive:

- the target-safety decision;
- rejection reasons, if any;
- a proposed `GCBOOT`/`GCDATA` layout when the supplied evidence is eligible.

The command performs no destructive operation.

## Administrative Capabilities

None currently. There is no production management interface, device inventory, privileged write path, policy service, account system, or centrally managed trust store.

## Platform Integrations

### Glaze UI

**Not implemented.** Firmware-adapted Glaze UI requirements are documented for the future preboot interface.

### Wardveil Security

**Not implemented as a platform integration.** The current repository does contain conservative target-safety rules, but it does not yet provide Wardveil release provenance, cryptographic image verification, signed trust policy, or tamper response.

### Privacy Shield

**Not implemented as a platform integration.** Current code has no network or telemetry capability and therefore does not transmit device or image information, but no Privacy Shield contract or control interface is implemented yet.

### Everkeep

**Not implemented.** No device repair, backup, restore, or reconstruction workflow exists yet.

### GoreeCloud Mesh

**Not implemented.** There is no Mesh capability or runtime dependency.

### GoreeCloud Identity

**Not implemented and not currently required for the offline development foundation.** No authentication or delegated administration capability exists.

## Data and Interoperability

- Catalog paths are represented as relative paths.
- Optional SHA-256 metadata is validated syntactically.
- Current code uses only Rust standard-library functionality and introduces no external runtime data service.

No ISO, IMG, EFI, WIM, VHD, VHDX, exFAT, FAT32, GRUB, EDK II, or iPXE interoperability is implemented yet.

## Supported Platforms and Interfaces

- Source/build interface: Rust/Cargo.
- Development CLI: `bootctl`.
- Initial implementation assumptions: Linux host-side development and UEFI x86-64 as the future first firmware target.

No physical hardware platform is production-validated.

## Security and Privacy Capabilities

Current implemented safeguards are limited to:

- conservative explicit-evidence target rejection;
- arithmetic bounds checks in layout planning;
- traversal-resistant catalog-path validation;
- strict optional SHA-256 metadata formatting;
- absence of destructive block-device writes;
- absence of telemetry or network code.

These safeguards are development controls, not a complete security certification.

## Resilience, Backup, and Recovery Capabilities

None currently. The layout model reserves a separate boot-system and data boundary to support future non-destructive repair, but no repair or recovery operation is implemented.

## Accessibility Capabilities

No user interface is implemented yet. Accessibility requirements for the future preboot interface remain planned.

## Automation and API Capabilities

- Cargo tests validate current core rules.
- The repository includes CI configuration for formatting, linting, and tests.
- No external API is exposed.

## Current Limitations

The repository cannot currently:

- discover actual removable block devices;
- partition or format media;
- install a bootloader;
- create a bootable USB;
- boot any ISO, IMG, EFI application, Windows media, or Linux image;
- verify file contents cryptographically;
- provide Secure Boot;
- provide persistence;
- provide network boot;
- provide ARM64 or legacy BIOS support;
- provide a graphical preboot or host-side management interface;
- provide production recovery or repair.

## Capability Validation

Current capabilities are validated by Rust unit tests and source review only. No runtime QEMU/OVMF boot evidence, physical removable-media provisioning evidence, hardware compatibility evidence, Stable release qualification, or production acceptance exists yet.
