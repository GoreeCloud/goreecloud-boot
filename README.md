# GoreeCloud Boot

GoreeCloud Boot is a native open-source multiboot USB platform under active development for booting, installing, recovering, diagnosing, and maintaining operating systems and GoreeCloud environments from removable media.

> **Development status:** Foundation only. This repository does **not** yet provide a bootable USB release, production provisioning, Secure Boot support, Windows installation-media support, persistence, network boot, ARM64 support, or legacy BIOS support.

## Current repository capabilities

The current foundation provides:

- a dependency-free Rust workspace for the initial host-side safety and catalog logic;
- deterministic planning for the future `GCBOOT` and `GCDATA` partition layout;
- conservative removable-target safety assessment that rejects known system/root/boot disks, non-removable devices, read-only devices, and undersized media;
- validation for initial GoreeCloud Boot catalog-entry identifiers, relative image paths, architecture, boot kind, and optional SHA-256 metadata;
- a development-only `bootctl` CLI that evaluates supplied device evidence and prints a proposed layout without writing to a disk;
- unit tests for the implemented safety, layout, and catalog rules.

See [CAPABILITIES.md](CAPABILITIES.md) for the canonical current-state capability inventory.

## Planned architecture

The intended device layout is:

```text
GoreeCloud Boot device
├── GCBOOT   small FAT32 system partition
│   ├── UEFI boot chain
│   ├── GoreeCloud Boot runtime
│   ├── trusted configuration
│   └── recovery metadata
└── GCDATA   large user-facing data partition
    ├── boot images
    ├── checksums and signatures
    ├── persistence data
    └── catalog metadata
```

The initial firmware target is UEFI x86-64. The project will add boot methods only after each method has explicit implementation and validation evidence.

## Native development boundary

GoreeCloud Boot is an original GoreeCloud implementation. Ventoy may be studied as a capability and compatibility reference, but Ventoy source is not part of this repository.

Mature open-source foundations such as GNU GRUB, EDK II, and optional iPXE may be integrated later as bounded components after architecture, provenance, licensing, and build-distribution review. No such third-party boot component is currently vendored or distributed by this repository.

## Safety model

Provisioning removable media is inherently destructive. Wrong-disk prevention is a release-blocking requirement.

The current code deliberately contains **no disk-writing implementation**. The safety evaluator is being established before partitioning code so destructive behavior cannot silently outrun its guardrails.

Future provisioning must, at minimum:

1. positively identify the requested target;
2. reject system/root/boot disks by default;
3. require removable-media evidence;
4. display stable device identity and capacity;
5. require explicit authorization before destructive changes;
6. revalidate device identity immediately before writes; and
7. fail safely if device identity changes.

See [docs/THREAT-MODEL.md](docs/THREAT-MODEL.md).

## Development

Prerequisites:

- Rust 1.85 or newer;
- Cargo;
- a supported Linux development environment for the initial `bootctl` work.

Run the checks:

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
```

Run the development planner with explicit evidence:

```bash
cargo run -p bootctl -- plan-device \
  --device /dev/sdX \
  --size-bytes 68719476736 \
  --removable yes \
  --root-mounted no \
  --boot-mounted no \
  --read-only no
```

This command only evaluates the supplied evidence and prints a layout proposal. It does not inspect or modify `/dev/sdX`.

## Repository documentation

- [SPECIFICATIONS.md](SPECIFICATIONS.md) — repository-local specification and requirements
- [FEATURES.md](FEATURES.md) — feature inventory and implementation state
- [CAPABILITIES.md](CAPABILITIES.md) — verified current capability inventory
- [BENEFITS.md](BENEFITS.md) — supportable product benefits and design value
- [COMPETITIVE-OBJECTIVES.md](COMPETITIVE-OBJECTIVES.md) — differentiation and benchmark objectives
- [BRANDING.md](BRANDING.md) — product identity and artwork boundary
- [SECURITY.md](SECURITY.md) — security posture and reporting guidance
- [THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md) — dependency and provenance status
- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) — component architecture
- [docs/DEVICE-LAYOUT.md](docs/DEVICE-LAYOUT.md) — partition-layout contract
- [docs/THREAT-MODEL.md](docs/THREAT-MODEL.md) — destructive-operation threat model

## License

GoreeCloud-owned source in this repository is licensed under the GNU General Public License version 3. See [LICENSE](LICENSE).

Third-party components, when introduced, retain their own licenses and must be tracked separately in the dependency/provenance documentation.
