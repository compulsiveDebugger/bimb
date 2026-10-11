# Reference-product research

Reviewed 10 October 2026 by three exploratory agents, one per product. Sources below are official product documentation or vendor announcements. These establish documented offerings, not independently audited behavior. No hands-on competitor testing was performed.

## Comparison

| Product | Relevant strengths | Lesson for Bimb |
|---|---|---|
| Mylio | Catalog and optimized representations across devices, offline access, storage visibility, Vault backups, metadata organization | Make original availability and thumbnail availability separate, visible concepts. Model drives before paths. |
| Google Photos | Approachable search, albums, memories, exclusion controls, place information | Make everyday retrieval easy and memories controllable. Keep metadata provenance and organization semantics clear. |
| Excire Foto | Local AI, content/face search, duplicates, culling, photo/video catalog | Keep analysis local, versioned and optional. Distinguish indexing/preview/playback capabilities. |

## Mylio

Mylio documents a catalog copied across devices for offline browsing, with originals and configured Vault copies handled separately. Its Storage Panel distinguishes thumbnails, optimized images, originals and storage locations. Bimb should make “thumbnail available” and “connect this drive to open the original” explicit. [Library protection](https://support.mylio.com/how-does-mylio-photos-protect-my-photos), [Storage Panel](https://support.mylio.com/understanding-the-storage-panel).

A particularly relevant support article describes linking an external folder before registering the drive: files can appear missing after disconnecting, and registering the drive later can duplicate folder structures. The documented repair can lose album/event/category membership. Bimb's proposed volume registry and stable asset IDs directly address this class of problem. [Linked-folder troubleshooting](https://support.mylio.com/linked-folders-and-external-drives-what-to-do-if-you-linked-first).

Mylio supports metadata-driven organization and documents catalog/XMP metadata storage with optional original write-back. Its “On This Day” filter retrieves media captured on the current calendar date across years. These support Bimb's proposal to deliver useful search and memories before ML. [Metadata](https://support.mylio.com/how-does-mylio-photos-handle-metadata-mylio-photos-help-center), [On This Day](https://inspire.mylio.com/on-this-day-rediscover-memories-with-mylio-photos/).

Mylio describes cloud-independent operation with optional cloud integrations; it also has an account/login system. This research did not establish a precise periodic reauthentication requirement. Its Vault feature stores originals and organization for protection, which is a larger scope than Bimb's initial index-in-place catalog. [Cloud model](https://support.mylio.com/is-mylio-photos-a-cloud-service), [Login](https://support.mylio.com/logging-in-to-an-existing-mylio-account), [Vault backups](https://mylio.com/features/auto-photo-backup/).

Current official release notes list version 24.8, updated August 11, 2026. Historical pricing pages mix older Photos/Photos+ and current Personal/Business offerings, so numeric prices are intentionally omitted from this design research. [Release notes](https://support.mylio.com/version-24-release-notes).

## Google Photos

Google's current search documentation describes descriptive queries, exact quoted searches over supported fields, people labels, and relevance/chronological results. Ask Photos availability depends on platform/account/region. Bimb should preserve predictable metadata search and explicit sorting even if local semantic search is added later. [Search](https://support.google.com/photos/answer/15235862?co=GENIE.Platform%3DDesktop&hl=en-419).

Albums support manual ordering and date-based sort options. Removing an album item does not remove it from the library. Archive has different semantics again: archived items can remain in albums and search. Bimb should keep album removal, memory exclusion, catalog removal and original deletion distinct. [Albums](https://support.google.com/photos/answer/6128849?hl=en), [Archive](https://support.google.com/photos/answer/7362432?co=GENIE.Platform%3DAndroid&hl=en).

Featured memories include time/themed experiences and controls for dates, people/pets, items and disabling memories. Google explicitly documents that some memories can use local on-device content. It would be inaccurate to describe Google Photos as incapable of local memories. Bimb's distinction is its complete offline desktop workflow over ordinary folders/removable drives. [Memories](https://support.google.com/photos/answer/9454489?co=GENIE.Platform%3DAndroid&hl=en).

Location information can originate in the camera, user edits, or estimates, with different editing/sharing rules. Similar-photo stacks are separate from exact duplicate detection and do not themselves save storage. Bimb should store location provenance and keep grouping independent from deletion. [Locations](https://support.google.com/photos/answer/6153599?co=GENIE.Platform%3DAndroid&hl=en), [Stacks](https://support.google.com/photos/answer/14169846?co=GENIE.Platform%3DAndroid&hl=en).

Google states that Photos personal data is not used for ads. Its Gemini privacy documentation describes use of photos and related information for Gemini features, possible query review with controls, and a model-training restriction specifically concerning models outside Google Photos. These should not be simplified into blanket claims about all AI training or all offline behavior. Bimb can instead state its own narrower, testable local-processing promise. [Privacy hub](https://support.google.com/photos/answer/15344015?hl=en-GB), [Photos privacy](https://safety.google/products/photos/).

## Excire Foto

Excire Foto 2027 is an actual released offering, despite its version year: the official announcement is dated June 15, 2026, with availability June 16. It offers photo/video management, local AI search, OCR, map/timeline exploration and culling, with a perpetual license. [Release announcement](https://excire.com/en/excire-foto-2027-is-here/), [Product](https://excire.com/en/excire-foto/).

Excire states that analysis is local and image data is not uploaded. Its support documentation nevertheless requires internet license validation every 30 days; account/activation requirements are also documented. Thus local AI is not equivalent to indefinitely disconnected operation. Bimb's proposed account-free offline contract is a meaningful distinction. [Network/license FAQ](https://support.excire.com/portal/en/kb/articles/does-excire-upload-data-to-the-cloud), [Activation information](https://excire.com/en/excire-foto/).

An older official quickstart documents catalog previews while external originals are disconnected. Current support documents folder relocation after path/drive-letter changes but limits whole-drive reassignment. These support designing Bimb around source-level relinking; the older preview documentation is not a hands-on verification of the current release. [2022 quickstart](https://excire.com/manuals/ExcireFoto2022_Quickstart-EN.pdf), [Relocation](https://support.excire.com/portal/en/kb/articles/what-should-i-do-if-a-folder-path-or-drive-letter-has-changed).

Excire distinguishes binary duplicates, visual similarity and sequences; exact duplicate detection requires originals. People search/tagging also includes human verification rather than an assumption that every match should automatically inherit a name. Bimb should preserve these distinctions when those features arrive. [Duplicates](https://learning-center.excire.com/s/excire-foto/m/excire-foto-2024-quickstart-en/a/find-duplicates), [People tagging](https://support.excire.com/portal/en/kb/articles/can-excire-foto-tag-people-automatically).

Official video documentation distinguishes preview/analysis technology from playback and records OS/codec, HEVC, HDR and rotation limitations. That page concerns the 2025 version and is evidence of relevant engineering failure modes, not proof that each remains a 2027 defect. The current format FAQ also contains legacy version wording, so Bimb needs its own tested per-operation capability matrix. [Video details](https://support.excire.com/portal/en/kb/articles/addl-info-excire-foto-2025-videos), [Formats](https://excire.com/en/excire-foto/).

## Bimb conclusions — recommendations

1. Lead with a complete offline personal-library workflow, and make its network/privacy contract precise.
2. Preserve stable library identities through drive changes; distinguish offline, inaccessible, missing, and removed.
3. Separate original media, catalog, thumbnails, user organization and future analysis outputs.
4. Make metadata search, sorting and controllable memories useful before adding ML.
5. Treat video as a first-class workload. For the agreed 3,000 files / 1 TB, media I/O and decoding matter more than sophisticated distributed indexing.
6. Expose storage and scan status honestly; never imply that an index or thumbnail is an original-media backup.
