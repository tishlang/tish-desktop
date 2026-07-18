# hello-ios (cross-device / surfaces)

**tish-desktop** demo: switch between full **Native** and full **Webview** panes with the same BrokerCore features.

| Layer | Repo |
|-------|------|
| This example (parity switcher) | **tish-desktop** |
| Pure-native UIKit hello | **tish-apple** `examples/hello-ios` |
| UIKit host crate | **tish-apple** `tish-ios` |
| BrokerCore | **tish-desktop** `crates/tish_broker` (standalone crate) |

## Quick start

```bash
# from tish-desktop (sibling tish-apple + tish required)
npm run example:hello-ios
# or
cd examples/hello-ios && npm install && npm run run
```

For the minimal host-only counter (no broker/webview), use tish-apple:

```bash
cd ../tish-apple/examples/hello-ios && npm run run
```
