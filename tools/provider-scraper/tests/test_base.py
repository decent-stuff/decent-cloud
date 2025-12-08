"""Tests for async base scraper class."""

import pytest
from pathlib import Path
from unittest.mock import AsyncMock, Mock, patch

from scraper.base import BaseScraper, DocsScrapeError
from scraper.models import Offering


class ConcreteTestScraper(BaseScraper):
    """Concrete implementation for testing."""

    provider_name = "Test Provider"
    provider_website = "https://test.com"
    docs_base_url = "https://docs.test.com"

    async def scrape_offerings(self) -> list[Offering]:
        """Return test offerings."""
        return [
            Offering(
                offering_id="test-1",
                offer_name="Test Server",
                currency="USD",
                monthly_price=10.0,
                product_type="compute",
                datacenter_country="US",
                datacenter_city="New York",
            )
        ]


@pytest.fixture
def temp_output_dir(tmp_path):
    """Create temp output directory."""
    return tmp_path / "test-provider"


@pytest.fixture
def scraper(temp_output_dir):
    """Create test scraper instance."""
    return ConcreteTestScraper(output_dir=temp_output_dir)


class TestInitialization:
    """Test scraper initialization."""

    def test_init_with_custom_output_dir(self, temp_output_dir):
        scraper = ConcreteTestScraper(output_dir=temp_output_dir)
        assert scraper.output_dir == temp_output_dir
        assert scraper.archive is not None

    def test_init_with_default_output_dir(self):
        scraper = ConcreteTestScraper()
        assert scraper.output_dir == Path("output/test-provider")
        assert scraper.archive is not None

    def test_provider_id(self, scraper):
        assert scraper.provider_id == "test-provider"

    def test_provider_id_with_spaces(self):
        class MultiWordProvider(BaseScraper):
            provider_name = "Multi Word Provider"
            provider_website = "https://example.com"

            async def scrape_offerings(self):
                return []

        scraper = MultiWordProvider()
        assert scraper.provider_id == "multi-word-provider"


class TestDocURLDiscovery:
    """Test documentation URL discovery."""

    @pytest.mark.asyncio
    async def test_discover_doc_urls_uses_docs_base_url(self, scraper):
        with patch("scraper.base.discover_sitemap", new_callable=AsyncMock) as mock_sitemap:
            mock_sitemap.return_value = [
                "https://docs.test.com/guide",
                "https://docs.test.com/api",
            ]

            urls = await scraper.discover_doc_urls()

            # Verify docs_base_url was used
            mock_sitemap.assert_called_once_with("https://docs.test.com")
            assert len(urls) == 2

    @pytest.mark.asyncio
    async def test_discover_doc_urls_falls_back_to_provider_website(self, temp_output_dir):
        # Scraper without docs_base_url
        class NoDocsURLScraper(ConcreteTestScraper):
            docs_base_url = None

        scraper = NoDocsURLScraper(output_dir=temp_output_dir)

        with patch("scraper.base.discover_sitemap", new_callable=AsyncMock) as mock_sitemap:
            mock_sitemap.return_value = ["https://test.com/help"]

            urls = await scraper.discover_doc_urls()

            # Verify provider_website was used
            mock_sitemap.assert_called_once_with("https://test.com")
            assert len(urls) == 1

    @pytest.mark.asyncio
    async def test_discover_doc_urls_uses_sitemap_when_available(self, scraper):
        with patch("scraper.base.discover_sitemap", new_callable=AsyncMock) as mock_sitemap:
            mock_sitemap.return_value = [
                "https://docs.test.com/page1",
                "https://docs.test.com/page2",
            ]

            urls = await scraper.discover_doc_urls()

            mock_sitemap.assert_called_once()
            assert len(urls) == 2

    @pytest.mark.asyncio
    async def test_discover_doc_urls_falls_back_to_deep_crawl(self, scraper):
        with (
            patch("scraper.base.discover_sitemap", new_callable=AsyncMock) as mock_sitemap,
            patch("scraper.base.discover_via_crawl", new_callable=AsyncMock) as mock_crawl,
        ):
            mock_sitemap.return_value = None  # No sitemap
            mock_crawl.return_value = [
                "https://docs.test.com/page1",
                "https://docs.test.com/page2",
            ]

            urls = await scraper.discover_doc_urls()

            mock_sitemap.assert_called_once()
            mock_crawl.assert_called_once_with("https://docs.test.com", max_depth=2, max_pages=50)
            assert len(urls) == 2


class TestFilterDocURLs:
    """Test URL filtering logic."""

    def test_filter_doc_urls_keeps_urls_from_docs_base(self, scraper):
        """Filter keeps URLs that start with docs_base_url."""
        urls = [
            "https://docs.test.com/getting-started",
            "https://other.com/blog/post",
            "https://docs.test.com/api",
        ]
        filtered = scraper._filter_doc_urls(urls)
        assert len(filtered) == 2
        assert "https://docs.test.com/getting-started" in filtered
        assert "https://docs.test.com/api" in filtered

    def test_filter_doc_urls_removes_external_urls(self, scraper):
        """Filter removes URLs not from docs_base_url domain."""
        urls = [
            "https://docs.test.com/page",
            "https://external.com/help/page",
            "https://test.com/docs/api",  # Different subdomain
        ]
        filtered = scraper._filter_doc_urls(urls)
        assert len(filtered) == 1
        assert "https://docs.test.com/page" in filtered

    def test_filter_doc_urls_deduplicates(self, scraper):
        """Filter removes duplicate URLs."""
        urls = [
            "https://docs.test.com/page",
            "https://docs.test.com/page",
            "https://docs.test.com/page/",  # Trailing slash variant
        ]
        filtered = scraper._filter_doc_urls(urls)
        assert len(filtered) == 1

    def test_filter_doc_urls_empty_list(self, scraper):
        filtered = scraper._filter_doc_urls([])
        assert filtered == []


class TestExtractTopic:
    """Test topic extraction from URL/title."""

    def test_extract_topic_uses_title_when_available(self, scraper):
        topic = scraper._extract_topic("https://test.com/docs/page", "Getting Started")
        assert topic == "Getting Started"

    def test_extract_topic_uses_url_when_no_title(self, scraper):
        topic = scraper._extract_topic("https://test.com/docs/getting-started", "")
        assert topic == "getting-started"

    def test_extract_topic_uses_url_when_title_too_long(self, scraper):
        long_title = "A" * 150  # > 100 chars
        topic = scraper._extract_topic("https://test.com/docs/page", long_title)
        assert topic == "page"

    def test_extract_topic_uses_index_for_root_url(self, scraper):
        topic = scraper._extract_topic("https://test.com", "")
        assert topic == "index"

    def test_extract_topic_extracts_last_path_segment(self, scraper):
        topic = scraper._extract_topic("https://test.com/docs/api/reference", "")
        assert topic == "reference"


class TestScrapeOfferings:
    """Test offerings scraping."""

    @pytest.mark.asyncio
    async def test_scrape_offerings_is_abstract(self):
        # Verify that BaseScraper cannot be instantiated without implementing scrape_offerings
        with pytest.raises(TypeError, match="Can't instantiate abstract class.*scrape_offerings"):
            class NoImplementation(BaseScraper):
                provider_name = "Test"
                provider_website = "https://test.com"

            NoImplementation()


class TestScrapeDocs:
    """Test documentation scraping."""

    @pytest.mark.asyncio
    async def test_scrape_docs_raises_when_no_urls(self, scraper):
        """scrape_docs raises DocsScrapeError when no URLs found."""
        with patch.object(scraper, "discover_doc_urls", new_callable=AsyncMock) as mock_discover:
            mock_discover.return_value = []

            with pytest.raises(DocsScrapeError, match="No doc URLs found"):
                await scraper.scrape_docs()

    @pytest.mark.asyncio
    async def test_scrape_docs_crawls_and_writes_changed_content(self, scraper):
        mock_result = Mock()
        mock_result.success = True
        mock_result.error_message = None
        mock_result.markdown.fit_markdown = "# Test Content"
        mock_result.markdown.raw_markdown = "# Raw Content"
        mock_result.metadata = {"title": "Test Page", "etag": "abc123"}

        with (
            patch.object(scraper, "discover_doc_urls", new_callable=AsyncMock) as mock_discover,
            patch("scraper.base.AsyncWebCrawler") as mock_crawler_class,
        ):
            mock_discover.return_value = ["https://docs.test.com/page1"]

            mock_crawler = AsyncMock()
            mock_crawler.arun = AsyncMock(return_value=mock_result)
            mock_crawler_class.return_value.__aenter__.return_value = mock_crawler

            # Mock archive methods
            scraper.archive.has_changed = Mock(return_value=True)
            scraper.archive.write = Mock(return_value="test-page.md")

            count = await scraper.scrape_docs()

            assert count == 1
            scraper.archive.has_changed.assert_called_once()
            scraper.archive.write.assert_called_once_with(
                "https://docs.test.com/page1",
                "# Test Content",
                "Test Page",
                "abc123",
            )

    @pytest.mark.asyncio
    async def test_scrape_docs_skips_unchanged_content(self, scraper):
        mock_result = Mock()
        mock_result.success = True
        mock_result.error_message = None
        mock_result.markdown.fit_markdown = "# Test Content"
        mock_result.metadata = {"title": "Test Page"}

        with (
            patch.object(scraper, "discover_doc_urls", new_callable=AsyncMock) as mock_discover,
            patch("scraper.base.AsyncWebCrawler") as mock_crawler_class,
        ):
            mock_discover.return_value = ["https://docs.test.com/page1"]

            mock_crawler = AsyncMock()
            mock_crawler.arun = AsyncMock(return_value=mock_result)
            mock_crawler_class.return_value.__aenter__.return_value = mock_crawler

            # Mock archive to say content unchanged
            scraper.archive.has_changed = Mock(return_value=False)
            scraper.archive.write = Mock()

            count = await scraper.scrape_docs()

            assert count == 0
            scraper.archive.has_changed.assert_called_once()
            scraper.archive.write.assert_not_called()

    @pytest.mark.asyncio
    async def test_scrape_docs_raises_when_all_pages_fail(self, scraper):
        """scrape_docs raises DocsScrapeError when all pages fail."""
        mock_result = Mock()
        mock_result.success = False
        mock_result.error_message = "Connection timeout"

        with (
            patch.object(scraper, "discover_doc_urls", new_callable=AsyncMock) as mock_discover,
            patch("scraper.base.AsyncWebCrawler") as mock_crawler_class,
        ):
            mock_discover.return_value = ["https://docs.test.com/page1"]

            mock_crawler = AsyncMock()
            mock_crawler.arun = AsyncMock(return_value=mock_result)
            mock_crawler_class.return_value.__aenter__.return_value = mock_crawler

            scraper.archive.write = Mock()

            with pytest.raises(DocsScrapeError, match="All 1 doc pages failed"):
                await scraper.scrape_docs()

    @pytest.mark.asyncio
    async def test_scrape_docs_raises_when_no_markdown_content(self, scraper):
        """scrape_docs raises when all pages have no content."""
        mock_result = Mock()
        mock_result.success = True
        mock_result.error_message = None
        mock_result.markdown.fit_markdown = None
        mock_result.markdown.raw_markdown = None

        with (
            patch.object(scraper, "discover_doc_urls", new_callable=AsyncMock) as mock_discover,
            patch("scraper.base.AsyncWebCrawler") as mock_crawler_class,
        ):
            mock_discover.return_value = ["https://docs.test.com/page1"]

            mock_crawler = AsyncMock()
            mock_crawler.arun = AsyncMock(return_value=mock_result)
            mock_crawler_class.return_value.__aenter__.return_value = mock_crawler

            scraper.archive.write = Mock()

            with pytest.raises(DocsScrapeError, match="All 1 doc pages failed"):
                await scraper.scrape_docs()

    @pytest.mark.asyncio
    async def test_scrape_docs_uses_raw_markdown_when_no_fit_markdown(self, scraper):
        mock_result = Mock()
        mock_result.success = True
        mock_result.error_message = None
        mock_result.markdown.fit_markdown = None
        mock_result.markdown.raw_markdown = "# Raw Content"
        mock_result.metadata = {"title": "Test Page"}

        with (
            patch.object(scraper, "discover_doc_urls", new_callable=AsyncMock) as mock_discover,
            patch("scraper.base.AsyncWebCrawler") as mock_crawler_class,
        ):
            mock_discover.return_value = ["https://docs.test.com/page1"]

            mock_crawler = AsyncMock()
            mock_crawler.arun = AsyncMock(return_value=mock_result)
            mock_crawler_class.return_value.__aenter__.return_value = mock_crawler

            scraper.archive.has_changed = Mock(return_value=True)
            scraper.archive.write = Mock(return_value="test-page.md")

            count = await scraper.scrape_docs()

            assert count == 1
            scraper.archive.write.assert_called_once_with(
                "https://docs.test.com/page1",
                "# Raw Content",
                "Test Page",
                None,
            )

    @pytest.mark.asyncio
    async def test_scrape_docs_continues_after_single_exception(self, scraper):
        """scrape_docs continues processing after exception on single page."""
        with (
            patch.object(scraper, "discover_doc_urls", new_callable=AsyncMock) as mock_discover,
            patch("scraper.base.AsyncWebCrawler") as mock_crawler_class,
        ):
            mock_discover.return_value = [
                "https://docs.test.com/page1",
                "https://docs.test.com/page2",
            ]

            mock_crawler = AsyncMock()
            # First URL throws, second succeeds
            mock_result = Mock()
            mock_result.success = True
            mock_result.error_message = None
            mock_result.markdown.fit_markdown = "# Content"
            mock_result.metadata = {"title": "Page 2"}

            mock_crawler.arun = AsyncMock(side_effect=[Exception("Network error"), mock_result])
            mock_crawler_class.return_value.__aenter__.return_value = mock_crawler

            scraper.archive.has_changed = Mock(return_value=True)
            scraper.archive.write = Mock(return_value="page2.md")

            count = await scraper.scrape_docs()

            # Should continue after exception and process page2
            assert count == 1


class TestRun:
    """Test full scraping workflow."""

    @pytest.mark.asyncio
    async def test_run_writes_csv_and_scrapes_docs(self, scraper):
        with (
            patch.object(scraper, "scrape_offerings", new_callable=AsyncMock) as mock_offerings,
            patch.object(scraper, "scrape_docs", new_callable=AsyncMock) as mock_docs,
        ):
            mock_offerings.return_value = [
                Offering(
                    offering_id="test-1",
                    offer_name="Server",
                    currency="USD",
                    monthly_price=10.0,
                    product_type="compute",
                    datacenter_country="US",
                    datacenter_city="NYC",
                )
            ]
            mock_docs.return_value = 5

            csv_path, docs_count = await scraper.run()

            assert csv_path == scraper.output_dir / "offerings.csv"
            assert csv_path.exists()
            assert docs_count == 5

            mock_offerings.assert_called_once()
            mock_docs.assert_called_once()

    @pytest.mark.asyncio
    async def test_run_creates_valid_csv(self, scraper):
        with patch.object(scraper, "scrape_docs", new_callable=AsyncMock) as mock_docs:
            mock_docs.return_value = 0

            csv_path, _ = await scraper.run()

            # Verify CSV was created
            assert csv_path.exists()

            # Read and verify CSV content
            content = csv_path.read_text()
            lines = content.strip().split("\n")
            assert len(lines) == 2  # Header + 1 offering
            assert lines[0].startswith("offering_id,offer_name")
            assert "test-1" in lines[1]

    @pytest.mark.asyncio
    async def test_run_skip_offerings_on_failure(self, scraper):
        """Test run with skip_offerings=True continues on failure."""
        with (
            patch.object(scraper, "scrape_offerings", new_callable=AsyncMock) as mock_offerings,
            patch.object(scraper, "scrape_docs", new_callable=AsyncMock) as mock_docs,
        ):
            mock_offerings.side_effect = RuntimeError("API error")
            mock_docs.return_value = 3

            csv_path, docs_count = await scraper.run(skip_offerings=True)

            assert csv_path is None
            assert docs_count == 3

    @pytest.mark.asyncio
    async def test_run_skip_docs_on_failure(self, scraper):
        """Test run with skip_docs=True continues on failure."""
        with (
            patch.object(scraper, "scrape_offerings", new_callable=AsyncMock) as mock_offerings,
            patch.object(scraper, "scrape_docs", new_callable=AsyncMock) as mock_docs,
        ):
            mock_offerings.return_value = [
                Offering(
                    offering_id="test-1",
                    offer_name="Server",
                    currency="USD",
                    monthly_price=10.0,
                    product_type="compute",
                    datacenter_country="US",
                    datacenter_city="NYC",
                )
            ]
            mock_docs.side_effect = RuntimeError("Docs error")

            csv_path, docs_count = await scraper.run(skip_docs=True)

            assert csv_path is not None
            assert docs_count == 0
