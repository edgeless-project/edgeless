use edgeless_api::link::LinkWriter;
use futures::FutureExt;
use std::ops::{DerefMut, Mul};

#[derive(Clone)]
struct MulticastWriter {
    sender: tokio::sync::mpsc::UnboundedSender<Vec<u8>>,
}

#[derive(Clone)]
pub struct MulticastLink {
    reader: std::sync::Arc<tokio::sync::Mutex<Option<Box<dyn LinkWriter>>>>,
    writer: Box<MulticastWriter>,
    task: std::sync::Arc<tokio::sync::Mutex<tokio::task::JoinHandle<()>>>,
}

#[derive(Clone)]
pub struct MulticastProvider {
    links: std::sync::Arc<tokio::sync::Mutex<std::collections::HashMap<edgeless_api::link::LinkInstanceId, Box<MulticastLink>>>>,
}

struct ActiveMulticastLink {
    addr: std::net::Ipv4Addr,
    active_nodes: Vec<edgeless_api::function_instance::NodeId>,
}

pub struct MulticastManager {
    pool_free: Vec<std::net::Ipv4Addr>,
    active: std::collections::HashMap<edgeless_api::link::LinkInstanceId, ActiveMulticastLink>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct MulticastConfig {
    ip: std::net::Ipv4Addr,
    port: u16,
}

impl MulticastLink {
    pub fn new(addr: std::net::Ipv4Addr, port: u16) -> Self {
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel::<Vec<u8>>();
        let reader: std::sync::Arc<tokio::sync::Mutex<Option<Box<dyn LinkWriter>>>> = std::sync::Arc::new(tokio::sync::Mutex::new(None));

        let reader_clone = reader.clone();
        let task = tokio::task::spawn(async move {
            let addr = addr;
            let sock_addr = std::net::SocketAddrV4::new(addr, port);
            let mut receiver = receiver;

            assert!(addr.is_multicast());

            let sock = tokio::net::UdpSocket::bind(sock_addr.clone()).await.unwrap();
            sock.join_multicast_v4(addr, "0.0.0.0".parse().unwrap()).unwrap();
            let mut buffer = vec![0 as u8; 5000];

            loop {
                tokio::select! {
                    outgoing = Box::pin(receiver.recv()).fuse() => {
                        if let Some(outgoing) = outgoing {
                            sock.send_to(&outgoing[..], sock_addr).await.unwrap();
                        }
                    },
                    incomming = Box::pin(sock.recv_from(&mut buffer[..])).fuse() => {
                        match incomming {
                            Ok((data_size, sender)) => {
                                if let Some(r) = reader_clone.lock().await.deref_mut() {
                                    let data = Vec::from(&buffer[0..data_size]);
                                    r.handle(data).await;
                                }
                            },
                            Err(err) => {
                                log::error!("{}", err);
                            },
                        }
                    }
                }
            }
        });

        MulticastLink {
            reader: reader,
            writer: Box::new(MulticastWriter { sender: sender }),
            task: std::sync::Arc::new(tokio::sync::Mutex::new(task)),
        }
    }
}

impl MulticastProvider {
    pub fn new() -> Self {
        MulticastProvider {
            links: std::sync::Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
        }
    }
}

#[async_trait::async_trait]
impl edgeless_api::link::LinkProvider for MulticastProvider {
    async fn create(&mut self, req: edgeless_api::link::CreateLinkRequest) -> anyhow::Result<Box<dyn edgeless_api::link::LinkInstance>> {
        let cfg: MulticastConfig = serde_json::from_slice(&req.config).unwrap();

        let link = Box::new(MulticastLink::new(cfg.ip, cfg.port));

        self.links.lock().await.insert(req.id, link.clone());

        return Ok(link);
    }
    async fn remove(&mut self, id: edgeless_api::link::LinkInstanceId) -> anyhow::Result<()> {
        self.links.lock().await.remove(&id);
        Ok(())
    }
    async fn register_reader(&mut self, link_id: &edgeless_api::link::LinkInstanceId, reader: Box<dyn LinkWriter>) {
        *self.links.lock().await.get_mut(&link_id).unwrap().as_mut().reader.lock().await = Some(reader);
    }
    async fn get_writer(&mut self, link_id: &edgeless_api::link::LinkInstanceId) -> Option<Box<dyn LinkWriter>> {
        Some(self.links.lock().await.get_mut(&link_id).unwrap().as_mut().writer.clone())
    }
}

impl MulticastManager {
    pub fn new() -> MulticastManager {
        let pool_free: Vec<_> = std::ops::Range { start: 153, end: 253 }
            .into_iter()
            .map(|i| std::net::Ipv4Addr::new(224, 0, 0, i))
            .collect();

        MulticastManager {
            pool_free: pool_free,
            active: std::collections::HashMap::new(),
        }
    }

    pub fn new_link(&mut self, nodes: Vec<edgeless_api::function_instance::NodeId>) -> anyhow::Result<edgeless_api::link::LinkInstanceId> {
        let id = edgeless_api::link::LinkInstanceId(uuid::Uuid::new_v4());
        let ip = self.pool_free.pop();

        if let Some(ip) = ip {
            self.active.insert(
                id.clone(),
                ActiveMulticastLink {
                    addr: ip.clone(),
                    active_nodes: nodes,
                },
            );
            Ok(id)
        } else {
            Err(anyhow::anyhow!("No Capacity"))
        }
    }
}

#[async_trait::async_trait]
impl edgeless_api::link::LinkInstance for MulticastLink {
    async fn register_reader(&mut self, reader: Box<dyn LinkWriter>) -> anyhow::Result<()> {
        *self.reader.lock().await = Some(reader);
        Ok(())
    }
    async fn get_writer(&mut self) -> Option<Box<dyn LinkWriter>> {
        Some(self.writer.clone())
    }
}

#[async_trait::async_trait]
impl LinkWriter for MulticastWriter {
    async fn handle(&mut self, msg: Vec<u8>) {
        self.sender.send(msg).unwrap();
    }
}
