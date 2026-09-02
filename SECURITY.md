# Security

## Current security posture

GoreeCloud Boot is in development and is not approved for production, physical provisioning, or recovery-critical use.

The current code intentionally has **no physical block-device write path**. It provides read-only Linux device discovery with bounded bidirectional topology and active-swap analysis, target-safety assessment, byte/sector layout planning, GPT metadata generation for new sparse regular-file test images, catalog metadata validation, and development CLI output.

## Current implemented safeguards

The development foundation currently includes:

- read-only Linux device metadata discovery rather than trusting a mutable `/dev` path alone;
- recursive traversal of both sysfs `holders` and `slaves` relationships from each candidate whole disk and its directly enumerated partitions;
- canonical-path cycle protection for reciprocal holder/slave links;
- closure through shared holders so related backing members can become part of the same candidate safety topology;
- intersection of that discovered topology with all mounted major/minor identities from `/proc/self/mountinfo`;
- rejection when any mounted filesystem is present in the discovered topology, with explicit root and boot rejection evidence retained;
- active-swap discovery from `/proc/swaps`, resolving swap partitions to sysfs major/minor identities and swap files to the deepest containing mountinfo filesystem;
- rejection when active swap intersects the discovered candidate topology;
- fail-closed omission of a candidate when mandatory device or required holder/slave-topology metadata cannot be read safely;
- fail-closed Linux discovery when global active-swap metadata is unreadable, malformed, unsupported, or cannot be resolved safely;
- current device-instance and persistent identity evidence where the Linux system exposes it;
- a revalidation-token model that binds current identity, removable/read-only state, discovered topology, mounted-topology state, and active-swap-topology state for later comparison;
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
- destructive-target topology qualification beyond the current bidirectional `holders`/`slaves`, mountinfo, and active-swap layers, including validated coverage for relevant device-mapper, encryption, RAID, multipath, hotplug/reconfiguration, mount/swap namespaces, unusual storage configurations, and other storage indirection;
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

The implemented Linux topology check is intentionally conservative for the current target question. Starting from a candidate whole disk and its direct partitions, it recursively traverses both sysfs `holders` and `slaves` links. Canonical paths are deduplicated so normal reciprocal links cannot loop indefinitely. Traversing downward from a shared holder can add sibling backing devices to the same candidate topology; this prevents a candidate from appearing isolated when it participates in a larger active stack.

The resulting major/minor set is intersected with mountinfo. Any mounted device in the discovered topology makes the candidate ineligible. Synthetic tests verify a mounted sibling backing member reached through a shared holder is sufficient to reject the candidate.

Active swap is checked separately because it is not represented by ordinary mountinfo entries. `/proc/swaps` is parsed as mandatory global safety evidence. Swap partitions are resolved to sysfs major/minor identities; swap files are canonicalized and associated with the deepest mountinfo filesystem containing the file. Any resolved swap device that intersects the candidate topology makes the candidate ineligible. Synthetic tests verify active swap on a slave-connected sibling backing member is sufficient to reject the candidate. If the active-swap table cannot be read, has an unexpected or unsupported entry, or an active swap area cannot be resolved safely, Linux discovery fails closed.

These checks catch important mounted and swap-backed stacked-device cases. They do **not** establish complete Linux storage safety for destructive use. Sysfs, mount, and swap relationships can change during hotplug/reconfiguration, mount/swap namespaces can alter visible state, and not every relevant device-mapper, encryption, RAID, multipath, unusual storage, or other dependency has been independently validated against this model. Physical writes therefore remain disabled.

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