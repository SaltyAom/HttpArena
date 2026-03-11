---
title: Upload
---

Required for the `upload` and `mixed` profiles. The mixed profile uses a 1 MB body; the standalone upload test uses 20 MB.

## `POST /upload`

Read the entire request body (up to 20 MB binary) and return its CRC32 checksum as an 8-character lowercase hex string.

```
POST /upload (body: 20MB binary) → "a1b2c3d4"
```

The CRC32 uses the ISO 3309 polynomial (`0xEDB88320`), the same as `zlib.crc32`.

Response must have `Content-Type: text/plain`.
