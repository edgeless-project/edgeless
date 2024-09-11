#[derive(Debug, Clone)]
enum LinkDirection {
    Read,
    Write,
    BiDi,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct LinkProviderId(pub uuid::Uuid);

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct LinkInstanceId(pub uuid::Uuid);

// #[derive(Debug, Clone)]
// struct LinkId {
//     provider: LinkProviderId,
//     id: LinkInstanceId,
// }

#[derive(Debug, Clone)]
pub struct CreateLinkRequest {
    pub id: LinkInstanceId,
    pub provider: LinkProviderId,
    pub config: Vec<u8>,
    pub direction: LinkDirection,
}

#[async_trait::async_trait]
pub trait LinkInstanceAPI: LinkInstanceAPIClone + Send + Sync {
    async fn create(&mut self, req: CreateLinkRequest) -> anyhow::Result<()>;
    async fn remove(&mut self, id: LinkInstanceId) -> anyhow::Result<()>;
}

#[async_trait::async_trait]
pub trait LinkProvider: LinkProviderClone + Send + Sync {
    async fn create(&mut self, req: CreateLinkRequest) -> anyhow::Result<Box<dyn LinkInstance>>;
    async fn remove(&mut self, id: LinkInstanceId) -> anyhow::Result<()>;
    async fn register_reader(&mut self, link_id: &LinkInstanceId, reader: Box<dyn LinkWriter>);
    async fn get_writer(&mut self, link_id: &LinkInstanceId) -> Option<Box<dyn LinkWriter>>;
}

#[async_trait::async_trait]
pub trait LinkManager: LinkManagerClone + Send + Sync {
    async fn register_reader(&mut self, link_id: &LinkInstanceId, reader: Box<dyn LinkWriter>) -> anyhow::Result<()>;
    async fn get_writer(&mut self, link_id: &LinkInstanceId) -> Option<Box<dyn LinkWriter>>;
}

#[async_trait::async_trait]
pub trait LinkInstance: Send {
    async fn register_reader(&mut self, reader: Box<dyn LinkWriter>) -> anyhow::Result<()>;
    async fn get_writer(&mut self) -> Option<Box<dyn LinkWriter>>;
}

// pub trait LinkReader {

// }

#[async_trait::async_trait]
pub trait LinkWriter: Send {
    async fn handle(&mut self, msg: Vec<u8>);
}

// https://stackoverflow.com/a/30353928
pub trait LinkProviderClone {
    fn clone_box(&self) -> Box<dyn LinkProvider>;
}
impl<T> LinkProviderClone for T
where
    T: 'static + LinkProvider + Clone,
{
    fn clone_box(&self) -> Box<dyn LinkProvider> {
        Box::new(self.clone())
    }
}
impl Clone for Box<dyn LinkProvider> {
    fn clone(&self) -> Box<dyn LinkProvider> {
        self.clone_box()
    }
}
pub trait LinkManagerClone {
    fn clone_box(&self) -> Box<dyn LinkManager>;
}
impl<T> LinkManagerClone for T
where
    T: 'static + LinkManager + Clone,
{
    fn clone_box(&self) -> Box<dyn LinkManager> {
        Box::new(self.clone())
    }
}
impl Clone for Box<dyn LinkManager> {
    fn clone(&self) -> Box<dyn LinkManager> {
        self.clone_box()
    }
}

pub trait LinkInstanceAPIClone {
    fn clone_box(&self) -> Box<dyn LinkInstanceAPI>;
}
impl<T> LinkInstanceAPIClone for T
where
    T: 'static + LinkInstanceAPI + Clone,
{
    fn clone_box(&self) -> Box<dyn LinkInstanceAPI> {
        Box::new(self.clone())
    }
}
impl Clone for Box<dyn LinkInstanceAPI> {
    fn clone(&self) -> Box<dyn LinkInstanceAPI> {
        self.clone_box()
    }
}
