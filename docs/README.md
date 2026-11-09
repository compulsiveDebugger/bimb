# Bimb documentation

Bimb is a local photo and video library. It indexes folders without moving originals and keeps a searchable catalog and small thumbnails when a drive is disconnected.

## Start here

| You want to… | Read |
|---|---|
| Install Bimb on a Mac or Windows PC | [Installation](installation.md) |
| Add photos, search, organize and back up your library | [User guide](user-guide.md) |
| Fix a blocked launch, missing drive, unsupported file or scan problem | [Troubleshooting](troubleshooting.md) |
| Build the application from source | [Development](development.md) |
| Create installers, check signatures and prepare a release | [Packaging and release](releasing.md) |
| See what has actually been tested | [Validation evidence](validation.md) |
| Understand the agreed scope and design | [Product](product.md) · [Architecture](architecture.md) |

**Current channel: development preview.** macOS Apple Silicon has local native testing. Windows native tests and lint have passed in CI; installer and desktop interaction checks are separate. Public signing/notarization and clean-machine acceptance remain release gates. Read the platform status in the installation guide before sharing a preview.

## A few terms

- **Source:** a folder or mounted drive Bimb is allowed to index.
- **Original:** the photo or video in that source. Bimb opens originals for reading.
- **Catalog:** Bimb's local metadata, favorites, tags, captions, albums and collections.
- **Thumbnail:** a small cached image for browsing while an original is unavailable.
- **Album:** a manually chosen set of catalog items.
- **Saved collection:** a named search whose results change as matching media is indexed or edited.
- **Catalog backup:** a snapshot of Bimb's organization and metadata. It does not contain original media or thumbnails.

Installer packages include these Markdown guides so they can also be read offline in a text editor. Images in the guides use synthetic fixtures.
