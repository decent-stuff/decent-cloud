"""Tests for crawler module."""

import pytest
from crawl4ai import BrowserConfig, CacheMode, CrawlerRunConfig
from crawl4ai.content_filter_strategy import PruningContentFilter
from crawl4ai.markdown_generation_strategy import DefaultMarkdownGenerator

from scraper.crawler import (
    DEFAULT_BROWSER_CONFIG,
    DEFAULT_PRUNING_THRESHOLD,
    DEFAULT_WORD_THRESHOLD,
    create_crawl_config,
    create_markdown_generator,
)


class TestDefaultBrowserConfig:
    """Tests for DEFAULT_BROWSER_CONFIG."""

    def test_is_browser_config_instance(self):
        """Verify DEFAULT_BROWSER_CONFIG is a BrowserConfig instance."""
        assert isinstance(DEFAULT_BROWSER_CONFIG, BrowserConfig)

    def test_uses_chromium(self):
        """Verify browser type is chromium."""
        assert DEFAULT_BROWSER_CONFIG.browser_type == "chromium"

    def test_is_headless(self):
        """Verify headless mode is enabled."""
        assert DEFAULT_BROWSER_CONFIG.headless is True

    def test_verbose_disabled(self):
        """Verify verbose logging is disabled."""
        assert DEFAULT_BROWSER_CONFIG.verbose is False


class TestCreateMarkdownGenerator:
    """Tests for create_markdown_generator function."""

    def test_returns_markdown_generator(self):
        """Verify function returns DefaultMarkdownGenerator instance."""
        generator = create_markdown_generator()
        assert isinstance(generator, DefaultMarkdownGenerator)

    def test_has_content_filter(self):
        """Verify generator has content filter configured."""
        generator = create_markdown_generator()
        assert generator.content_filter is not None
        assert isinstance(generator.content_filter, PruningContentFilter)

    def test_default_threshold(self):
        """Verify default threshold matches constant."""
        generator = create_markdown_generator()
        assert generator.content_filter.threshold == DEFAULT_PRUNING_THRESHOLD

    def test_custom_threshold(self):
        """Verify custom threshold is applied."""
        generator = create_markdown_generator(threshold=0.6)
        assert generator.content_filter.threshold == 0.6

    def test_pruning_filter_config(self):
        """Verify pruning filter has correct configuration."""
        generator = create_markdown_generator()
        filter_obj = generator.content_filter
        assert filter_obj.threshold_type == "fixed"
        assert filter_obj.min_word_threshold == DEFAULT_WORD_THRESHOLD

    def test_custom_min_word_threshold(self):
        """Verify custom min_word_threshold is applied."""
        generator = create_markdown_generator(min_word_threshold=20)
        assert generator.content_filter.min_word_threshold == 20


class TestCreateCrawlConfig:
    """Tests for create_crawl_config function."""

    def test_returns_crawler_run_config(self):
        """Verify function returns CrawlerRunConfig instance."""
        config = create_crawl_config()
        assert isinstance(config, CrawlerRunConfig)

    def test_default_cache_mode_bypass(self):
        """Verify default cache mode is BYPASS."""
        config = create_crawl_config()
        assert config.cache_mode == CacheMode.BYPASS

    def test_custom_cache_mode(self):
        """Verify custom cache mode is applied."""
        config = create_crawl_config(cache_mode=CacheMode.ENABLED)
        assert config.cache_mode == CacheMode.ENABLED

    def test_default_excluded_tags(self):
        """Verify default excluded tags are set."""
        config = create_crawl_config()
        expected_tags = ["nav", "footer", "header", "script", "style"]
        assert config.excluded_tags == expected_tags

    def test_custom_excluded_tags(self):
        """Verify custom excluded tags are applied."""
        custom_tags = ["form", "aside"]
        config = create_crawl_config(excluded_tags=custom_tags)
        assert config.excluded_tags == custom_tags

    def test_default_word_count_threshold(self):
        """Verify default word count threshold matches constant."""
        config = create_crawl_config()
        assert config.word_count_threshold == DEFAULT_WORD_THRESHOLD

    def test_custom_word_count_threshold(self):
        """Verify custom word count threshold is applied."""
        config = create_crawl_config(word_count_threshold=20)
        assert config.word_count_threshold == 20

    def test_default_exclude_external_links(self):
        """Verify external links are excluded by default."""
        config = create_crawl_config()
        assert config.exclude_external_links is True

    def test_custom_exclude_external_links(self):
        """Verify custom exclude_external_links is applied."""
        config = create_crawl_config(exclude_external_links=False)
        assert config.exclude_external_links is False

    def test_has_markdown_generator(self):
        """Verify config has markdown generator."""
        config = create_crawl_config()
        assert config.markdown_generator is not None
        assert isinstance(config.markdown_generator, DefaultMarkdownGenerator)

    def test_custom_markdown_generator(self):
        """Verify custom markdown generator is applied."""
        custom_gen = create_markdown_generator(threshold=0.7)
        config = create_crawl_config(markdown_generator=custom_gen)
        assert config.markdown_generator is custom_gen
        assert config.markdown_generator.content_filter.threshold == 0.7

    def test_verbose_disabled(self):
        """Verify verbose logging is disabled."""
        config = create_crawl_config()
        assert config.verbose is False


class TestIntegration:
    """Integration tests combining multiple components."""

    def test_create_full_config_stack(self):
        """Verify we can create a complete configuration stack."""
        # Create custom markdown generator
        md_gen = create_markdown_generator(threshold=0.5)

        # Create crawl config with all custom params
        config = create_crawl_config(
            cache_mode=CacheMode.ENABLED,
            excluded_tags=["aside", "form"],
            word_count_threshold=15,
            exclude_external_links=False,
            markdown_generator=md_gen,
        )

        # Verify all settings are applied
        assert config.cache_mode == CacheMode.ENABLED
        assert config.excluded_tags == ["aside", "form"]
        assert config.word_count_threshold == 15
        assert config.exclude_external_links is False
        assert config.markdown_generator is md_gen
        assert config.markdown_generator.content_filter.threshold == 0.5

    def test_browser_and_crawl_config_compatibility(self):
        """Verify browser config and crawl config can be used together."""
        browser_cfg = DEFAULT_BROWSER_CONFIG
        crawl_cfg = create_crawl_config()

        # These configs should be usable together with AsyncWebCrawler
        assert browser_cfg.browser_type == "chromium"
        assert browser_cfg.headless is True
        assert crawl_cfg.cache_mode == CacheMode.BYPASS
        assert crawl_cfg.markdown_generator is not None
