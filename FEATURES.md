# Features

This file records GoreeCloud Boot features and their implementation state. It must not be read as a claim that planned functionality is available.

## Implemented development foundation

| Feature | State | Notes |
| --- | --- | --- |
| Rust workspace | Implemented | Dependency-free Rust 1.85 / Edition 2024 workspace for host-side core logic and `bootctl`. |
| Explicit device evidence model | Implemented | Retains the manual development evidence path for safety-policy testing. |
| Read-only Linux block-device discovery | Implemented, development-only | Reads `/sys/block`, `/proc/self/mountinfo`, and available `/dev/disk/by-id` aliases; does not open a block-device node for writing. |
| Linux device identity snapshot | Implemented, development-only | Records major/minor identity, capacity, logical/physical block size, `diskseq` when exposed, and available WWID/serial/`by-id` evidence. |
| Device revalidation token | Implemented, development-only | Compares a later discovered snapshot against selected current identity evidence. Matching is necessary evidence only, not destructive authorization. |
| Root/boot device-family detection | Implemented, bounded | Associates the whole disk and directly enumerated child partitions with mounted `/`, `/boot`, and `/boot/*` major/minor identities. Complex mapper/multipath/RAID/encryption/swap topology is not yet comprehensively modeled. |
| Conservative target-safety assessment | Implemented | Rejects non-removable, read-only, undersized, root-mounted, or boot-mounted targets. This is not destructive-operation authorization. |
| GCBOOT/GCDATA byte layout calculation | Implemented | Calculates an aligned 512 MiB `GCBOOT` region and remaining `GCDATA` space without writing a partition table. |
| Sector-aware layout calculation | Implemented | Converts planned byte boundaries to checked logical-block ranges for supported logical block sizes including 512 and 4096 bytes. |
| Protective MBR/GPT metadata generation | Implemented, test-image only | Generates primary/backup GPT headers and entry arrays with CRC32 plus `GCBOOT`/`GCDATA` partition records in memory. |
| GPT sparse regular-file image | Implemented, development-only | `bootctl create-test-gpt-image` creates a new no-overwrite sparse regular file, writes generated GPT metadata, syncs it, and reads the metadata back for verification. It refuses output beneath `/dev`, `/sys`, or `/proc`. |
| Catalog-entry validation | Implemented | Validates entry identifiers, relative non-traversing paths, supported schema values, and optional SHA-256 formatting. |
| `bootctl plan-device` | Implemented, development-only | Evaluates explicit supplied evidence and prints a proposed byte layout. Does not inspect or modify the device. |
| `bootctl list-linux-devices` | Implemented, development-only | Prints discovered Linux whole-device metadata and current safety assessment. |
| `bootctl plan-linux-device` | Implemented, development-only | Selects a discovered device by current device node or discovered `by-id` alias and prints a sector-aware plan. No physical write occurs. |
| Unit/synthetic-fixture tests | Implemented | Covers current catalog, target safety, Linux discovery, byte/sector layout, CRC32, protective MBR, GPT headers/entries, and 512/4096-byte logical-block cases. |

## Planned boot and provisioning features

| Feature | State | Notes |
| --- | --- | --- |
| Complete Linux destructive-target topology analysis | Planned | Must add conservative handling for device mapper, encrypted mappings, software RAID, multipath, swap, and other system-storage relationships before physical writes. |
| Production-unique GPT identities | Planned | Physical provisioning must generate unique disk and partition GUIDs; fixed development test GUIDs are not acceptable. |
| Destructive authorization workflow | Planned | Must require explicit confirmation and immediate pre-write revalidation. |
| Physical GPT/protective-MBR provisioning | Planned | Must remain disabled until topology, authorization, revalidation, integration-test, interruption/recovery, and physical-device controls are complete. |
| FAT32 `GCBOOT` creation | Planned | Boot-system filesystem. |
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

Current repository code does not yet claim substantive Glaze UI, Wardveil Security, Privacy Shield, Everkeep, GoreeCloud Mesh, or GoreeCloud Identity integration. The native safety and privacy properties in this foundation support future platform integration but are not substitutes for implemented platform contracts, controls, or validated integration evidence.
