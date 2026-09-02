# Third-Party Notices and Provenance

## Current state

The current GoreeCloud Boot foundation does not vendor, link, embed, or distribute a third-party bootloader, firmware framework, filesystem implementation, network-boot component, or Rust crate dependency.

The initial Rust workspace uses the Rust standard library only.

## Candidate future foundations

The following technologies are candidates for bounded integration and are **not currently part of this repository's distributed implementation**:

| Component | Intended possible role | Integration status |
| --- | --- | --- |
| GNU GRUB | Bootloader/boot-method foundation | Not integrated |
| EDK II | UEFI development/foundation components | Not integrated |
| iPXE | Optional network boot | Not integrated |

Before any candidate becomes a distributed dependency, this file or a successor machine-readable dependency inventory must record:

- authoritative upstream project and repository;
- exact version/tag/commit;
- component license(s);
- required copyright and attribution notices;
- whether source is vendored, linked, built separately, or distributed as an aggregate;
- GoreeCloud patches and their provenance;
- corresponding-source obligations;
- build inputs and reproducibility information;
- known license compatibility constraints;
- security/update ownership.

## License compatibility boundary

GoreeCloud-owned Boot source is intended to use the SPDX expression `GPL-3.0-or-later`.

Third-party components retain their own licenses. A component must not be combined into a derivative or linked work when its license is incompatible with the resulting distribution model.

In particular, any future iPXE integration must be reviewed at the actual file/build level before distribution because iPXE licensing is not safely summarized by assuming every possible build has one identical license combination.

## No implicit endorsement or ownership

A third-party component's presence in a future build will not make that component GoreeCloud-owned, and GoreeCloud branding must not obscure upstream copyright, license, or attribution requirements.
