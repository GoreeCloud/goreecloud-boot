# Device Layout Contract

## Status

This is the current **planning contract** for the initial GoreeCloud Boot device layout. The Rust code can calculate this layout, but the repository does not yet write it to physical media.

## Units and interval model

- 1 MiB = 1,048,576 bytes.
- 1 GiB = 1,073,741,824 bytes.
- Partition byte ranges use half-open intervals: start is inclusive and end is exclusive.
- The planner aligns major boundaries to 1 MiB.

## Minimum planned device capacity

The current planner requires at least **8 GiB**.

This is a development policy floor, not a final product capacity recommendation. It may change after real filesystem, GPT, image, and update testing.

## Initial layout

```text
byte 0
│
├── initial reserved/alignment region: 1 MiB
│
├── GCBOOT
│   ├── start: 1 MiB
│   ├── size: 512 MiB
│   ├── intended filesystem: FAT32
│   └── intended role: boot-system/runtime/recovery metadata
│
├── alignment to next 1 MiB boundary
│
├── GCDATA
│   ├── start: aligned after GCBOOT
│   ├── size: remaining usable bytes
│   ├── intended filesystem: exFAT, pending validation
│   └── intended role: user boot assets and related metadata
│
└── final reserved region: 1 MiB
    intended to leave space for future partition-table/alignment requirements
```

## Required invariants

A valid planned layout must satisfy all of the following:

- `GCBOOT.start < GCBOOT.end`;
- `GCDATA.start < GCDATA.end`;
- `GCBOOT.end <= GCDATA.start`;
- neither partition extends beyond the supplied device capacity;
- arithmetic must fail on overflow rather than wrap;
- a device below the minimum planning size is rejected;
- `GCDATA` must retain non-zero usable capacity after reserves and alignment.

## Filesystem boundary

The current source records intended filesystem identities only. It does not create FAT32 or exFAT filesystems and therefore must not be used as evidence that filesystem compatibility is complete.

## GPT and protective MBR boundary

GPT with a protective MBR is the planned default where compatible. The current planner operates in bytes and does not yet produce sector-level GPT structures.

Before physical partitioning is implemented, the design must additionally specify and test:

- logical/physical sector sizes;
- GPT header and entry-array placement;
- backup GPT placement;
- partition type GUIDs;
- partition unique GUID generation;
- protective MBR behavior;
- alignment on representative removable media;
- recovery after interrupted table updates.

## Update invariant

Routine GoreeCloud Boot runtime updates and repair are intended to target `GCBOOT` without erasing or recreating `GCDATA` unless the user explicitly invokes a full destructive reprovisioning workflow.
