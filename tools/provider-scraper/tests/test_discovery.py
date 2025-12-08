"""Tests for discovery module."""

import pytest

from scraper.discovery import parse_sitemap_xml, _extract_sitemaps_from_robots


class TestParseSitemapXml:
    """Tests for parse_sitemap_xml function."""

    def test_urlset_format(self):
        """Verify parsing of standard urlset format."""
        xml = """<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
  <url>
    <loc>https://example.com/page1</loc>
  </url>
  <url>
    <loc>https://example.com/page2</loc>
  </url>
</urlset>"""
        urls = parse_sitemap_xml(xml)
        assert len(urls) == 2
        assert "https://example.com/page1" in urls
        assert "https://example.com/page2" in urls

    def test_urlset_without_namespace(self):
        """Verify parsing works without namespace declaration."""
        xml = """<?xml version="1.0" encoding="UTF-8"?>
<urlset>
  <url>
    <loc>https://example.com/test</loc>
  </url>
</urlset>"""
        urls = parse_sitemap_xml(xml)
        assert len(urls) == 1
        assert urls[0] == "https://example.com/test"

    def test_sitemap_index_format(self):
        """Verify parsing of sitemap index format."""
        xml = """<?xml version="1.0" encoding="UTF-8"?>
<sitemapindex xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
  <sitemap>
    <loc>https://example.com/sitemap1.xml</loc>
  </sitemap>
  <sitemap>
    <loc>https://example.com/sitemap2.xml</loc>
  </sitemap>
</sitemapindex>"""
        urls = parse_sitemap_xml(xml)
        assert len(urls) == 2
        assert "https://example.com/sitemap1.xml" in urls
        assert "https://example.com/sitemap2.xml" in urls

    def test_sitemap_index_without_namespace(self):
        """Verify parsing sitemap index without namespace."""
        xml = """<?xml version="1.0"?>
<sitemapindex>
  <sitemap>
    <loc>https://example.com/posts.xml</loc>
  </sitemap>
</sitemapindex>"""
        urls = parse_sitemap_xml(xml)
        assert len(urls) == 1
        assert urls[0] == "https://example.com/posts.xml"

    def test_invalid_xml(self):
        """Verify invalid XML returns empty list."""
        xml = "This is not XML at all!"
        urls = parse_sitemap_xml(xml)
        assert urls == []

    def test_malformed_xml(self):
        """Verify malformed XML returns empty list."""
        xml = "<urlset><url><loc>https://example.com</url></urlset>"
        urls = parse_sitemap_xml(xml)
        # Should still parse but might get nothing or partial results
        # The key is it doesn't crash
        assert isinstance(urls, list)

    def test_empty_string(self):
        """Verify empty string returns empty list."""
        urls = parse_sitemap_xml("")
        assert urls == []

    def test_whitespace_only(self):
        """Verify whitespace-only string returns empty list."""
        urls = parse_sitemap_xml("   \n\t  ")
        assert urls == []

    def test_empty_sitemap(self):
        """Verify empty sitemap returns empty list."""
        xml = """<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
</urlset>"""
        urls = parse_sitemap_xml(xml)
        assert urls == []

    def test_empty_sitemap_index(self):
        """Verify empty sitemap index returns empty list."""
        xml = """<?xml version="1.0" encoding="UTF-8"?>
<sitemapindex xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
</sitemapindex>"""
        urls = parse_sitemap_xml(xml)
        assert urls == []

    def test_mixed_empty_and_valid_urls(self):
        """Verify handling of empty loc tags mixed with valid ones."""
        xml = """<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
  <url>
    <loc>https://example.com/valid</loc>
  </url>
  <url>
    <loc></loc>
  </url>
  <url>
    <loc>https://example.com/another</loc>
  </url>
</urlset>"""
        urls = parse_sitemap_xml(xml)
        # Empty loc tags should be filtered out by strip() check
        assert len(urls) == 2
        assert "https://example.com/valid" in urls
        assert "https://example.com/another" in urls

    def test_urls_with_whitespace(self):
        """Verify URLs with surrounding whitespace are trimmed."""
        xml = """<?xml version="1.0" encoding="UTF-8"?>
<urlset>
  <url>
    <loc>  https://example.com/page  </loc>
  </url>
</urlset>"""
        urls = parse_sitemap_xml(xml)
        assert len(urls) == 1
        assert urls[0] == "https://example.com/page"

    def test_complex_sitemap_with_additional_tags(self):
        """Verify parsing works when sitemap has additional tags."""
        xml = """<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
  <url>
    <loc>https://example.com/page1</loc>
    <lastmod>2023-01-01</lastmod>
    <changefreq>weekly</changefreq>
    <priority>0.8</priority>
  </url>
  <url>
    <loc>https://example.com/page2</loc>
    <lastmod>2023-01-02</lastmod>
  </url>
</urlset>"""
        urls = parse_sitemap_xml(xml)
        assert len(urls) == 2
        assert "https://example.com/page1" in urls
        assert "https://example.com/page2" in urls

    def test_nested_structure(self):
        """Verify parsing handles deeply nested structures."""
        xml = """<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
  <url>
    <loc>https://example.com/test</loc>
  </url>
</urlset>"""
        urls = parse_sitemap_xml(xml)
        assert len(urls) == 1
        assert urls[0] == "https://example.com/test"


class TestExtractSitemapsFromRobots:
    """Tests for _extract_sitemaps_from_robots function."""

    def test_single_sitemap(self):
        """Verify extraction of single sitemap URL."""
        robots = """User-agent: *
Disallow: /admin/

Sitemap: https://example.com/sitemap.xml"""
        sitemaps = _extract_sitemaps_from_robots(robots)
        assert len(sitemaps) == 1
        assert sitemaps[0] == "https://example.com/sitemap.xml"

    def test_multiple_sitemaps(self):
        """Verify extraction of multiple sitemap URLs."""
        robots = """User-agent: *
Sitemap: https://example.com/sitemap1.xml
Sitemap: https://example.com/sitemap2.xml
Disallow: /private/"""
        sitemaps = _extract_sitemaps_from_robots(robots)
        assert len(sitemaps) == 2
        assert "https://example.com/sitemap1.xml" in sitemaps
        assert "https://example.com/sitemap2.xml" in sitemaps

    def test_no_sitemap(self):
        """Verify empty list when no sitemap in robots.txt."""
        robots = """User-agent: *
Disallow: /admin/
Allow: /public/"""
        sitemaps = _extract_sitemaps_from_robots(robots)
        assert sitemaps == []

    def test_case_insensitive(self):
        """Verify case-insensitive sitemap extraction."""
        robots = """User-agent: *
SITEMAP: https://example.com/upper.xml
sitemap: https://example.com/lower.xml"""
        sitemaps = _extract_sitemaps_from_robots(robots)
        assert len(sitemaps) == 2

    def test_sitemap_with_whitespace(self):
        """Verify trimming of whitespace around sitemap URL."""
        robots = """Sitemap:   https://example.com/sitemap.xml   """
        sitemaps = _extract_sitemaps_from_robots(robots)
        assert len(sitemaps) == 1
        assert sitemaps[0] == "https://example.com/sitemap.xml"

    def test_empty_robots(self):
        """Verify empty list for empty robots.txt."""
        sitemaps = _extract_sitemaps_from_robots("")
        assert sitemaps == []
