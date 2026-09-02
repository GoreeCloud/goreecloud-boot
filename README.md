# GoreeCloud Boot

GoreeCloud Boot is a native open-source multiboot USB platform under active development for booting, installing, recovering, diagnosing, and maintaining operating systems and GoreeCloud environments from removable media.

> **Development status:** Foundation only. This repository does **not** yet provide physical USB provisioning, a bootable USB release, filesystem creation, a UEFI runtime, Secure Boot, Windows installation-media support, persistence, network boot, ARM64 support, or legacy BIOS support.

## Current repository capabilities

The current foundation provides:

- a dependency-free Rust workspace for host-side safety, discovery, layout, GPT metadata, catalog logic, and `bootctl`;
- read-only Linux whole-block-device discovery from sysfs, visible mount-namespace metadata, active-swap metadata, and persistent `by-id` aliases where available;
- bounded bidirectional Linux storage-topology closure through sysfs `holders` and `slaves` links, starting from each whole device and its directly enumerated partitions;
- visible mount-namespace discovery that unions mount evidence from distinct readable namespaces exposed through the caller-visible `/proc`, records namespace identities, and makes every candidate ineligible when visible namespace coverage is incomplete;
- conservative target assessment that rejects non-removable, read-only, undersized, currently mounted, active-swap, or incomplete-visible-namespace-evidence device topologies, including related backing members discovered through shared stacked devices;
- device revalidation tokens incorporating current Linux identity, removable/read-only state, discovered topology, mounted-topology intersection, active-swap topology intersection, visible mount-namespace identities/coverage, `diskseq`, capacity, logical block size, and available persistent identity data;
- deterministic byte and sector planning for the future `GCBOOT` and `GCDATA` layout;
- in-memory protective-MBR and redundant GPT metadata generation for 512-byte and 4096-byte logical-block test geometries;
- a development command that writes generated GPT metadata only to a **new sparse regular file**, then reads it back for verification;
- validation for initial catalog-entry identifiers, relative image paths, architecture, boot kind, and optional SHA-256 metadata;
- unit and synthetic-fixture tests for the implemented safety, Linux discovery/topology/mount-namespace/swap handling, layout, GPT, and catalog rules.

See [CAPABILITIES.md](CAPABILITIES.md) for the canonical current-state capability inventory.

## Planned architecture

The intended physical device layout remains:

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

The initial firmware target is UEFI x86-64. Boot methods will be added only after each method has explicit implementation and validation evidence.

## Native development boundary

GoreeCloud Boot is an original GoreeCloud implementation. Ventoy may be studied as a capability and compatibility reference, but Ventoy source is not part of this repository.

Mature open-source foundations such as GNU GRUB, EDK II, and optional iPXE may be integrated later as bounded components after architecture, provenance, licensing, and build-distribution review. No such third-party boot component is currently vendored or distributed by this repository.

## Safety model

Provisioning removable media is inherently destructive. Wrong-disk prevention is a release-blocking requirement.

The current code deliberately contains **no physical block-device write implementation**. Linux discovery reads metadata only. For each candidate it starts with the whole device and direct partitions, recursively follows both sysfs `holders` and `slaves` links, canonicalizes each discovered node, and uses cycle protection to form a bounded connected topology. This allows a shared holder such as an array or mapper-style device to bring its other backing members into the same safety evidence. Any non-`NotFound` failure while reading a required `holders` or `slaves` relation causes the affected candidate to be omitted rather than assumed safe.

Mount safety no longer assumes `/proc/self/mountinfo` represents every visible mount. Discovery reads the current namespace and enumerates caller-visible numeric process entries under `/proc`, deduplicates their mount namespaces by `/proc/<pid>/ns/mnt` identity, and unions mount evidence from each distinct namespace whose `/proc/<pid>/mountinfo` can be read and parsed. A mount in another visible namespace can therefore reject a candidate. If a visible namespace cannot be safely inspected, discovery retains read-only inspection output but marks namespace coverage incomplete, causing every candidate assessment to be ineligible. This is deliberately fail-conservative; it does not prove that namespaces hidden from or inaccessible through the caller’s `/proc` do not exist.

The implementation also reads `/proc/swaps`, resolves active swap partitions to sysfs major/minor identities, and resolves active swap files against the deepest containing mount information. If visible namespace evidence gives an active swap file ambiguous deepest backing filesystems, Linux discovery fails closed. A candidate is rejected when resolved active swap intersects its discovered topology. Unreadable, malformed, unsupported, or unresolvable global active-swap evidence also fails Linux discovery rather than permitting targets from incomplete safety evidence.

These controls are still a bounded development model. Device-mapper, encryption, software RAID, multipath, hotplug/reconfiguration, namespace visibility constraints, unusual storage configurations, and other Linux storage relationships are not yet exhaustively qualified for destructive use.

The GPT development path writes only to a newly created regular sparse file and refuses existing output paths and output beneath `/dev`, `/sys`, or `/proc`.

Future physical provisioning must, at minimum:

1. discover the requested target from platform evidence rather than trusting a mutable path alone;
2. reject active system-storage relationships, including mounted filesystems and swap, by default;
3. require removable-media evidence;
4. display stable identity, current-instance identity, geometry, topology, active-use and namespace-coverage evidence, and capacity;
5. require explicit destructive authorization;
6. revalidate device identity and all required topology/active-use/namespace evidence immediately before writes;
7. fail safely if device identity, topology, namespace visibility, mount state, swap state, or other critical safety evidence changes; and
8. complete image-backed destructive integration tests before physical write support is enabled.

See [docs/THREAT-MODEL.md](docs/THREAT-MODEL.md).

## Development

Prerequisites:

- Rust 1.85 or newer;
- Cargo;
- Linux for the current host-device discovery commands.

Run the checks:

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
```

### Read Linux device metadata

```bash
cargo run -p bootctl -- list-linux-devices
```

This command reads Linux sysfs, caller-visible mount-namespace, mount, and active-swap metadata, including bounded bidirectional `holders`/`slaves` topology. It displays visible mount-namespace identities and whether visible namespace coverage was complete for the probe. It does not open a block-device node for writing.

### Plan a discovered Linux device

```bash
cargo run -p bootctl -- plan-linux-device --device /dev/disk/by-id/EXAMPLE
```

The selected whole device must match the current discovery report by device node or discovered `by-id` alias. Passing assessment still means **planning only**, not write authorization.

### Generate a GPT test image

```bash
cargo run -p bootctl -- create-test-gpt-image \
  --output ./gcboot-gpt-test.img \
  --size-bytes 8589934592 \
  --logical-block-size 512
```

This creates a new sparse regular file with protective-MBR/GPT metadata and verifies the generated metadata by reading it back. It does **not** create FAT32 or exFAT filesystems and is **not** a bootable GoreeCloud Boot image.

### Manual evidence planner

```bash
cargo run -p bootctl -- plan-device \
  --device /dev/sdX \
  --size-bytes 68719476736 \
  --removable yes \
  --root-mounted no \
  --boot-mounted no \
  --filesystem-mounted no \
  --active-swap no \
  --read-only no
```

This legacy development command evaluates only the supplied values and does not inspect or modify `/dev/sdX`.

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
- [docs/DEVICE-LAYOUT.md](docs/DEVICE-LAYOUT.md) — layout/GPT planning contract
- [docs/THREAT-MODEL.md](docs/THREAT-MODEL.md) — destructive-operation threat model

## License

GoreeCloud-owned source in this repository is licensed under **GPL-3.0-or-later**. See [LICENSE](LICENSE) and the SPDX identifiers in source files.

Third-party components, when introduced, retain their own licenses and must be tracked separately in dependency/provenance documentation.