use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct HealthStatus {
    pub status: &'static str,
}

impl HealthStatus {
    pub fn live() -> Self {
        Self { status: "live" }
    }

    pub fn ready() -> Self {
        Self { status: "ready" }
    }
}
