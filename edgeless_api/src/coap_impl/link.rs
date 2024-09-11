#[async_trait::async_trait]
impl crate::link::LinkInstanceAPI for crate::coap_impl::CoapClient {
    async fn create(&mut self, req: crate::link::CreateLinkRequest) -> anyhow::Result<()> {
        Ok(())
    }
    async fn remove(&mut self, id: crate::link::LinkInstanceId) -> anyhow::Result<()> {
        Ok(())
    }
}
