#[derive(Clone, Debug)]
pub struct ResponseError {
    pub summary: String,
    pub detail: Option<String>,
}
