#[derive(Clone)]
pub struct LinkInstanceClient {
    client: bool,
}

#[async_trait::async_trait]
impl crate::link::LinkInstanceAPI for LinkInstanceClient {
    async fn create(&mut self, req: crate::link::CreateLinkRequest) -> anyhow::Result<()> {
        Ok(())
    }
    async fn remove(&mut self, id: crate::link::LinkInstanceId) -> anyhow::Result<()> {
        Ok(())
    }
}

impl LinkInstanceClient {
    pub async fn new(server_addr: &str, retry_interval: Option<u64>) -> Self {
        Self { client: true }
    }
}
