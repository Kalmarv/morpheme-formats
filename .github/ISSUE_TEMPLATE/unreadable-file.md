---
name: Unreadable file
about: A Morpheme bundle or rig that morpheme-formats refuses or decodes wrongly
labels: unreadable-file
---

**The error** (the full `Error` text: it names the offset and what was expected):

```
Morpheme at 0x...: ...
```

**Which call refused it:** `bundle_be32` / `rig_be32`

**Where the file comes from:** game, platform and version, and the name of the file or
archive entry it is stored in.

**Facts about the file** (instead of the file):

- Total size in bytes, and the record's offset and payload length if `rig_be32` failed.
- The first 128 bytes of the record's payload as hex, and 128 bytes around the reported offset.
- Whether the data is big-endian with 32-bit pointers.

Please do not attach game files, archives or extracts from them.
