# Browser companion target lifecycle

The optional content script sends `exit` when an active image target loses window focus, its document becomes hidden, or the page leaves. Repeated cleanup events are idempotent. Returning to the page does not reactivate a target without a new hover event; ordinary mouseout and internal image movement behavior remain unchanged.

Run `node --test tests/browser/*.test.cjs`. Tests execute the actual content script in a VM with event-surface fixtures, covering blur, pagehide, visibility return, duplicate cleanup and ordinary pointer behavior. Before repair, all three inactive-page scenarios failed.

This verifies emitted target state, not a real browser extension installation or native preview delivery. The current native messaging host validates/acknowledges URLs; these tests do not establish end-to-end browser/native rendering. Native Windows preview remains independent and unchanged. No extension installation, permissions, grants, network listener or settings were changed.
