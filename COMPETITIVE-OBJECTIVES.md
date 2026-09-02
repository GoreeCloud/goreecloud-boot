# Competitive Objectives

These are design and engineering objectives for GoreeCloud Boot. They are **not** claims that the current repository already meets or exceeds another product.

## Primary objective

Build a native, open-source multiboot and recovery platform whose strongest differentiators are safety, explicit trust state, recoverability, offline independence, and truthful compatibility reporting.

## Benchmark areas

### Device safety

Objective: make wrong-disk prevention a first-class release requirement rather than a confirmation-dialog afterthought.

Success criteria should eventually include stable device identity, root/system-disk exclusion, immediate pre-write revalidation, interruption handling, and destructive-operation test coverage.

### Update and recovery model

Objective: preserve user image/data storage during ordinary boot-runtime upgrades and repair.

Success criteria should eventually include repeatable `GCBOOT` replacement/repair with verified preservation of `GCDATA`.

### Compatibility transparency

Objective: report support per boot method, image family, architecture, version, and validation state rather than relying on a universal-compatibility marketing claim.

Success criteria should include a maintained compatibility test matrix and reproducible evidence.

### Integrity and trust

Objective: make catalog checksums, signatures, release provenance, and trust state visible and enforceable.

Success criteria should eventually include signed GoreeCloud release artifacts, verified boot-runtime provenance, image verification policy, and explicit handling of unverified media.

### Offline independence

Objective: ensure core USB boot, local image discovery, recovery, and provisioning do not require a GoreeCloud account, vendor account, subscription, telemetry endpoint, or hosted control plane.

### Maintainability

Objective: keep the GoreeCloud-owned orchestration and safety layer understandable, modular, documented, tested, and independently buildable.

### User experience

Objective: provide a restrained firmware-appropriate Glaze UI experience with clear hierarchy, visible focus, keyboard accessibility, high-contrast state communication, and explicit destructive-action language.

### Open-source provenance

Objective: maintain auditable source and license provenance for every distributed component and avoid creating incompatible combined works.

## Reference products and technologies

Ventoy may be evaluated as a capability and compatibility reference. GNU GRUB, EDK II, iPXE, and other mature open-source boot technologies may be evaluated as bounded foundations.

Reference use does not authorize copying product source, branding, artwork, unsupported compatibility claims, or architectural decisions into GoreeCloud Boot.

## Non-objectives

GoreeCloud Boot does not need to maximize the number of nominally recognized image formats at the expense of safety, testability, maintainability, or truthful support state.

Legacy BIOS support is not a competitive requirement unless actual GoreeCloud use cases justify the additional implementation and validation burden.
