#[derive(Clone, Debug)]
pub struct ResponseError {
    pub summary: String,
    pub detail: Option<String>,
}

impl std::fmt::Display for ResponseError {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match &self.detail {
            Some(detail) => write!(fmt, "{} [detail: {}]", self.summary, detail),
            None => write!(fmt, "{}", self.summary),
        }
    }
}
