# Decoder Fallback Order

For each preview source:

1. Validate source limits and sniff the header after hover confirmation.
2. Try Shell thumbnail cache when it is cheap and sufficient.
3. Try embedded thumbnail or embedded preview when available.
4. Try WIC decode with decoder-native downscale.
5. Try feature-gated Rust fallback decoder.
6. Show nothing for unsupported sources unless diagnostics are explicitly enabled.

## Current Status

- Header sniffing is implemented.
- WIC-first decode boundary is implemented.
- Hard local file size and decoded pixel limits are implemented.
- [blocked] Shell thumbnail cache-only path is not implemented yet.
- [blocked] Embedded thumbnail path is not implemented yet.
- [blocked] Optional Rust fallback decoders are not selected yet.