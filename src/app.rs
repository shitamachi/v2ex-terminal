pub struct App {
    http_client: reqwest::Client,
}

impl App {
    pub fn new() -> Self {
        App {
            http_client: reqwest::Client::new(),
        }
    }
}