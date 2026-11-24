# 🦅 Griffin Search

**Search beyond limits.** Griffin Search is a Persian-first, bilingual web search application that runs on your own computer at `http://127.0.0.1:8080`. It is designed for the exact situation where the internet works but Google Search and/or Bing are unreachable.

Griffin does **not** proxy Google or Bing. It sends search requests to independent providers:

- **DuckDuckGo HTML** — built-in general web search; no API key required.
- **SearXNG** — optional independent metasearch endpoint. You can run your own instance or configure a trusted one.
- **Brave Search API** — optional additional provider; requires an API key.
- **Wikipedia API** — Persian or English encyclopedia results, selected automatically from the query language.

Each provider is queried independently. If one provider fails, Griffin keeps results from the others and shows provider status. Availability depends on your network; Griffin does not bypass ISP blocks or guarantee that any particular third-party service is reachable.

## Features in v0.2.0

- Rust + Axum local server, bound to `127.0.0.1` by default.
- Responsive search interface inspired by familiar search engines, with Persian RTL and English LTR modes.
- Search tabs: All results, Web, Wikipedia.
- Automatic Persian/English query-language detection, plus a manual interface-language switch.
- Parallel requests to configured web providers and Wikipedia.
- Provider status, result-source labels, safe HTTP(S)-only result links, duplicate removal.
- Recent-search history stored only in the browser's local storage.
- Optional SearXNG and Brave Search configuration through environment variables.
- Unit tests and GitHub Actions CI.

## Run on Windows 10/11

1. Install the stable Rust toolchain from <https://rustup.rs/>.
2. Download or clone this repository.
3. In PowerShell, open the repository folder and run:

   ```powershell
   cargo run --release
   ```

4. Open <http://127.0.0.1:8080> in your browser.
5. To stop the server, return to the terminal and press `Ctrl+C`.

The first build downloads Rust dependencies, so it needs internet access. Subsequent starts use the local build cache.

## Configure alternate search providers

Copy `.env.example` to `.env` and edit it locally. Griffin loads `.env` on startup.

```dotenv
GRIFFIN_HOST=127.0.0.1
GRIFFIN_PORT=8080
SEARXNG_URL=https://your-searxng-instance.example
BRAVE_SEARCH_API_KEY=your_private_api_key
```

- `SEARXNG_URL` is the base URL of a SearXNG instance (do not add `/search`).
- `BRAVE_SEARCH_API_KEY` is optional. Get a key from <https://brave.com/search/api/>.
- Never commit `.env`, API keys, or other secrets. `.gitignore` excludes `.env`.
- After changing `.env`, restart Griffin.

For best resilience, use a SearXNG instance you control or trust. Public instances can be rate-limited, unavailable, or blocked on a particular network. See <https://docs.searxng.org/>.

## Architecture

```text
Browser (localhost)
       |
       v
Rust / Axum API
       |
       +---- DuckDuckGo HTML search
       +---- SearXNG JSON API (optional)
       +---- Brave Search API (optional)
       +---- Wikipedia API (fa/en)
       |
       v
Normalize + de-duplicate + validate HTTP(S) links
       |
       v
JSON results + provider status -> Persian/English UI
```

More detail: [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md).

## Important limitations

- Griffin is a **search interface and aggregator**, not a crawler with its own global index. Live web results come from the configured independent providers.
- It cannot retrieve new internet pages while offline. A future offline archive/index can be added as a separate feature and will be clearly distinguished from live results.
- If all independent providers are blocked or unreachable from your network, live web search will fail. Configure a reachable SearXNG instance or another supported provider.
- Search results, availability, ranking, and rate limits are controlled by the third-party providers.
- The optional Brave API may have usage limits or costs under its own terms.

## Development

```powershell
cargo fmt --all -- --check
cargo test
cargo run
```

CI runs formatting, tests, and a release build on pushes and pull requests.