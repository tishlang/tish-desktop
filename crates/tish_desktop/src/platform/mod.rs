//! PlatformHost — thin adapters over native UI hosts (apple / future ms / lin).

use serde_json::Value;
use tishlang_core::Value as TishValue;

mod apple;
mod lin;
mod ms;

#[allow(unused_imports)]
pub use apple::ApplePlatformHost;
#[allow(unused_imports)]
pub use lin::LinPlatformHost;
#[allow(unused_imports)]
pub use ms::MsPlatformHost;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformId {
    Apple,
    Ms,
    Lin,
    DesktopTauri,
}

/// Contract for attaching a native surface under the desktop umbrella.
pub trait PlatformHost: Send + Sync {
    fn id(&self) -> PlatformId;
    fn name(&self) -> &'static str;

    /// Whether this host can create `kind: "native"` surfaces in-process.
    fn supports_native_surface(&self) -> bool;

    /// Attach / open a native root without owning the outer event loop.
    /// Default: unsupported.
    fn attach_native(&self, _root: &TishValue, _options: &Value) -> Result<Value, String> {
        Err("native attach unsupported on this platform".into())
    }
}

/// Active host for the current build (apple when feature enabled + macOS).
pub fn current_host() -> Box<dyn PlatformHost> {
    #[cfg(all(feature = "platform-apple", target_os = "macos"))]
    {
        return Box::new(ApplePlatformHost);
    }
    #[cfg(all(feature = "platform-ms", target_os = "windows"))]
    {
        return Box::new(MsPlatformHost);
    }
    #[cfg(all(feature = "platform-lin", target_os = "linux"))]
    {
        return Box::new(LinPlatformHost);
    }
    Box::new(DesktopOnlyHost)
}

struct DesktopOnlyHost;

impl PlatformHost for DesktopOnlyHost {
    fn id(&self) -> PlatformId {
        PlatformId::DesktopTauri
    }

    fn name(&self) -> &'static str {
        "desktop-tauri"
    }

    fn supports_native_surface(&self) -> bool {
        false
    }
}
