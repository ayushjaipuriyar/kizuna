use crate::discovery::{Discovery, Peer};
use anyhow::{anyhow, Result};
use async_trait::async_trait;

pub struct MdnsDiscovery {}

impl MdnsDiscovery {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl Discovery for MdnsDiscovery {
    async fn announce(&self) -> Result<()> {
        // Placeholder: integrate with a proper async mDNS implementation later.
        Err(anyhow!("mDNS announce not implemented"))
    }

    async fn browse(&self) -> Result<Vec<Peer>> {
        // For now, return an error so callers can fall back to other methods.
        Err(anyhow!("mDNS browse not implemented"))
    }
}
