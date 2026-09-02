# Architecture

## Status

This document describes the current native foundation plus the intended component boundaries. Only components identified as implemented in `CAPABILITIES.md` are current capabilities.

## Component model

```text
Host development / administration
│
├── bootctl
│   ├── explicit device-evidence input        [implemented]
│   ├── target-safety assessment              [implemented]
│   ├── layout planning                       [implemented]
│   ├── device discovery                      [planned]
│   ├── destructive authorization             [planned]
│   └── provisioning / repair                  [planned]
│
├── gcboot-core
│   ├── device safety rules                   [implemented]
│   ├── partition-layout model                [implemented]
│   ├── catalog metadata validation           [implemented]
│   ├── hashing/signature policy              [planned]
│   └── compatibility policy                  [planned]
│
└── build/test tooling
    ├── unit tests                             [implemented]
    ├── QEMU/OVMF harness                     [planned]
    └── release provenance/signing             [planned]

Removable device
│
├── GCBOOT                                    [planned physical implementation]
│   ├── UEFI boot runtime                     [planned]
│   ├── boot-method adapters                  [planned]
│   ├── trusted config                        [planned]
│   └── recovery metadata                     [planned]
│
└── GCDATA                                    [planned physical implementation]
    ├── images
    ├── catalog metadata
    ├── checksums/signatures
    └── persistence data
```

## Native ownership boundary

GoreeCloud owns and maintains the product-specific orchestration, device-safety policy, catalog contract, provisioning behavior, update/recovery policy, compatibility state, interface behavior, tests, release process, and GoreeCloud platform-system integration.

Mature open-source boot or firmware projects may later be used behind explicit boundaries. They must not silently become the architectural identity of GoreeCloud Boot.

## Host-side language

The initial core and CLI use Rust because device provisioning, metadata parsing, arithmetic, and future low-level operations are security-sensitive and benefit from memory-safe native code.

The current workspace intentionally avoids external crates until a dependency provides enough value to justify its maintenance, provenance, security, and licensing cost.

## Safety architecture

The safety design separates three concepts:

1. **Evidence** — facts discovered or explicitly supplied about a candidate target.
2. **Assessment** — conservative policy evaluation that can reject a target.
3. **Authorization** — a future destructive-operation decision requiring fresh identity validation and explicit user consent.

The current code implements evidence and assessment only. Assessment success must never be treated as sufficient authorization for a future write operation.

Future device discovery must produce stable identity material independent of mutable Linux path names such as `/dev/sdb` whenever practical.

## Layout architecture

The current planner models byte ranges using half-open intervals (`start` inclusive, `end` exclusive). It reserves alignment space at the beginning and end of the device and returns an error instead of relying on wrapping arithmetic.

Physical GPT-sector conversion, filesystem creation, protective MBR generation, and firmware boot-file placement are deliberately outside the current implementation.

## Catalog architecture

Catalog metadata must remain separate from the boot asset by default. The first schema validates safe identifiers and relative paths so a catalog entry cannot use obvious parent traversal to escape its expected data root.

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
- **GoreeCloud Mesh:** optional host-side coordination, never boot-time dependency;
- **GoreeCloud Identity:** future privileged administration where applicable, not bare boot-menu dependency.
