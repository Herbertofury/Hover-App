# Image Format Matrix

Hover uses WIC first for local decode. Rust fallback decoders are feature-gated and must pass dependency review before being enabled.

| Format | Current path | Status |
| --- | --- | --- |
| JPEG/JPG/JPE/JFIF | WIC | Implemented in WIC-first decode boundary and wired to overlay; interactive runtime validation pending |
| Progressive JPEG | WIC | Implemented in WIC-first decode boundary and wired to overlay; interactive runtime validation pending |
| PNG | WIC | Implemented in WIC-first decode boundary and wired to overlay; interactive runtime validation pending |
| GIF first frame | `gif` crate active-preview path | Implemented as part of animated GIF playback |
| Animated GIF | `gif` crate active-preview path | Implemented with canvas compositing, disposal handling, frame timing, frame cap, and memory cap |
| BMP | WIC | Implemented in WIC-first decode boundary and wired to overlay; interactive runtime validation pending |
| TIFF/TIF | WIC | Implemented in WIC-first decode boundary and wired to overlay; interactive runtime validation pending |
| ICO | WIC/system codec | Header sniff implemented; runtime preview unverified |
| WebP | WIC/system codec when available | Header sniff implemented; runtime depends on installed codec |
| AVIF | WIC/system codec when available | Header sniff implemented; optional Rust fallback deferred |
| HEIC/HEIF | WIC/system codec when available | Header sniff implemented; runtime depends on installed codec |
| SVG | Feature-gated rasterizer | [blocked] No rasterizer selected yet |
| JPEG XL | Feature-gated decoder | [blocked] No decoder selected yet |
| RAW | Embedded thumbnail/WIC codec | Deferred |

Missing codecs must fail gracefully with no popup in normal mode.