# Griffin Search

A Persian-first, bilingual web search app that runs locally at `http://127.0.0.1:8080`. It is designed for situations where you have internet access but Google Search and/or Bing are unavailable.

## Project status

The repository is being initialized with the first working MVP. The app uses independent providers rather than redirecting users to Google or Bing:

- DuckDuckGo HTML results (primary built-in web provider)
- SearXNG JSON endpoint (optional, configurable; useful as an alternate metasearch gateway)
- Wikipedia API (knowledge results and a dependable fallback for encyclopedia searches)
- Optional Brave Search API key (additional independent provider)

No provider can be guaranteed reachable on every network. Griffin reports provider failures and tries the next configured source; it does not bypass ISP/network restrictions.

## Run locally

Requirements: stable Rust toolchain.

```powershell
cargo run --release
```

Open <http://127.0.0.1:8080>.

Optional configuration can be set with environment variables or a local `.env` file (see `.env.example`):

- `GRIFFIN_HOST` (default `127.0.0.1`)
- `GRIFFIN_PORT` (default `8080`)
- `SEARXNG_URL` (base URL of your own or a trusted SearXNG instance)
- `BRAVE_SEARCH_API_KEY` (optional; never commit your key)

## Notes

- Online search requires an internet connection.
- Wikipedia searches can work even when the general web provider is unavailable, provided Wikipedia is reachable.
- A future offline archive/index is intentionally separate from online web search; it will be added without pretending that offline data is live web results.
