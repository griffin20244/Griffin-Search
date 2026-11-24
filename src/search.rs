use crate::{models::{ProviderStatus, SearchResponse, SearchResult}, AppState};
use scraper::{Html, Selector};
use serde_json::Value;
use std::collections::HashSet;
use url::Url;

pub fn validate_query(query: &str) -> Result<(), &'static str> {
    let query = query.trim();
    if query.is_empty() {
        return Err("عبارت جست‌وجو را وارد کن.");
    }
    if query.chars().count() > 200 {
        return Err("عبارت جست‌وجو بیش از حد طولانی است (حداکثر ۲۰۰ نویسه).");
    }
    Ok(())
}

pub fn infer_language(query: &str) -> &'static str {
    if query.chars().any(|ch| ('\u{0600}'..='\u{06FF}').contains(&ch)) {
        "fa"
    } else {
        "en"
    }
}

pub async fn run_search(state: &AppState, query: &str, lang: &str, tab: &str) -> SearchResponse {
    let web_future = async {
        if tab == "wiki" {
            (Vec::new(), Vec::new())
        } else {
            web_search(state, query, lang).await
        }
    };
    let wiki_future = async {
        if tab == "web" {
            (Vec::new(), None)
        } else {
            match wikipedia_search(&state.client, query, lang).await {
                Ok(results) => {
                    let available = !results.is_empty();
                    (
                        results,
                        Some(ProviderStatus {
                            name: "Wikipedia".to_owned(),
                            configured: true,
                            available: Some(available),
                            detail: if available {
                                "نتایج دریافت شد"
                            } else {
                                "نتیجه‌ای پیدا نشد"
                            }
                            .to_owned(),
                        }),
                    )
                }
                Err(_) => (
                    Vec::new(),
                    Some(ProviderStatus {
                        name: "Wikipedia".to_owned(),
                        configured: true,
                        available: Some(false),
                        detail: "اتصال به ویکی‌پدیا برقرار نشد".to_owned(),
                    }),
                ),
            }
        }
    };

    let ((mut web_results, mut providers), (mut wiki_results, wiki_status)) =
        tokio::join!(web_future, wiki_future);

    if let Some(status) = wiki_status {
        providers.push(status);
    }

    let mut results = Vec::new();
    if tab != "web" {
        results.append(&mut wiki_results);
    }
    if tab != "wiki" {
        results.append(&mut web_results);
    }

    let mut seen = HashSet::new();
    results.retain(|result| {
        if !is_http_url(&result.url) {
            return false;
        }
        let key = result.url.trim_end_matches('/').to_lowercase();
        seen.insert(key)
    });

    let mut sources = Vec::new();
    for result in &results {
        if !sources.iter().any(|source| source == &result.source) {
            sources.push(result.source.clone());
        }
    }

    let notice = if results.is_empty() {
        Some(if providers.iter().any(|provider| provider.available == Some(false)) {
            "از هیچ‌کدام از منابع انتخاب‌شده نتیجه‌ای دریافت نشد. دسترسی شبکه را بررسی کن یا یک SearXNG در فایل .env تنظیم کن.".to_owned()
        } else {
            "برای این عبارت نتیجه‌ای پیدا نشد. عبارت دیگری را امتحان کن.".to_owned()
        })
    } else {
        None
    };

    SearchResponse {
        query: query.to_owned(),
        results,
        elapsed_ms: 0,
        sources,
        providers,
        notice,
    }
}

async fn web_search(state: &AppState, query: &str, lang: &str) -> (Vec<SearchResult>, Vec<ProviderStatus>) {
    let ddg_future = duckduckgo_search(&state.client, query);
    let searx_future = async {
        match state.searxng_url.as_deref() {
            Some(base) => searxng_search(&state.client, base, query, lang).await,
            None => Ok(Vec::new()),
        }
    };
    let brave_future = async {
        match state.brave_api_key.as_deref() {
            Some(key) => brave_search(&state.client, key, query, lang).await,
            None => Ok(Vec::new()),
        }
    };

    let (ddg, searx, brave) = tokio::join!(ddg_future, searx_future, brave_future);
    let mut results = Vec::new();
    let mut statuses = Vec::new();

    append_provider("DuckDuckGo", ddg, true, &mut results, &mut statuses);
    append_provider("SearXNG", searx, state.searxng_url.is_some(), &mut results, &mut statuses);
    append_provider("Brave Search", brave, state.brave_api_key.is_some(), &mut results, &mut statuses);

    (results, statuses)
}

fn append_provider(
    name: &str,
    result: anyhow::Result<Vec<SearchResult>>,
    configured: bool,
    results: &mut Vec<SearchResult>,
    statuses: &mut Vec<ProviderStatus>,
) {
    if !configured {
        return;
    }
    match result {
        Ok(mut found) => {
            let count = found.len();
            results.append(&mut found);
            statuses.push(ProviderStatus {
                name: name.to_owned(),
                configured,
                available: Some(true),
                detail: if count == 0 { "پاسخ دریافت شد، اما نتیجه‌ای پیدا نشد" } else { "نتایج دریافت شد" }.to_owned(),
            });
        }
        Err(_) => statuses.push(ProviderStatus {
            name: name.to_owned(),
            configured,
            available: Some(false),
            detail: "اتصال برقرار نشد یا سرویس پاسخ معتبر نداد".to_owned(),
        }),
    }
}

async fn duckduckgo_search(client: &reqwest::Client, query: &str) -> anyhow::Result<Vec<SearchResult>> {
    let response = client
        .get("https://html.duckduckgo.com/html/")
        .query(&[("q", query)])
        .header("Accept", "text/html,application/xhtml+xml")
        .header("Accept-Language", "fa,en;q=0.8")
        .send()
        .await?
        .error_for_status()?;
    let body = response.text().await?;
    let document = Html::parse_document(&body);
    let result_selector = Selector::parse(".result").expect("valid CSS selector");
    let title_selector = Selector::parse(".result__a").expect("valid CSS selector");
    let snippet_selector = Selector::parse(".result__snippet").expect("valid CSS selector");

    let mut results = Vec::new();
    for node in document.select(&result_selector).take(12) {
        let Some(anchor) = node.select(&title_selector).next() else { continue };
        let title = clean_text(&anchor.text().collect::<Vec<_>>().join(" "));
        let href = anchor.value().attr("href").unwrap_or_default();
        let url = unwrap_duckduckgo_url(href).unwrap_or_else(|| href.to_owned());
        let snippet = node
            .select(&snippet_selector)
            .next()
            .map(|item| clean_text(&item.text().collect::<Vec<_>>().join(" ")))
            .unwrap_or_default();

        if !title.is_empty() && is_http_url(&url) {
            results.push(SearchResult {
                title,
                url,
                snippet,
                source: "DuckDuckGo".to_owned(),
                kind: "web".to_owned(),
            });
        }
    }
    Ok(results)
}

fn unwrap_duckduckgo_url(href: &str) -> Option<String> {
    let absolute = if href.starts_with("//") {
        format!("https:{href}")
    } else if href.starts_with('/') {
        format!("https://duckduckgo.com{href}")
    } else {
        href.to_owned()
    };
    let parsed = Url::parse(&absolute).ok()?;
    if parsed.domain().is_some_and(|domain| domain.ends_with("duckduckgo.com")) {
        if let Some((_, value)) = parsed.query_pairs().find(|(key, _)| key == "uddg") {
            return Some(value.into_owned());
        }
    }
    Some(absolute)
}

async fn searxng_search(
    client: &reqwest::Client,
    base: &str,
    query: &str,
    lang: &str,
) -> anyhow::Result<Vec<SearchResult>> {
    let endpoint = format!("{}/search", base.trim_end_matches('/'));
    let response: Value = client
        .get(endpoint)
        .query(&[("q", query), ("format", "json"), ("language", lang)])
        .header("Accept", "application/json")
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let mut results = Vec::new();
    if let Some(items) = response.get("results").and_then(Value::as_array) {
        for item in items.iter().take(15) {
            let title = item.get("title").and_then(Value::as_str).unwrap_or_default();
            let url = item.get("url").and_then(Value::as_str).unwrap_or_default();
            let snippet = item.get("content").and_then(Value::as_str).unwrap_or_default();
            if !title.trim().is_empty() && is_http_url(url) {
                results.push(SearchResult {
                    title: clean_text(title),
                    url: url.to_owned(),
                    snippet: clean_text(snippet),
                    source: "SearXNG".to_owned(),
                    kind: "web".to_owned(),
                });
            }
        }
    }
    Ok(results)
}

async fn brave_search(
    client: &reqwest::Client,
    api_key: &str,
    query: &str,
    lang: &str,
) -> anyhow::Result<Vec<SearchResult>> {
    let response: Value = client
        .get("https://api.search.brave.com/res/v1/web/search")
        .query(&[("q", query), ("count", "10"), ("search_lang", lang)])
        .header("Accept", "application/json")
        .header("X-Subscription-Token", api_key)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let mut results = Vec::new();
    if let Some(items) = response.get("web").and_then(|web| web.get("results")).and_then(Value::as_array) {
        for item in items.iter().take(10) {
            let title = item.get("title").and_then(Value::as_str).unwrap_or_default();
            let url = item.get("url").and_then(Value::as_str).unwrap_or_default();
            let snippet = item.get("description").and_then(Value::as_str).unwrap_or_default();
            if !title.trim().is_empty() && is_http_url(url) {
                results.push(SearchResult {
                    title: clean_text(title),
                    url: url.to_owned(),
                    snippet: clean_text(snippet),
                    source: "Brave Search".to_owned(),
                    kind: "web".to_owned(),
                });
            }
        }
    }
    Ok(results)
}

async fn wikipedia_search(
    client: &reqwest::Client,
    query: &str,
    lang: &str,
) -> anyhow::Result<Vec<SearchResult>> {
    let language = if lang == "fa" { "fa" } else { "en" };
    let endpoint = format!("https://{language}.wikipedia.org/w/api.php");
    let response: Value = client
        .get(endpoint)
        .query(&[
            ("action", "query"),
            ("list", "search"),
            ("srsearch", query),
            ("srprop", "snippet"),
            ("srlimit", "8"),
            ("format", "json"),
            ("utf8", "1"),
        ])
        .header("Accept", "application/json")
        .header("Accept-Language", if lang == "fa" { "fa,en;q=0.6" } else { "en,fa;q=0.6" })
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let mut results = Vec::new();
    if let Some(items) = response.get("query").and_then(|query| query.get("search")).and_then(Value::as_array) {
        for item in items.iter().take(8) {
            let title = item.get("title").and_then(Value::as_str).unwrap_or_default();
            let snippet = item.get("snippet").and_then(Value::as_str).unwrap_or_default();
            if !title.trim().is_empty() {
                let slug = title.replace(' ', "_");
                results.push(SearchResult {
                    title: title.to_owned(),
                    url: format!("https://{language}.wikipedia.org/wiki/{}", urlencoding::encode(&slug)),
                    snippet: clean_html_fragment(snippet),
                    source: if language == "fa" { "ویکی‌پدیای فارسی" } else { "Wikipedia" }.to_owned(),
                    kind: "wiki".to_owned(),
                });
            }
        }
    }
    Ok(results)
}

fn clean_html_fragment(value: &str) -> String {
    let fragment = Html::parse_fragment(value);
    clean_text(&fragment.root_element().text().collect::<Vec<_>>().join(" "))
}

fn clean_text(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn is_http_url(value: &str) -> bool {
    Url::parse(value)
        .map(|url| matches!(url.scheme(), "http" | "https") && url.host_str().is_some())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infers_persian_and_english() {
        assert_eq!(infer_language("محمدرضا شاه پهلوی"), "fa");
        assert_eq!(infer_language("Rust programming"), "en");
    }

    #[test]
    fn rejects_empty_and_oversized_queries() {
        assert!(validate_query("  ").is_err());
        assert!(validate_query(&"a".repeat(201)).is_err());
        assert!(validate_query("محمدرضا شاه پهلوی").is_ok());
    }

    #[test]
    fn only_http_links_are_allowed() {
        assert!(is_http_url("https://fa.wikipedia.org/wiki/Rust"));
        assert!(is_http_url("http://example.org"));
        assert!(!is_http_url("javascript:alert(1)"));
        assert!(!is_http_url("file:///etc/passwd"));
    }

    #[test]
    fn unwraps_duckduckgo_redirect() {
        let target = "https%3A%2F%2Fexample.org%2Farticle";
        let link = format!("https://duckduckgo.com/l/?uddg={target}&rut=abc");
        assert_eq!(unwrap_duckduckgo_url(&link).as_deref(), Some("https://example.org/article"));
    }
}
