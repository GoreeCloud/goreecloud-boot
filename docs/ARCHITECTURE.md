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
│   ├── target-safety assessment              [implemented]
│   ├── byte/sector layout planning           [implemented]
│   ├── GPT sparse test-image generation      [implemented]
│   ├── destructive authorization             [planned]
│   └── physical provisioning / repair        [planned]
│
├── gcboot-core
│   ├── device safety rules                   [implemented]
│   ├── Linux discovery/revalidation evidence [implemented]
│   ├── partition-layout model                [implemented]
│   ├── GPT metadata generator                [implemented, test-image foundation]
│   ├── catalog metadata validation           [implemented]
│   ├── content hashing/signature policy      [planned]
│   └── compatibility policy                  [planned]
│
└── build/test tooling
    ├── unit tests                             [implemented]
    ├── synthetic Linux discovery fixtures    [implemented]
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
2. **Evidence** — normalized facts about identity, topology, geometry, and state.
3. **Assessment** — conservative policy evaluation that can reject a target.
4. **Selection** — explicit user choice of one discovered candidate.
5. **Revalidation** — fresh comparison of selected identity/topology immediately before a future write.
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
- `/proc/self/mountinfo` for mounted root and boot major/minor identities;
- `/dev/disk/by-id` for available persistent aliases.

A mutable `/dev/sdX` name is display/current-path evidence, not stable identity by itself.

A device with unreadable mandatory discovery metadata is omitted with a warning rather than being treated as eligible from partial assumptions.

### Current topology boundary

Directly enumerated child partitions are associated with their whole device for root/boot rejection. This is not yet sufficient for all Linux storage stacks. Before physical writes, the topology layer must conservatively account for relevant device-mapper, LUKS/encryption, software RAID, multipath, swap, and other relationships that can make a seemingly removable block device part of active system storage.

## Revalidation architecture

The current `LinuxRevalidationToken` snapshots selected evidence including:

- major/minor device number;
- `diskseq` when available;
- capacity;
- logical block size;
- strongest current persistent identity discovered by the implementation;
- serial when exposed.

A future write path must perform a fresh discovery and compare identity plus topology close to the write operation. A matching token is necessary evidence, not sufficient authorization.

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
