# Security

## Current security posture

GoreeCloud Boot is in development and is not approved for production, physical provisioning, or recovery-critical use.

The current code intentionally has **no physical block-device write path**. It provides read-only Linux device discovery with bounded recursive topology analysis, target-safety assessment, byte/sector layout planning, GPT metadata generation for new sparse regular-file test images, catalog metadata validation, and development CLI output.

## Current implemented safeguards

The development foundation currently includes:

- read-only Linux device metadata discovery rather than trusting a mutable `/dev` path alone;
- recursive upward traversal of sysfs `holders` relationships from each candidate whole disk and its directly enumerated partitions;
- intersection of that discovered topology with all mounted major/minor identities from `/proc/self/mountinfo`;
- rejection when any mounted filesystem is present in the discovered topology, with explicit root and boot rejection evidence retained;
- fail-closed omission of a candidate when mandatory device or holder-topology metadata cannot be read safely;
- current device-instance and persistent identity evidence where the Linux system exposes it;
- a revalidation-token model that binds current identity, removable/read-only state, discovered topology, and mounted-topology state for later comparison;
- checked byte and sector geometry arithmetic;
- GPT usable-range and partition-overlap validation;
- redundant GPT header/entry-array generation and CRC32 checks;
- no-overwrite regular-file test-image creation;
- refusal to create GPT test images beneath `/dev`, `/sys`, or `/proc`;
- read-back verification of generated GPT metadata;
- no network or telemetry code;
- no external Rust runtime dependencies in the current foundation.

These are development safeguards. They are not a security certification, Wardveil Security integration claim, or destructive-operation authorization implementation.

## Security priorities before physical provisioning

The following remain release-blocking before a block-device write path can be enabled:

- comprehensive wrong-disk prevention;
- destructive-target topology qualification beyond the current recursive `holders` layer, including explicit swap handling and validated coverage for relevant device-mapper, encryption, RAID, multipath, hotplug, namespace, and other storage indirection;
- stable/current-instance target identity and immediate pre-write revalidation;
- explicit destructive user authorization;
- secure binding between revalidated identity/topology and the write target;
- production-unique GPT identity generation;
- independent GPT/parser validation;
- image-backed end-to-end provisioning tests;
- filesystem creation and verification tests;
- interrupted-operation and recovery testing;
- privilege minimization;
- malicious metadata/path handling;
- boot-runtime and release provenance;
- image content hashing and future signature verification;
- clear separation between trusted GoreeCloud components and unverified user images.

See [docs/THREAT-MODEL.md](docs/THREAT-MODEL.md).

## Current topology-security boundary

The implemented Linux topology check is intentionally one-way and conservative for the current target question: starting from a candidate whole disk and its direct partitions, it recursively follows sysfs `holders` links upward and records each holder device's major/minor identity. Any device in that discovered topology that appears in mountinfo makes the candidate ineligible.

This catches important mounted stacked-device cases, including synthetic device-mapper/RAID-style holder chains covered by the test suite. It does **not** establish complete Linux storage safety for destructive use. Swap is not represented in mountinfo, sysfs relationships can change during hotplug/reconfiguration, and not every relevant storage dependency has been independently validated against this model. Physical writes therefore remain disabled.

## Test-image boundary

`bootctl create-test-gpt-image` is a development integration aid, not a device provisioner.

It may create only a new regular path; it refuses to overwrite an existing path and rejects protected pseudo-filesystem/device parents `/dev`, `/sys`, and `/proc`. It writes protective-MBR/GPT metadata only and does not create filesystems or install executable boot code.

A future change that allows this code path to target an existing file or physical device requires a separate security/design review and the destructive-operation controls documented in the threat model.

## Reporting a vulnerability

Do not publish reusable credentials, private keys, tokens, sensitive device data, or exploit details that would unnecessarily increase risk in ordinary issue discussions.

For early development issues that do not contain sensitive exploit information, use the repository issue tracker and clearly describe the affected revision, component, expected behavior, actual behavior, and reproduction conditions.

A dedicated private vulnerability-reporting channel may be established before public release. Until that channel is documented and operational, do not infer that private security reporting is available.

## Supported versions

No Stable GoreeCloud Boot version currently exists. No production security-support window is currently promised.

## Third-party security

No third-party bootloader or firmware framework is currently vendored by this repository. When external components are introduced, exact versions/revisions, provenance, update policy, vulnerability review, and licensing must be documented before release acceptance.
