use crate::discovery::{Discovery, Peer};
use anyhow::Result;
use async_trait::async_trait;
use std::net::SocketAddr;
use std::time::Duration;

pub struct UdpDiscovery {
    port: u16,
}

impl UdpDiscovery {
    pub fn new() -> Self {
        Self { port: 9999 }
    }
}

#[async_trait]
impl Discovery for UdpDiscovery {
    async fn announce(&self) -> Result<()> {
        // In a full implementation we'd reply to discovery pings.
        Ok(())
    }

    async fn browse(&self) -> Result<Vec<Peer>> {
        // Simple broadcast-based discovery: send a short message and collect replies
        let msg = b"KIZUNA_DISCOVER";

        // Create a std UdpSocket so we can enable broadcast, then convert to tokio socket.
        let std_sock = std::net::UdpSocket::bind("0.0.0.0:0")?;
        std_sock.set_broadcast(true)?;
        let tokio_sock = tokio::net::UdpSocket::from_std(std_sock)?;

        let broadcast_addr: SocketAddr = format!("255.255.255.255:{}", self.port).parse()?;
        tokio_sock.send_to(msg, broadcast_addr).await?;

        // listen for replies for a short window
        let mut buf = [0u8; 1024];
        let mut peers = Vec::new();

        let listen = async {
            loop {
                match tokio::time::timeout(Duration::from_millis(400), tokio_sock.recv_from(&mut buf)).await {
                    Ok(Ok((n, addr))) => {
                        let s = String::from_utf8_lossy(&buf[..n]).to_string();
                        // very small parsing: expect `ID:<id>` or `PEER:<id>:<port>`
                        if s.starts_with("PEER:") {
                            let parts: Vec<&str> = s.trim().split(':').collect();
                            if parts.len() >= 3 {
                                let id = parts[1].to_string();
                                let port: u16 = parts[2].parse().unwrap_or(0);
                                peers.push(Peer { id, addr: addr.ip().to_string(), port });
                            }
                        } else {
                            peers.push(Peer { id: s.trim().to_string(), addr: addr.ip().to_string(), port: addr.port() });
                        }
                    }
                    _ => break,
                }
            }
            peers
        };

        let found = listen.await;
        Ok(found)
    }
}
