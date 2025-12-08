"""Core crawler module wrapping Crawl4AI with project-specific defaults."""

from crawl4ai import BrowserConfig, CrawlerRunConfig, CacheMode
from crawl4ai.content_filter_strategy import PruningContentFilter
from crawl4ai.markdown_generation_strategy import DefaultMarkdownGenerator

# Default thresholds
DEFAULT_PRUNING_THRESHOLD = 0.48
DEFAULT_WORD_THRESHOLD = 10

DEFAULT_BROWSER_CONFIG = BrowserConfig(
    browser_type="chromium",
    headless=True,
    verbose=False,
)


def create_markdown_generator(
    threshold: float = DEFAULT_PRUNING_THRESHOLD,
    min_word_threshold: int = DEFAULT_WORD_THRESHOLD,
) -> DefaultMarkdownGenerator:
    """Create markdown generator with pruning content filter.

    Args:
        threshold: Pruning threshold (0.0-1.0). Lower retains more content, higher prunes more.
        min_word_threshold: Minimum words per content block to retain.

    Returns:
        Configured DefaultMarkdownGenerator instance.
    """
    content_filter = PruningContentFilter(
        threshold=threshold,
        threshold_type="fixed",
        min_word_threshold=min_word_threshold,
    )
    return DefaultMarkdownGenerator(content_filter=content_filter)


def create_crawl_config(
    cache_mode: CacheMode = CacheMode.BYPASS,
    excluded_tags: list[str] | None = None,
    word_count_threshold: int = DEFAULT_WORD_THRESHOLD,
    exclude_external_links: bool = True,
    markdown_generator: DefaultMarkdownGenerator | None = None,
) -> CrawlerRunConfig:
    """Create crawler run config with sensible defaults.

    Args:
        cache_mode: Cache mode for the crawler (default: BYPASS).
        excluded_tags: HTML tags to exclude from content (default: nav, footer, header, script, style).
        word_count_threshold: Minimum words per content block.
        exclude_external_links: Whether to exclude external links (default: True).
        markdown_generator: Optional custom markdown generator.

    Returns:
        Configured CrawlerRunConfig instance.
    """
    if excluded_tags is None:
        excluded_tags = ["nav", "footer", "header", "script", "style"]

    if markdown_generator is None:
        markdown_generator = create_markdown_generator()

    return CrawlerRunConfig(
        cache_mode=cache_mode,
        excluded_tags=excluded_tags,
        word_count_threshold=word_count_threshold,
        exclude_external_links=exclude_external_links,
        markdown_generator=markdown_generator,
        verbose=False,
    )
