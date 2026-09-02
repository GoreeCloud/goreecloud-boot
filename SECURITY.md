# Security

## Current security posture

GoreeCloud Boot is in development and is not approved for production or recovery-critical use.

The current code intentionally has **no block-device write path**. It provides only target-safety evaluation, layout planning, catalog metadata validation, and development CLI output.

## Security priorities

The project treats the following as release-blocking concerns before destructive provisioning is enabled:

- wrong-disk write prevention;
- stable target identity and immediate pre-write revalidation;
- integer/bounds safety for partition calculations;
- malformed and malicious catalog metadata;
- boot-runtime and release provenance;
- dependency provenance and license review;
- image checksum and future signature verification;
- interrupted-update recovery;
- privilege minimization;
- clear separation between trusted GoreeCloud components and unverified user images.

See [docs/THREAT-MODEL.md](docs/THREAT-MODEL.md).

## Reporting a vulnerability

Do not publish reusable credentials, private keys, tokens, sensitive device data, or exploit details that would unnecessarily increase risk in ordinary issue discussions.

For early development issues that do not contain sensitive exploit information, use the repository issue tracker and clearly describe the affected revision, component, expected behavior, actual behavior, and reproduction conditions.

A dedicated private vulnerability-reporting channel may be established before public release. Until that channel is documented and operational, avoid making claims that private security reporting is available.

## Supported versions

No Stable GoreeCloud Boot version currently exists. No production security-support window is currently promised.

## Third-party security

No third-party bootloader or firmware framework is currently vendored by this repository. When external components are introduced, exact versions/revisions, provenance, update policy, vulnerability review, and licensing must be documented before release acceptance.
