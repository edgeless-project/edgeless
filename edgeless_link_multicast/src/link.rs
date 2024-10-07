// SPDX-FileCopyrightText: © 2024 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

use futures::FutureExt;

#[derive(Clone)]
struct MulticastWriter {
    sender: tokio::sync::mpsc::UnboundedSender<Vec<u8>>,
}

#[derive(Clone)]
pub struct MulticastLink {
    reader: std::sync::Arc<tokio::sync::Mutex<Vec<Box<dyn edgeless_api::link::LinkWriter>>>>,
    writer: Box<MulticastWriter>,
    task: std::sync::Arc<tokio::sync::Mutex<tokio::task::JoinHandle<()>>>,
}

#[derive(Clone)]
pub struct MulticastProvider {
    links: std::sync::Arc<tokio::sync::Mutex<std::collections::HashMap<edgeless_api::link::LinkInstanceId, Box<MulticastLink>>>>,
}

impl MulticastLink {
    pub fn new(addr: std::net::Ipv4Addr, port: u16) -> Self {
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel::<Vec<u8>>();
        let reader: std::sync::Arc<tokio::sync::Mutex<Vec<Box<dyn edgeless_api::link::LinkWriter>>>> =
            std::sync::Arc::new(tokio::sync::Mutex::new(Vec::new()));

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
                            Ok((data_size, _sender)) => {
                                for r in reader_clone.lock().await.iter_mut() {
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

impl Drop for MulticastLink {
    fn drop(&mut self) {
        self.task.blocking_lock().abort();
    }
}

#[async_trait::async_trait]
impl edgeless_api::link::LinkProvider for MulticastProvider {
    fn class(&self) -> edgeless_api::link::LinkType {
        edgeless_api::link::LinkType("MULTICAST".to_string())
    }

    async fn create(&mut self, req: edgeless_api::link::CreateLinkRequest) -> anyhow::Result<Box<dyn edgeless_api::link::LinkInstance>> {
        let cfg: crate::common::MulticastConfig = serde_json::from_slice(&req.config).unwrap();

        let link = Box::new(MulticastLink::new(cfg.ip, cfg.port));

        self.links.lock().await.insert(req.id, link.clone());

        return Ok(link);
    }
    async fn remove(&mut self, id: edgeless_api::link::LinkInstanceId) -> anyhow::Result<()> {
        self.links.lock().await.remove(&id);
        Ok(())
    }
    async fn register_reader(&mut self, link_id: &edgeless_api::link::LinkInstanceId, reader: Box<dyn edgeless_api::link::LinkWriter>) {
        self.links
            .lock()
            .await
            .get_mut(&link_id)
            .unwrap()
            .as_mut()
            .reader
            .lock()
            .await
            .push(reader);
    }
    async fn get_writer(&mut self, link_id: &edgeless_api::link::LinkInstanceId) -> Option<Box<dyn edgeless_api::link::LinkWriter>> {
        Some(self.links.lock().await.get_mut(&link_id).unwrap().as_mut().writer.clone())
    }
}

#[async_trait::async_trait]
impl edgeless_api::link::LinkInstance for MulticastLink {
    async fn register_reader(&mut self, reader: Box<dyn edgeless_api::link::LinkWriter>) -> anyhow::Result<()> {
        self.reader.lock().await.push(reader);
        Ok(())
    }
    async fn get_writer(&mut self) -> Option<Box<dyn edgeless_api::link::LinkWriter>> {
        Some(self.writer.clone())
    }
}

#[async_trait::async_trait]
impl edgeless_api::link::LinkWriter for MulticastWriter {
    async fn handle(&mut self, msg: Vec<u8>) {
        self.sender.send(msg).unwrap();
    }
}
