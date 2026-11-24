# Griffin Search architecture

## Purpose

Griffin is intended for networks where general internet access is available but Google Search and/or Bing cannot be reached. It is not a crawler and does not attempt to impersonate or proxy either service. It queries independent providers directly from the local Rust backend.

## Request flow

1. The browser sends `GET /api/search?q=...&lang=fa|en&tab=all|web|wiki` to the local Axum server.
2. The backend validates the query and chooses the language (the UI detects Persian/English; the API also has an automatic fallback).
3. For `all` and `web`, configured web providers are queried concurrently:
   - DuckDuckGo HTML (built in)
   - SearXNG JSON (if `SEARXNG_URL` is set)
   - Brave Search API (if `BRAVE_SEARCH_API_KEY` is set)
4. For `all` and `wiki`, the appropriate Wikipedia API is queried (`fa.wikipedia.org` or `en.wikipedia.org`).
5. Results are normalized, unsafe non-HTTP(S) links are discarded, duplicates are removed, and provider state is returned with the result list.
6. The browser renders the results and a small knowledge panel when a Wikipedia result exists.

## Local safety defaults

- The server binds to `127.0.0.1` unless `GRIFFIN_HOST` is deliberately changed.
- No permissive CORS policy is enabled.
- API keys are read by the Rust process from environment variables and are never sent to the browser.
- `.env` is ignored by Git.
- Result links are restricted to HTTP and HTTPS before rendering.
- Recent searches stay in browser local storage; they are not uploaded to Griffin or stored in a server database.

## Provider behaviour

Providers are independent and may have different regional availability, rate limits, bot protections, or terms. A successful HTTP response with zero results is reported differently from a connection error. Adding a provider does not guarantee access if the user's network blocks that provider too.

## Future modules

An offline document index (PDF/HTML/local files), bookmarks, and a richer knowledge panel can be implemented as separate modules. Offline content must be visibly labelled as local archive content so it is not confused with current live web results.
