# Features

This file records GoreeCloud Boot features and their implementation state. It must not be read as a claim that planned functionality is available.

## Implemented development foundation

| Feature | State | Notes |
| --- | --- | --- |
| Rust workspace | Implemented | Dependency-free initial workspace for host-side core logic and `bootctl`. |
| Device evidence model | Implemented | Represents path, capacity, removable state, root/boot mount evidence, and read-only state supplied to the evaluator. |
| Conservative target-safety assessment | Implemented | Rejects non-removable, read-only, undersized, root-mounted, or boot-mounted targets. This is not yet destructive-operation authorization. |
| GCBOOT/GCDATA layout calculation | Implemented | Calculates an aligned 512 MiB `GCBOOT` partition and remaining `GCDATA` space without writing a partition table. |
| Catalog-entry validation | Implemented | Validates entry identifiers, relative non-traversing paths, supported schema values, and optional SHA-256 formatting. |
| `bootctl plan-device` | Implemented, development-only | Evaluates explicit supplied evidence and prints a proposed layout. Does not inspect or modify the device. |
| Unit tests | Implemented | Covers current layout, target-safety, and catalog validation rules. |

## Planned boot and provisioning features

| Feature | State | Notes |
| --- | --- | --- |
| Linux removable-device discovery | Planned | Must use stable device identity and mount/system-disk evidence. |
| Destructive authorization workflow | Planned | Must require explicit confirmation and immediate pre-write revalidation. |
| GPT/protective-MBR provisioning | Planned | Must be interruption-aware and release-blocked on insufficient target safety. |
| FAT32 `GCBOOT` creation | Planned | Boot-system partition. |
| exFAT `GCDATA` creation | Planned | Subject to implementation and compatibility validation. |
| UEFI x86-64 boot runtime | Planned | First firmware target. |
| Glaze UI preboot menu | Planned | Firmware-adapted implementation, not desktop UI parity. |
| EFI application launch | Planned | First boot-method target. |
| Selected Linux image booting | Planned | Requires explicit per-image-family implementation and evidence. |
| Checksum verification | Planned | Catalog metadata exists, but content hashing and boot gating are not yet implemented. |
| Detached-signature verification | Planned | Wardveil Security responsibility; signing model not yet established. |
| Non-destructive GCBOOT repair/update | Planned | Must preserve `GCDATA` by default. |
| Windows installation-media support | Planned | Dedicated implementation; not treated as generic Linux ISO behavior. |
| Persistence | Planned | File or explicitly managed partition model. |
| Secure Boot | Planned | Must be separately implemented, signed, and validated. |
| ARM64 | Planned | Deferred until x86-64 foundation is validated. |
| Optional iPXE network boot | Planned | Must remain optional and license-separated where required. |
| Legacy BIOS | Deferred | Only if actual GoreeCloud requirements justify its maintenance/testing cost. |

## Platform-system features

Current repository code does not yet implement substantive Glaze UI, Wardveil Security, Privacy Shield, Everkeep, GoreeCloud Mesh, or GoreeCloud Identity integration. Their required product responsibilities are documented in `SPECIFICATIONS.md` and will be moved into current capability claims only after implementation and validation evidence exists.
