# Branding

## Product identity

- **Official product name:** GoreeCloud Boot
- **Repository name:** `goreecloud-boot`
- **Short description:** Native open-source multiboot USB platform for booting, installing, recovering, diagnosing, and maintaining operating systems and GoreeCloud environments.
- **Product class:** GoreeCloud native application and boot platform

## Naming rules

Use **GoreeCloud Boot** in user-facing prose and documentation.

Use `goreecloud-boot` for the repository and project slug.

Use `bootctl` only for the host-side command-line utility unless a later approved naming decision changes it.

The partition labels are currently specified as:

- `GCBOOT` — boot-system partition
- `GCDATA` — user-facing data partition

## Visual identity status

An official GoreeCloud Boot logo/icon/artwork set has **not yet been approved or added to this repository**.

Do not substitute the Ventoy logo, upstream bootloader branding, generic USB artwork, or an unofficial GoreeCloud mark as the official GoreeCloud Boot product identity.

When official artwork is created, repository references should point to the approved GoreeCloud-controlled source and preserve any applicable Glaze UI and GoreeCloud branding requirements.

## Interface identity

The future preboot interface should identify itself as GoreeCloud Boot while remaining visually restrained and firmware-appropriate. It should apply Glaze UI principles only to the extent the firmware environment can substantively implement them; documentation must not imply desktop Glaze UI parity where that does not exist.

## Third-party identity

Third-party component names, marks, and notices must remain attributable to their respective projects where required. Their presence must not imply that they are GoreeCloud-owned products or that GoreeCloud Boot is an official upstream distribution.
