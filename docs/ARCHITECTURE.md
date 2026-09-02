# Architecture

## Status

This document describes the current native foundation plus intended component boundaries. Only components identified as implemented in `CAPABILITIES.md` are current capabilities.

## Component model

```text
Host development / administration
│
├── bootctl
│   ├── explicit development evidence input   [implemented]
│   ├── Linux read-only device discovery      [implemented]
│   ├── bounded topology/mount/swap assessment [implemented]
│   ├── target-safety assessment              [implemented]
│   ├── byte/sector layout planning           [implemented]
│   ├── GPT sparse test-image generation      [implemented]
│   ├── destructive authorization             [planned]
│   └── physical provisioning / repair        [planned]
│
├── gcboot-core
│   ├── device safety rules                   [implemented]
│   ├── Linux discovery/revalidation evidence [implemented]
│   ├── recursive holder-topology evidence    [implemented, bounded]
│   ├── active-swap topology evidence         [implemented, bounded]
│   ├── partition-layout model                [implemented]
│   ├── GPT metadata generator                [implemented, test-image foundation]
│   ├── catalog metadata validation           [implemented]
│   ├── content hashing/signature policy      [planned]
│   └── compatibility policy                  [planned]
│
└── build/test tooling
    ├── unit tests                             [implemented]
    ├── synthetic Linux topology/swap fixtures [implemented]
    ├── regular-file GPT metadata tests       [implemented]
    ├── QEMU/OVMF boot harness                [planned]
    └── release provenance/signing            [planned]

Removable device
│
├── GCBOOT                                    [planned physical implementation]
│   ├── FAT32 filesystem                      [planned]
│   ├── UEFI boot runtime                     [planned]
│   ├── boot-method adapters                  [planned]
│   ├── trusted config                        [planned]
│   └── recovery metadata                     [planned]
│
└── GCDATA                                    [planned physical implementation]
    ├── exFAT filesystem                      [planned]
    ├── images
    ├── catalog metadata
    ├── checksums/signatures
    └── persistence data
```

## Native ownership boundary

GoreeCloud owns and maintains product-specific orchestration, device-safety policy, catalog contract, provisioning behavior, update/recovery policy, compatibility state, interface behavior, tests, release process, and GoreeCloud platform-system integration.

Mature open-source boot or firmware projects may later be used behind explicit boundaries. They must not silently become the architectural identity of GoreeCloud Boot.

## Host-side language and dependency boundary

The current core and CLI use Rust because device discovery, provisioning geometry, metadata parsing, arithmetic, and future low-level operations are security-sensitive and benefit from memory-safe native code.

The workspace currently uses no external Rust runtime crates. This keeps the first safety/discovery/GPT layer independently inspectable while the project establishes its dependency-review process. A future dependency must provide enough value to justify provenance, security, maintenance, and licensing cost.

## Safety-state architecture

The safety design separates these concepts:

1. **Discovery** — read current platform evidence about candidate devices.
2. **Evidence** — normalized facts about identity, topology, geometry, mounts, swap, and state.
3. **Assessment** — conservative policy evaluation that can reject a target.
4. **Selection** — explicit user choice of one discovered candidate.
5. **Revalidation** — fresh comparison of selected identity/topology/active-use state immediately before a future write.
6. **Authorization** — a future destructive-operation decision requiring explicit consent after successful revalidation.

The current code implements discovery, evidence, assessment, selection for planning, and a revalidation-token data model. It does **not** implement destructive authorization or physical writes.

No earlier state may be treated as equivalent to authorization.

## Linux discovery architecture

The Linux discovery layer deliberately reads metadata rather than opening block-device nodes.

Current sources include:

- `/sys/block/<name>/dev` for whole-device major/minor identity;
- `/sys/block/<name>/size` for kernel-exported capacity units;
- `/sys/block/<name>/queue/logical_block_size` and `physical_block_size` for geometry;
- `/sys/block/<name>/removable` and `ro` for current flags;
- `/sys/block/<name>/diskseq` when exposed for current drive-instance identity;
- device vendor/model/serial/WWID attributes when exposed;
- direct child-partition `dev` identities;
- recursive sysfs `holders` links from the whole device and each direct partition to discover upward stacked-device dependencies;
- `/proc/self/mountinfo` for all currently mounted major/minor identities, mount points, and explicit root/boot state;
- `/proc/swaps` for current active swap areas;
- `/dev/disk/by-id` for available persistent aliases.

A mutable `/dev/sdX` name is display/current-path evidence, not stable identity by itself.

A device with unreadable mandatory per-device discovery or holder-topology metadata is omitted with a warning rather than being treated as eligible from partial assumptions. Global active-swap evidence is stricter: if `/proc/swaps` cannot be read or an active swap entry cannot be parsed/resolved safely, Linux discovery fails rather than returning candidates based on incomplete system-storage evidence.

### Current topology and active-use boundary

For each candidate whole device, the implementation starts with the disk and its directly enumerated partitions, then recursively follows sysfs `holders` relationships upward. The resulting major/minor topology is intersected with every major/minor identity reported by mountinfo. If any intersection exists, the candidate is rejected; mounted root and boot remain separately identified for clearer safety evidence.

Active swap is resolved separately. Swap partitions from `/proc/swaps` are canonicalized and mapped to their sysfs major/minor identity. Swap files are canonicalized and associated with the deepest mountinfo mount point that contains them. The resulting active-swap device set is intersected with the candidate topology, and any intersection rejects the candidate.

This catches important active stacked-device cases and is covered by synthetic tests including a recursive partition → device-mapper-style holder → RAID-style holder chain, direct and holder-device swap, swap-file backing storage, and swap-state revalidation changes. It is still a **bounded development topology/state model**, not complete destructive-target qualification.

Device-mapper, encryption, software RAID, multipath, hotplug/reconfiguration, mount/swap namespaces, unusual swap/storage configurations, and other Linux storage relationships require additional validation and possibly additional evidence before physical writes can be enabled. Passing the current topology/active-use assessment does not mean those cases have been exhaustively ruled out.

## Revalidation architecture

The current `LinuxRevalidationToken` snapshots selected evidence including:

- major/minor device number;
- `diskseq` when available;
- capacity;
- logical block size;
- current removable and read-only state;
- strongest current persistent identity discovered by the implementation;
- serial when exposed;
- sorted major/minor identities in the discovered candidate topology;
- sorted major/minor identities where that topology currently intersects mountinfo;
- sorted major/minor identities where that topology currently intersects active swap.

Because topology, mounted-topology state, and active-swap topology state are part of the token, a relevant holder, mount, or swap-state change causes token comparison to fail. Synthetic tests verify mount- and swap-state invalidation.

A future write path must still perform a fresh discovery and compare identity plus all required safety state close to the write operation. A matching token is necessary evidence, not sufficient authorization.

## Layout architecture

The byte planner models half-open byte intervals and reserves 1 MiB alignment regions. The sector planner converts those boundaries to inclusive logical-block partition ranges only after validating logical-block geometry and capacity alignment.

The current supported planning geometry includes conventional 512-byte and 4096-byte logical blocks. The planner rejects unsupported block sizes and arithmetic/geometry conflicts instead of rounding silently.

## GPT metadata architecture

The current GPT generator creates metadata buffers and offsets only. It does not open a block device.

The generated structure includes:

- protective MBR;
- primary GPT header at LBA 1;
- primary partition-entry array beginning at LBA 2;
- backup partition-entry array before the final logical block;
- backup GPT header in the final logical block;
- CRC32 for partition-entry contents and each GPT header;
- EFI System Partition type for `GCBOOT`;
- Microsoft Basic Data type for `GCDATA`.

The development test identity uses fixed GUIDs only to make tests reproducible. Physical provisioning must replace those identities with unique generated GUIDs.

## Regular-file test boundary

`bootctl create-test-gpt-image` is the only current write-capable command, and its write target is deliberately restricted to a **new regular sparse file**:

- the output is created with no-overwrite semantics;
- existing paths are rejected;
- output under `/dev`, `/sys`, or `/proc` is rejected;
- generated GPT metadata is written at planned offsets;
- the file is synchronized;
- the metadata is read back and compared to the generated buffers;
- a failed write/verification attempt removes the incomplete output when possible.

This test path is not physical provisioning and is not a bootable-image builder. It creates no FAT32/exFAT filesystem and installs no UEFI runtime.

## Catalog architecture

Catalog metadata remains separate from the boot asset by default. The first schema validates safe identifiers and relative paths so a catalog entry cannot use obvious parent traversal to escape its expected data root.

A syntactically valid catalog entry does not mean its image type is boot-supported. Compatibility state and boot-method implementation remain separate concerns.

## Future firmware runtime

The first firmware target is UEFI x86-64. The runtime should ultimately:

- locate the GoreeCloud Boot device;
- load trusted configuration;
- enumerate explicitly supported catalog entries;
- display verification/support state;
- dispatch only through implemented boot methods;
- provide recovery/diagnostic information when a method fails.

The firmware runtime must remain bootable without GoreeCloud Mesh, GoreeCloud Identity, or a network connection.

## Update architecture

Future routine updates should stage and verify replacement `GCBOOT` content before activation where practical. `GCDATA` is a separate persistence boundary and must not be erased merely to update the boot runtime.

## Platform systems

The architecture reserves substantive responsibility for:

- **Glaze UI:** firmware-appropriate interface and accessibility behavior;
- **Wardveil Security:** provenance, integrity, verification, trust state, safe update and response;
- **Privacy Shield:** offline-first behavior and data minimization;
- **Everkeep:** reconstruction, repair, preservation, and restore validation;
- **GoreeCloud Mesh:** optional host-side coordination, never a boot-time dependency;
- **GoreeCloud Identity:** future privileged administration where applicable, not a bare boot-menu dependency.

Current native safety/privacy properties are implementation foundations only; they do not constitute completed integration with these platform systems.