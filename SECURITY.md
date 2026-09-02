# Security

## Current security posture

GoreeCloud Boot is in development and is not approved for production, physical provisioning, or recovery-critical use.

The current code intentionally has **no physical block-device write path**. It provides read-only Linux device discovery, target-safety assessment, byte/sector layout planning, GPT metadata generation for new sparse regular-file test images, catalog metadata validation, and development CLI output.

## Current implemented safeguards

The development foundation currently includes:

- read-only Linux device metadata discovery rather than trusting a mutable `/dev` path alone;
- mounted root and boot rejection for the discovered whole-device/direct-partition family;
- current device-instance and persistent identity evidence where the Linux system exposes it;
- a revalidation-token model for comparing later discovery evidence;
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
- topology-aware root/boot/swap/system-storage exclusion, including relevant device-mapper, encryption, RAID, multipath, and other indirection;
- stable/current-instance target identity and immediate pre-write revalidation;
- explicit destructive user authorization;
- secure binding between revalidated identity and the write target;
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
