# Features

This file records GoreeCloud Boot features and their implementation state. It must not be read as a claim that planned functionality is available.

## Implemented development foundation

| Feature | State | Notes |
| --- | --- | --- |
| Rust workspace | Implemented | Dependency-free Rust 1.85 / Edition 2024 workspace for host-side core logic and `bootctl`. |
| Explicit device evidence model | Implemented | Retains the manual development evidence path for safety-policy testing, including general mounted-filesystem and active-swap evidence. |
| Read-only Linux block-device discovery | Implemented, development-only | Reads `/sys/block`, `/proc/self/mountinfo`, `/proc/swaps`, and available `/dev/disk/by-id` aliases; does not open a block-device node for writing. |
| Linux device identity snapshot | Implemented, development-only | Records major/minor identity, capacity, logical/physical block size, `diskseq` when exposed, and available WWID/serial/`by-id` evidence. |
| Bidirectional Linux holder/slave topology closure | Implemented, bounded | Starting from a whole device and directly enumerated partitions, recursively follows both sysfs `holders` and `slaves` links. Canonical-path deduplication prevents reciprocal cycles, and shared holders can pull sibling backing members into the same safety topology. This is not exhaustive destructive-target qualification. |
| Mounted topology detection | Implemented, bounded | Intersects the discovered device topology with all major/minor identities in `/proc/self/mountinfo`; retains explicit root and boot evidence and rejects any discovered topology with a mounted filesystem, including a mounted slave-connected peer. |
| Active swap topology detection | Implemented, bounded | Parses `/proc/swaps`; resolves swap partitions through sysfs and swap files through the deepest containing mountinfo filesystem; rejects a candidate when resolved active-swap identity intersects its discovered topology, including a slave-connected peer. Global swap evidence fails closed when unreadable, malformed, unsupported, or unresolvable. |
| Device revalidation token | Implemented, development-only | Compares a later discovered snapshot against selected current identity, removable/read-only state, topology, mounted-topology, and active-swap-topology evidence. A new slave-connected topology member invalidates the token. Matching is necessary evidence only, not destructive authorization. |
| Conservative target-safety assessment | Implemented | Rejects non-removable, read-only, undersized, filesystem-mounted, or active-swap target evidence, with specific root/boot reasons when applicable. This is not destructive-operation authorization. |
| Fail-closed per-device discovery | Implemented, development-only | A candidate with unreadable mandatory device or required `holders`/`slaves` topology evidence is omitted with a warning instead of being treated as eligible from incomplete evidence. |
| Fail-closed global swap discovery | Implemented, development-only | Linux discovery aborts if required active-swap state cannot be read or safely resolved instead of producing target eligibility from incomplete host-storage evidence. |
| GCBOOT/GCDATA byte layout calculation | Implemented | Calculates an aligned 512 MiB `GCBOOT` region and remaining `GCDATA` space without writing a partition table. |
| Sector-aware layout calculation | Implemented | Converts planned byte boundaries to checked logical-block ranges for supported logical block sizes including 512 and 4096 bytes. |
| Protective MBR/GPT metadata generation | Implemented, test-image only | Generates primary/backup GPT headers and entry arrays with CRC32 plus `GCBOOT`/`GCDATA` partition records in memory. |
| GPT sparse regular-file image | Implemented, development-only | `bootctl create-test-gpt-image` creates a new no-overwrite sparse regular file, writes generated GPT metadata, syncs it, and reads the metadata back for verification. It refuses output beneath `/dev`, `/sys`, or `/proc`. |
| Catalog-entry validation | Implemented | Validates entry identifiers, relative non-traversing paths, supported schema values, and optional SHA-256 formatting. |
| `bootctl plan-device` | Implemented, development-only | Evaluates explicit supplied evidence, including general mounted-filesystem and active-swap evidence, and prints a proposed byte layout. Does not inspect or modify the device. |
| `bootctl list-linux-devices` | Implemented, development-only | Prints discovered Linux whole-device metadata, bounded bidirectional topology/mount/swap evidence, and current safety assessment. |
| `bootctl plan-linux-device` | Implemented, development-only | Selects a discovered device by current device node or discovered `by-id` alias and prints a sector-aware plan. No physical write occurs. |
| Unit/synthetic-fixture tests | Implemented | Covers current catalog, target safety, Linux discovery/topology, mounted direct partitions, recursive holder chains, reciprocal holder/slave cycles, mounted and active-swap sibling backing members, fail-closed swap resolution, revalidation-token mount/swap/topology changes, byte/sector layout, CRC32, protective MBR, GPT headers/entries, and 512/4096-byte logical-block cases. |

## Planned boot and provisioning features

| Feature | State | Notes |
| --- | --- | --- |
| Complete Linux destructive-target topology qualification | Planned | Must validate and, where necessary, extend conservative handling for device mapper, encrypted mappings, software RAID, multipath, hotplug/reconfiguration, mount/swap namespaces, unusual storage configurations, and other system-storage relationships before physical writes. Bidirectional `holders`/`slaves` closure plus mounted-filesystem and active-swap exclusion are implemented layers, not complete qualification. |
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