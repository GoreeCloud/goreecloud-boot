# Threat Model

## Scope

This threat model begins with the highest-risk GoreeCloud Boot behavior: selecting and modifying removable block devices. It expands as Linux discovery/topology/active-use evidence and GPT metadata generation are implemented, while physical block-device writes remain intentionally disabled.

## Assets to protect

- host system disks and partitions;
- user data on non-target storage devices;
- `GCDATA` contents during future routine updates/repair;
- GoreeCloud Boot runtime integrity;
- image/catalog trust metadata;
- release provenance and signing material;
- device identity, topology, and active-use state used for destructive authorization.

## Current trust boundary

The current implementation has two evidence paths:

1. explicit command-line development evidence used by `plan-device`; and
2. read-only Linux discovery using sysfs, mountinfo, `/proc/swaps`, and available persistent aliases.

Neither path authorizes a destructive operation. No current command opens a physical block-device node for writing.

The only current write path creates a **new sparse regular file** for GPT metadata testing. It uses no-overwrite creation semantics and rejects output beneath `/dev`, `/sys`, or `/proc`.

## Primary threats

### T1 — Wrong target selected by mutable device path

A removable device can change from `/dev/sdb` to another path, or a different device can later appear at the same path.

**Current controls:** Linux discovery records major/minor identity, `diskseq` when available, capacity, logical-block size, removable/read-only state, available WWID/serial/`by-id` identity, bounded bidirectional holder/slave topology, mounted-topology intersection, and active-swap-topology intersection. A revalidation token can compare a later snapshot, including holder/slave topology, mount, and swap-state changes.

**Remaining controls before physical writes:** fresh comprehensive topology/active-use-aware discovery and identity comparison immediately before writes; abort on mismatch; bind the write to the revalidated device as strongly as platform APIs permit.

### T2 — System disk incorrectly treated as removable

Hardware, USB bridges, unusual storage, virtual environments, device mapper, encrypted mappings, software RAID, multipath, or incomplete discovery can produce misleading signals.

**Current controls:** whole-device removable/read-only state; direct child-partition identity; recursive bidirectional sysfs `holders`/`slaves` traversal from the candidate disk and partitions; canonical-path cycle protection; closure through shared holders to related backing members; intersection of the resulting topology with every major/minor identity in mountinfo; conservative rejection when any mounted filesystem is found; explicit root/boot evidence; active-swap discovery from `/proc/swaps` with swap-partition and swap-file backing-device resolution; rejection when active swap intersects the candidate topology; fail-closed omission when mandatory per-device topology evidence is unreadable; fail-closed Linux discovery when mandatory global swap evidence cannot be safely resolved.

**Residual risk:** the implemented holder/slave/mount/swap model is bounded. Relevant device-mapper, encryption, software RAID, multipath, hotplug/reconfiguration, mount/swap namespace, unusual storage, and other relationships have not been exhaustively qualified for destructive use.

**Required controls before physical writes:** conservative topology/active-use graph sufficient to reject active host storage dependencies; validation across representative complex storage stacks and namespace configurations; multiple independent signals where appropriate; fail closed on contradictory or incomplete critical evidence.

### T3 — Time-of-check/time-of-use device or topology replacement

The target can disappear and a new device can occupy its path after selection. A holder/slave relationship, mounted state, or active-swap state can also change after assessment.

**Current controls:** `LinuxRevalidationToken` captures selected identity, removable/read-only state, the complete discovered bidirectional topology device set, mounted-topology device numbers, and active-swap-topology device numbers for comparison against a later probe. Synthetic tests verify that mount/swap changes and the addition of a slave-connected topology member invalidate the token.

**Required controls before physical writes:** fresh discovery close to use; compare stable/current-instance identity and capacity/geometry/topology/active-use state; re-open/re-identify immediately before writes; keep the transaction bound to the verified device handle where possible.

### T4 — Arithmetic or geometry error damages media

Overflow, sector-size assumptions, alignment mistakes, or incorrect GPT calculations can generate writes outside intended ranges.

**Current controls:** checked byte arithmetic; supported logical-block validation; device-capacity alignment checks; checked conversion to sector ranges; GPT usable-range/overlap validation; tests for 512-byte and 4096-byte logical blocks; regular-file GPT metadata generation.

**Future controls:** broader property/boundary testing, independent GPT parser/tool validation, end-to-end image-backed provisioning tests, and physical-media tests before production use.

### T5 — GPT metadata is malformed or non-recoverable

Incorrect header fields, CRCs, entry locations, partition type IDs, or missing backup metadata could create an unusable or ambiguous disk.

**Current controls:** primary/backup GPT headers; primary/backup entry arrays; protective MBR; CRC32 test vector; header CRC verification tests; entry-array CRC tests; partition-type/range tests; regular-file read-back verification.

**Remaining controls:** independent standards-conformance parsing, production GUID generation, filesystem integration tests, interruption tests, and physical validation.

### T6 — Test-image command accidentally writes an existing or special target

A development test helper could become a shortcut around the physical-write safety boundary.

**Current controls:** `create_new(true)` no-overwrite creation; parent canonicalization; rejection beneath `/dev`, `/sys`, and `/proc`; write target is a newly created regular path; failed generation/verification attempts remove the incomplete file when practical.

**Residual risk:** this is a development safeguard, not a universal sandbox. Users must still choose an ordinary development path. The command must never be generalized to existing files/device nodes without a separately reviewed destructive design.

### T7 — Interrupted future provisioning leaves ambiguous media

Power loss, unplugging, process termination, or write failure can leave inconsistent partition/boot state.

**Current controls:** regular-file GPT test writes are synchronized and verified; incomplete test output is removed on detected failure.

**Required controls for physical media:** staged/safe ordering where possible, operation/recovery metadata, post-write verification, backup-header validation, documented repair flow, interrupted-operation tests.

### T8 — Routine update erases user data

An update intended for boot-system files could incorrectly reinitialize the whole device.

**Required controls:** separate `GCBOOT` and `GCDATA` identities; update operations target verified boot partition only; full reprovisioning is a separate explicit workflow; preservation tests.

### T9 — Malicious catalog metadata escapes data root

Crafted paths or identifiers could cause file access outside expected storage roots.

**Current controls:** rejection of absolute paths, parent traversal, empty paths, and invalid entry identifiers.

**Future controls:** canonical filesystem-root confinement, symlink/reparse-point handling, strict parser limits, hostile corpus testing.

### T10 — Malicious or corrupted image is represented as trusted

A checksum string may be valid syntactically but refer to the wrong content, or an unverified image may be shown without clear state.

**Current controls:** SHA-256 metadata syntax validation only.

**Future controls:** content hashing, signature verification, trust-source separation, explicit verified/unverified UI states, policy gating before boot.

### T11 — Compromised GoreeCloud Boot update

A malicious release or update could replace the boot runtime.

**Required controls:** exact source revision traceability, signed release process when established, checksum/signature verification before activation, rollback/recovery path, build provenance.

### T12 — Dependency or license provenance failure

A bundled binary could be unverifiable, vulnerable, or legally incompatible with the distribution.

**Current control:** the Rust foundation has no external runtime crates.

**Required controls when dependencies are introduced:** exact upstream provenance, source availability, license inventory, build recipes, SBOM/provenance records where supported, vulnerability review, aggregate-license compatibility.

### T13 — Optional network boot becomes required control plane

A network feature could create dependency on an external service or expose local boot metadata.

**Required controls:** network boot remains optional; local boot works offline; network destinations/purpose are explicit; least data and least privilege.

## Destructive-operation authorization model

A future write-capable flow should preserve distinct states:

```text
Discovered -> Assessed -> User-selected -> Confirmed -> Revalidated -> Authorized -> Writing -> Verified
                 |             |              |             |
                 +-------------+--------------+-------------+--> Reject/Abort on contradictory evidence
```

No earlier state may be treated as equivalent to `Authorized`.

Current implementation stops before `Confirmed`/`Authorized` for physical devices.

## Current topology and active-use evidence model

For each Linux whole-device candidate, current discovery:

1. records the whole-device major/minor identity;
2. records directly enumerated child-partition identities;
3. recursively follows both sysfs `holders` and `slaves` links from the disk and those partitions;
4. canonicalizes each discovered node and deduplicates canonical paths so reciprocal relation links terminate safely;
5. follows shared holders back down through their slaves so related backing members can enter the candidate safety topology;
6. builds a sorted set of discovered topology major/minor identities;
7. parses all major/minor identities and mount points in `/proc/self/mountinfo`;
8. rejects the candidate when the topology intersects mounted filesystem identities;
9. parses `/proc/swaps` as mandatory global safety evidence;
10. resolves active swap partitions through sysfs and active swap files through their deepest containing mountinfo filesystem;
11. rejects the candidate when the topology intersects resolved active-swap identities; and
12. places topology, mounted-topology, and active-swap-topology state into the revalidation token.

Synthetic evidence confirms that a mounted or active-swap sibling backing member discovered through a shared holder rejects the candidate, reciprocal links do not loop, and adding a new slave-connected topology member invalidates a prior revalidation token.

Unreadable mandatory per-device topology data fails closed for the affected candidate. Unreadable, malformed, unsupported, or unresolvable active-swap evidence fails Linux discovery globally. This is not a claim that sysfs `holders`/`slaves` plus mountinfo plus `/proc/swaps` is a complete Linux destructive-target safety model.

## Release blockers before physical disk writing

The project must not enable or advertise physical destructive provisioning as supported until at least the following exist:

- topology/active-use-aware platform-native device discovery sufficient to protect active host storage across supported Linux configurations;
- validated coverage across representative device-mapper, encryption, RAID, multipath, hotplug/reconfiguration, mount/swap namespace, unusual storage, and other system-storage configurations;
- stable/current-instance identity strategy;
- fresh pre-write identity, topology, mount, swap, and other required state revalidation;
- explicit destructive confirmation;
- production-unique GPT identity generation;
- sector-aware GPT implementation validated independently;
- image-backed end-to-end destructive integration tests;
- filesystem creation/verification tests;
- interrupted-operation/recovery design and tests;
- privilege/device-handle/input-validation review;
- physical test-device validation.

## Current residual risk

Because physical block-device writes remain disabled, the primary present risks are incorrect discovery/assessment output, incomplete topology/active-use interpretation, malformed development GPT metadata, accidental misuse of the regular-file test command, or misleading documentation.

`CAPABILITIES.md`, CLI output, tests, and review records must continue to state that:

- Linux discovery/topology/swap handling is read-only evidence;
- bidirectional `holders`/`slaves` closure plus mountinfo and active-swap resolution is bounded and does not yet qualify every destructive-target relationship;
- a successful assessment/revalidation token is not write authorization;
- GPT generation is currently a regular-file development path;
- no bootable or production-capable device is currently produced.