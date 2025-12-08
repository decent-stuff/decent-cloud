"""URL discovery via sitemap parsing and deep crawl fallback."""

import logging
import xml.etree.ElementTree as ET
from urllib.parse import urljoin

import httpx
from crawl4ai import AsyncWebCrawler
from crawl4ai.deep_crawling import BFSDeepCrawlStrategy

from scraper.crawler import DEFAULT_BROWSER_CONFIG

logger = logging.getLogger(__name__)


class DiscoveryError(Exception):
    """Raised when URL discovery fails."""


# Common sitemap locations to try
SITEMAP_PATHS = [
    "/sitemap.xml",
    "/sitemap_index.xml",
    "/sitemap1.xml",
    "/robots.txt",  # Parse robots.txt for sitemap location
]


def parse_sitemap_xml(xml_content: str) -> list[str]:
    """Parse sitemap XML and extract URLs.

    Handles both sitemap index format (with <sitemap> tags) and
    urlset format (with <url> tags).

    Args:
        xml_content: Raw XML content from sitemap.

    Returns:
        List of URLs found in sitemap. Empty list if XML is invalid or empty.
    """
    if not xml_content:
        return []

    try:
        root = ET.fromstring(xml_content)
    except ET.ParseError:
        return []

    urls = []

    # Handle sitemap index format: <sitemapindex><sitemap><loc>
    for sitemap in root.findall(".//{*}sitemap/{*}loc"):
        if sitemap.text and sitemap.text.strip():
            urls.append(sitemap.text.strip())

    # Handle urlset format: <urlset><url><loc>
    for url in root.findall(".//{*}url/{*}loc"):
        if url.text and url.text.strip():
            urls.append(url.text.strip())

    return urls


async def discover_sitemap(base_url: str) -> list[str] | None:
    """Try to find and parse sitemap for a website.

    Tries common sitemap locations in order. If sitemap index is found,
    fetches and parses all child sitemaps.

    Args:
        base_url: Base URL of the website (e.g., "https://example.com").

    Returns:
        List of URLs from sitemap(s), or None if no sitemap found.

    Raises:
        DiscoveryError: If rate limited (429) or server error (5xx).
    """
    async with httpx.AsyncClient(timeout=30.0, follow_redirects=True) as client:
        for path in SITEMAP_PATHS:
            sitemap_url = urljoin(base_url, path)

            # Fetch sitemap content
            content, status = await _fetch_sitemap_content(client, sitemap_url, path)

            # Check for rate limiting or server errors
            if status == 429:
                raise DiscoveryError(f"Rate limited while fetching sitemap from {base_url}")
            if status and status >= 500:
                raise DiscoveryError(f"Server error ({status}) while fetching sitemap from {base_url}")

            if not content:
                continue

            # Parse URLs from sitemap
            urls = parse_sitemap_xml(content)
            if not urls:
                logger.debug(f"No URLs found in {sitemap_url}")
                continue

            # Check if this is a sitemap index (all URLs are .xml files)
            if all(url.endswith(".xml") for url in urls):
                logger.debug(f"Found sitemap index at {sitemap_url}, fetching {len(urls)} child sitemaps")
                return await _fetch_child_sitemaps(client, urls)

            logger.info(f"Found {len(urls)} URLs in sitemap: {sitemap_url}")
            return urls

    logger.warning(f"No sitemap found for {base_url}")
    return None


async def _fetch_sitemap_content(
    client: httpx.AsyncClient, sitemap_url: str, path: str
) -> tuple[str | None, int | None]:
    """Fetch sitemap content from URL.

    Args:
        client: HTTP client to use for fetching.
        sitemap_url: Full URL to sitemap.
        path: Original path (used for robots.txt detection).

    Returns:
        Tuple of (content, status_code). Content is None if fetch failed.
    """
    try:
        response = await client.get(sitemap_url)

        # Return status for rate limit/error detection
        if response.status_code == 429:
            return None, 429
        if response.status_code >= 500:
            return None, response.status_code
        if response.status_code != 200:
            logger.debug(f"Sitemap not found at {sitemap_url} (status: {response.status_code})")
            return None, response.status_code

        content = response.text

        # Check if we got an error page instead of XML
        if "429" in content[:500] or "Too Many Requests" in content[:500]:
            logger.warning(f"Rate limit page returned for {sitemap_url}")
            return None, 429

        # Special handling for robots.txt
        if path == "/robots.txt":
            sitemap_urls = _extract_sitemaps_from_robots(content)
            if not sitemap_urls:
                logger.debug(f"No sitemap URLs found in robots.txt at {sitemap_url}")
                return None, None

            # Fetch first sitemap from robots.txt
            first_sitemap_url = sitemap_urls[0]
            logger.debug(f"Found sitemap URL in robots.txt: {first_sitemap_url}")
            response = await client.get(first_sitemap_url)

            if response.status_code == 429:
                return None, 429
            if response.status_code >= 500:
                return None, response.status_code
            if response.status_code != 200:
                logger.warning(f"Failed to fetch sitemap from robots.txt: {first_sitemap_url} (status: {response.status_code})")
                return None, response.status_code

            content = response.text

            # Check for rate limit in response body
            if "429" in content[:500] or "Too Many Requests" in content[:500]:
                return None, 429

        return content, 200

    except httpx.HTTPError as e:
        logger.debug(f"HTTP error fetching {sitemap_url}: {e}")
        return None, None
    except Exception as e:
        logger.warning(f"Unexpected error fetching {sitemap_url}: {e}")
        return None, None


async def _fetch_child_sitemaps(
    client: httpx.AsyncClient, sitemap_urls: list[str]
) -> list[str] | None:
    """Fetch and parse child sitemaps from a sitemap index.

    Args:
        client: HTTP client to use for fetching.
        sitemap_urls: List of child sitemap URLs to fetch.

    Returns:
        Combined list of URLs from all child sitemaps, or None if all fetches failed.

    Raises:
        DiscoveryError: If rate limited.
    """
    all_urls = []

    for child_sitemap_url in sitemap_urls:
        try:
            response = await client.get(child_sitemap_url)

            if response.status_code == 429:
                raise DiscoveryError(f"Rate limited while fetching child sitemap: {child_sitemap_url}")

            if response.status_code != 200:
                logger.warning(f"Failed to fetch child sitemap: {child_sitemap_url} (status: {response.status_code})")
                continue

            # Check for rate limit in body
            if "429" in response.text[:500] or "Too Many Requests" in response.text[:500]:
                raise DiscoveryError(f"Rate limited while fetching child sitemap: {child_sitemap_url}")

            child_urls = parse_sitemap_xml(response.text)
            all_urls.extend(child_urls)
            logger.debug(f"Fetched {len(child_urls)} URLs from {child_sitemap_url}")

        except DiscoveryError:
            raise
        except httpx.HTTPError as e:
            logger.warning(f"HTTP error fetching child sitemap {child_sitemap_url}: {e}")
        except Exception as e:
            logger.error(f"Unexpected error fetching child sitemap {child_sitemap_url}: {e}")

    return all_urls if all_urls else None


def _extract_sitemaps_from_robots(robots_content: str) -> list[str]:
    """Extract sitemap URLs from robots.txt content.

    Args:
        robots_content: Content of robots.txt file.

    Returns:
        List of sitemap URLs found in robots.txt.
    """
    sitemaps = []
    for line in robots_content.splitlines():
        line = line.strip()
        if line.lower().startswith("sitemap:"):
            sitemap_url = line.split(":", 1)[1].strip()
            sitemaps.append(sitemap_url)
    return sitemaps


async def discover_via_crawl(
    base_url: str, max_depth: int = 2, max_pages: int = 50
) -> list[str]:
    """Discover URLs via deep crawl using BFS strategy.

    Fallback method when sitemap is not available. Uses Crawl4AI's
    BFSDeepCrawlStrategy to explore site structure.

    Args:
        base_url: Base URL to start crawling from.
        max_depth: Maximum depth to crawl (default: 2).
        max_pages: Maximum number of pages to crawl (default: 50).

    Returns:
        List of discovered URLs.

    Raises:
        DiscoveryError: If crawl completely fails.
    """
    from crawl4ai import CrawlerRunConfig

    # Deep crawl strategy is passed via CrawlerRunConfig
    config = CrawlerRunConfig(
        deep_crawl_strategy=BFSDeepCrawlStrategy(
            max_depth=max_depth,
            max_pages=max_pages,
            include_external=False,
        ),
        verbose=False,
    )

    try:
        async with AsyncWebCrawler(config=DEFAULT_BROWSER_CONFIG) as crawler:
            results = await crawler.arun(url=base_url, config=config)
    except Exception as e:
        raise DiscoveryError(f"Deep crawl failed for {base_url}: {e}") from e

    # Results can be a single result or list depending on deep crawl
    if not isinstance(results, list):
        results = [results]

    # Check for errors in results
    for result in results:
        if result.error_message:
            if "429" in result.error_message or "rate" in result.error_message.lower():
                raise DiscoveryError(f"Rate limited during deep crawl of {base_url}")

    # Extract URLs from results
    urls = []
    for result in results:
        if result.url:
            urls.append(result.url)

    if not urls:
        raise DiscoveryError(f"Deep crawl returned no URLs for {base_url}")

    logger.info(f"Deep crawl discovered {len(urls)} URLs from {base_url}")
    return urls
