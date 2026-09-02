# Threat Model

## Scope

This threat model begins with the highest-risk GoreeCloud Boot behavior: selecting and modifying removable block devices. It will expand as bootloaders, image verification, networking, persistence, and update channels are implemented.

## Assets to protect

- host system disks and partitions;
- user data on non-target storage devices;
- `GCDATA` contents during routine updates/repair;
- GoreeCloud Boot runtime integrity;
- image/catalog trust metadata;
- release provenance and signing material;
- device identity used for destructive authorization.

## Current trust boundary

The current implementation accepts explicit device evidence as command-line input and **does not write to a block device**. Supplied evidence is therefore test/development input, not trusted discovery data.

A future destructive implementation must discover its own evidence and must not trust user-supplied booleans such as `--removable yes` as sufficient authorization.

## Primary threats

### T1 — Wrong target selected by mutable device path

A removable device can change from `/dev/sdb` to another path, or a different device can appear at the same path.

**Required controls:** stable identity, topology checks, mount/system-disk checks, immediate pre-write identity revalidation, abort on mismatch.

### T2 — System disk incorrectly treated as removable

Hardware, USB bridges, unusual storage, virtual environments, or incomplete discovery can produce misleading removable-state signals.

**Required controls:** multiple independent signals; explicit root/boot/swap/system-volume exclusion; conservative rejection when evidence conflicts; no single removable flag as sole authority.

### T3 — Time-of-check/time-of-use device replacement

The target can disappear and a new device can occupy its path after confirmation.

**Required controls:** re-open/re-identify immediately before writes; compare immutable/stable identifiers and capacity/topology; keep destructive transaction bound to verified device handle where platform APIs permit.

### T4 — Arithmetic or geometry error damages media

Overflow, sector-size assumptions, alignment mistakes, or incorrect GPT calculations can generate writes outside intended ranges.

**Current controls:** checked byte arithmetic and minimum-size validation in the planning layer.

**Future controls:** sector-aware checked arithmetic, redundant invariant validation, test vectors for 512/4096-byte sectors, image-backed destructive tests before physical hardware.

### T5 — Interrupted provisioning leaves ambiguous media

Power loss, unplugging, process termination, or write failure can leave inconsistent partition/boot state.

**Required controls:** staged operations where possible, explicit operation journal/recovery metadata, safe ordering, post-write verification, documented repair flow.

### T6 — Routine update erases user data

An update intended for boot-system files could incorrectly reinitialize the whole device.

**Required controls:** separate `GCBOOT` and `GCDATA` identities; update operations target verified boot partition only; full reprovisioning is a different explicit workflow; preservation tests.

### T7 — Malicious catalog metadata escapes data root

Crafted paths or identifiers could cause file access outside expected storage roots.

**Current controls:** rejection of absolute paths, parent traversal, empty paths, and invalid entry identifiers.

**Future controls:** canonical filesystem-root confinement, symlink/reparse-point handling, strict schema/parser limits, hostile corpus testing.

### T8 — Malicious or corrupted image is represented as trusted

A checksum string may be valid syntactically but refer to the wrong content, or an unverified image may be shown without clear state.

**Current controls:** SHA-256 metadata syntax validation only.

**Future controls:** content hashing, signature verification, trust-source separation, explicit verified/unverified UI states, policy gating before boot.

### T9 — Compromised GoreeCloud Boot update

A malicious release or update could replace the boot runtime.

**Required controls:** exact source revision traceability, signed release process when established, checksum/signature verification before activation, rollback/recovery path, build provenance.

### T10 — Dependency or license provenance failure

A bundled binary could be unverifiable, vulnerable, or legally incompatible with the distribution.

**Required controls:** exact upstream provenance, source availability, license inventory, build recipes, SBOM/provenance records where supported, review of aggregate compatibility.

### T11 — Optional network boot becomes required control plane

A network feature could accidentally create dependency on an external service or expose local boot metadata.

**Required controls:** network boot remains optional; local boot functions operate offline; network destinations/purpose are explicit; least data and least privilege.

## Destructive-operation authorization model

A future write-capable flow should have distinct states:

```text
Discovered -> Assessed -> User-selected -> Confirmed -> Revalidated -> Authorized -> Writing -> Verified
                 |             |              |             |
                 +-------------+--------------+-------------+--> Reject/Abort on contradictory evidence
```

No earlier state may be treated as equivalent to `Authorized`.

## Release blockers before disk writing

The project must not enable or advertise physical destructive provisioning as supported until at least the following exist:

- platform-native device discovery;
- stable identity strategy;
- system/root/boot exclusion;
- fresh pre-write revalidation;
- explicit destructive confirmation;
- sector-aware layout implementation;
- image-backed integration tests;
- interrupted-operation/recovery design;
- privilege and input-validation review;
- physical test-device validation.

## Current residual risk

Because the current implementation does not write to disks, the primary present risks are incorrect planning output or misleading documentation. `CAPABILITIES.md` and CLI wording must therefore continue to state that planning is non-destructive and supplied device evidence is not authoritative discovery.
