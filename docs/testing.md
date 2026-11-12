# Testing

## Automated Tests

Current command:

```powershell
cargo test --workspace
```

Current result:

- 20 passed.
- 0 failed.

Covered:

- Zoom anchor preservation.
- Zoom clamp and invalid-scale prevention.
- Pan clamp.
- Fit All/Fit Width/Fit Height scale math.
- Preview placement edge flipping.
- Cache hard-cap eviction.
- Replacement decode queue behavior.
- Settings parse and invalid fallback.
- Header sniffing and pixel-limit enforcement.
- Dirty render present behavior.
- Browser URL validation.

## Blocked Manual/Performance Tests

- [blocked] Explorer hover tests require interactive File Explorer control.
- [blocked] Desktop hover tests require interactive Desktop control.
- [blocked] Task Manager, Resource Monitor, WPR/WPA, Process Explorer, and GPUView checks require running the finished native GUI app interactively.
- [blocked] Installer install/uninstall tests require packaging the final app and running installer flows.