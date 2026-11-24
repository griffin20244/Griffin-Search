use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: String,
    pub lang: Option<String>,
    pub tab: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
    pub source: String,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProviderStatus {
    pub name: String,
    pub configured: bool,
    pub available: Option<bool>,
    pub detail: String,
}

#[derive(Debug, Serialize)]
pub struct SearchResponse {
    pub query: String,
    pub results: Vec<SearchResult>,
    pub elapsed_ms: u128,
    pub sources: Vec<String>,
    pub providers: Vec<ProviderStatus>,
    pub notice: Option<String>,
}

impl SearchResponse {
    pub fn empty(query: String, providers: Vec<ProviderStatus>, notice: Option<String>) -> Self {
        Self {
            query,
            results: Vec::new(),
            elapsed_ms: 0,
            sources: Vec::new(),
            providers,
            notice,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub ok: bool,
    pub app: String,
    pub version: String,
    pub providers: Vec<ProviderStatus>,
}
