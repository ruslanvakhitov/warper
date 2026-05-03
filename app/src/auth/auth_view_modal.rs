/// Passive marker retained for stale login-gated call sites while the hosted
/// auth UI is removed from Warper.
#[derive(Debug, Clone, Copy)]
pub enum AuthViewVariant {
    Initial,
    RequireLoginCloseable,
    HitDriveObjectLimitCloseable,
    ShareRequirementCloseable,
}
