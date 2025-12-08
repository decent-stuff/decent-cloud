"""Tests for storage module."""

import json
import zipfile
from datetime import datetime, timezone
from pathlib import Path

import pytest

from scraper.storage import CacheEntry, DocsArchive


@pytest.fixture
def tmp_output_dir(tmp_path):
    """Create a temporary output directory for tests."""
    output_dir = tmp_path / "output"
    output_dir.mkdir()
    return output_dir


@pytest.fixture
def archive(tmp_output_dir):
    """Create a DocsArchive instance for tests."""
    return DocsArchive(tmp_output_dir)


class TestDocsArchiveInit:
    """Tests for DocsArchive initialization."""

    def test_creates_output_directory(self, tmp_path):
        """Verify output directory is created if not exists."""
        output_dir = tmp_path / "new_dir"
        assert not output_dir.exists()

        DocsArchive(output_dir)
        assert output_dir.exists()

    def test_initializes_paths(self, archive, tmp_output_dir):
        """Verify zip_path and cache_path are initialized correctly."""
        assert archive.zip_path == tmp_output_dir / "docs.zip"
        assert archive.cache_path == tmp_output_dir / "cache.json"

    def test_loads_empty_cache_when_not_exists(self, archive):
        """Verify empty cache is loaded when cache.json doesn't exist."""
        assert archive._cache == {}


class TestCacheLoadSave:
    """Tests for cache loading and saving."""

    def test_save_cache_creates_json_file(self, archive):
        """Verify _save_cache creates cache.json with correct format."""
        archive._cache = {
            "https://example.com/page1": CacheEntry(
                filename="page1.md",
                etag='"abc123"',
                content_hash="a1b2c3d4e5f6g7h8",
                crawled_at="2025-12-08T12:00:00Z",
            )
        }
        archive._save_cache()

        assert archive.cache_path.exists()
        with open(archive.cache_path, "r") as f:
            data = json.load(f)

        assert "https://example.com/page1" in data
        assert data["https://example.com/page1"]["filename"] == "page1.md"
        assert data["https://example.com/page1"]["etag"] == '"abc123"'
        assert data["https://example.com/page1"]["content_hash"] == "a1b2c3d4e5f6g7h8"

    def test_load_cache_reads_existing_file(self, tmp_output_dir):
        """Verify _load_cache reads existing cache.json correctly."""
        cache_data = {
            "https://example.com/page1": {
                "filename": "page1.md",
                "etag": '"abc123"',
                "content_hash": "a1b2c3d4e5f6g7h8",
                "crawled_at": "2025-12-08T12:00:00Z",
            }
        }
        cache_path = tmp_output_dir / "cache.json"
        with open(cache_path, "w") as f:
            json.dump(cache_data, f)

        archive = DocsArchive(tmp_output_dir)
        assert "https://example.com/page1" in archive._cache
        entry = archive._cache["https://example.com/page1"]
        assert entry.filename == "page1.md"
        assert entry.etag == '"abc123"'
        assert entry.content_hash == "a1b2c3d4e5f6g7h8"

    def test_load_cache_raises_on_invalid_json(self, tmp_output_dir):
        """Verify _load_cache raises on invalid JSON."""
        cache_path = tmp_output_dir / "cache.json"
        with open(cache_path, "w") as f:
            f.write("invalid json {{{")

        with pytest.raises(json.JSONDecodeError):
            DocsArchive(tmp_output_dir)


class TestContentHash:
    """Tests for content hashing."""

    def test_content_hash_returns_16_chars(self, archive):
        """Verify _content_hash returns first 16 chars of SHA256."""
        content = "test content"
        hash_result = archive._content_hash(content)
        assert len(hash_result) == 16
        assert isinstance(hash_result, str)

    def test_content_hash_consistent(self, archive):
        """Verify _content_hash returns same hash for same content."""
        content = "test content"
        hash1 = archive._content_hash(content)
        hash2 = archive._content_hash(content)
        assert hash1 == hash2

    def test_content_hash_different_for_different_content(self, archive):
        """Verify _content_hash returns different hash for different content."""
        hash1 = archive._content_hash("content 1")
        hash2 = archive._content_hash("content 2")
        assert hash1 != hash2


class TestSafeFilename:
    """Tests for safe filename generation."""

    def test_safe_filename_uses_topic(self, archive):
        """Verify _safe_filename uses topic when provided."""
        filename = archive._safe_filename("https://example.com/page", "my-topic")
        assert filename == "my-topic.md"

    def test_safe_filename_extracts_from_url_when_no_topic(self, archive):
        """Verify _safe_filename extracts from URL when topic is empty."""
        filename = archive._safe_filename("https://example.com/cloud-servers", "")
        assert filename == "cloud-servers.md"

    def test_safe_filename_replaces_special_chars(self, archive):
        """Verify _safe_filename replaces special chars with hyphens and collapses them."""
        filename = archive._safe_filename("https://example.com/page", "my topic!@#$%")
        # Multiple hyphens get collapsed to single hyphen
        assert filename == "my-topic.md"

    def test_safe_filename_collapses_multiple_hyphens(self, archive):
        """Verify _safe_filename collapses multiple hyphens."""
        filename = archive._safe_filename("https://example.com/page", "my---topic")
        assert filename == "my-topic.md"

    def test_safe_filename_strips_leading_trailing_hyphens(self, archive):
        """Verify _safe_filename strips leading/trailing hyphens."""
        filename = archive._safe_filename("https://example.com/page", "-topic-")
        assert filename == "topic.md"

    def test_safe_filename_handles_empty_topic_and_url(self, archive):
        """Verify _safe_filename returns 'index.md' when path is empty."""
        filename = archive._safe_filename("https://example.com/", "")
        # When path is empty (only domain), falls back to "index"
        assert filename == "index.md"

    def test_safe_filename_handles_topic_with_only_special_chars(self, archive):
        """Verify _safe_filename returns 'page.md' when topic is only special chars."""
        filename = archive._safe_filename("https://example.com/page", "!@#$%^&*()")
        # Special chars get replaced with hyphens, then collapsed, then stripped
        # Result is empty string, so fallback to "page"
        assert filename == "page.md"

    def test_safe_filename_preserves_alphanumeric_and_safe_chars(self, archive):
        """Verify _safe_filename preserves alphanumeric, hyphens, underscores."""
        filename = archive._safe_filename("https://example.com/page", "my_topic-123")
        assert filename == "my_topic-123.md"


class TestHasChanged:
    """Tests for change detection."""

    def test_has_changed_returns_true_for_new_url(self, archive):
        """Verify has_changed returns True for URL not in cache."""
        assert archive.has_changed("https://example.com/new", None, "content") is True

    def test_has_changed_uses_etag_when_available(self, archive):
        """Verify has_changed uses ETag comparison when available on both sides."""
        archive._cache = {
            "https://example.com/page": CacheEntry(
                filename="page.md",
                etag='"abc123"',
                content_hash="hash1",
                crawled_at="2025-12-08T12:00:00Z",
            )
        }

        # ETag matches - not changed
        assert archive.has_changed("https://example.com/page", '"abc123"', "different content") is False

    def test_has_changed_etag_mismatch_returns_true(self, archive):
        """Verify has_changed returns True when ETags differ."""
        archive._cache = {
            "https://example.com/page": CacheEntry(
                filename="page.md",
                etag='"abc123"',
                content_hash="hash1",
                crawled_at="2025-12-08T12:00:00Z",
            )
        }

        # ETag differs - changed
        assert archive.has_changed("https://example.com/page", '"xyz789"', "content") is True

    def test_has_changed_falls_back_to_content_hash(self, archive):
        """Verify has_changed uses content hash when ETag not available."""
        content = "test content"
        content_hash = archive._content_hash(content)

        archive._cache = {
            "https://example.com/page": CacheEntry(
                filename="page.md",
                etag=None,
                content_hash=content_hash,
                crawled_at="2025-12-08T12:00:00Z",
            )
        }

        # Same content hash - not changed
        assert archive.has_changed("https://example.com/page", None, content) is False

    def test_has_changed_content_hash_mismatch_returns_true(self, archive):
        """Verify has_changed returns True when content hash differs."""
        archive._cache = {
            "https://example.com/page": CacheEntry(
                filename="page.md",
                etag=None,
                content_hash="oldhash",
                crawled_at="2025-12-08T12:00:00Z",
            )
        }

        # Different content - changed
        assert archive.has_changed("https://example.com/page", None, "new content") is True

    def test_has_changed_etag_match_ignores_content_hash(self, archive):
        """Verify has_changed trusts ETag match even if content hash would differ."""
        archive._cache = {
            "https://example.com/page": CacheEntry(
                filename="page.md",
                etag='"abc123"',
                content_hash="oldhash",
                crawled_at="2025-12-08T12:00:00Z",
            )
        }

        # ETag matches - not changed (even though content hash would differ)
        assert archive.has_changed("https://example.com/page", '"abc123"', "different content") is False


class TestWrite:
    """Tests for writing to local docs/ dir (ZIP created on finalize)."""

    def test_write_creates_local_file(self, archive):
        """Verify write creates file in local docs/ directory."""
        assert not (archive.docs_dir / "test-page.md").exists()

        archive.write("https://example.com/page", "# Test", "test-page")
        assert (archive.docs_dir / "test-page.md").exists()

    def test_write_adds_content_to_local_dir(self, archive):
        """Verify write adds markdown content to local dir with correct filename."""
        content = "# Test Page\n\nTest content"
        filename = archive.write("https://example.com/page", content, "test-page")

        assert filename == "test-page.md"
        local_path = archive.docs_dir / "test-page.md"
        assert local_path.exists()
        assert local_path.read_text(encoding="utf-8") == content

    def test_write_updates_cache(self, archive):
        """Verify write updates cache with correct entry."""
        content = "# Test"
        etag = '"abc123"'

        archive.write("https://example.com/page", content, "test-page", etag)

        assert "https://example.com/page" in archive._cache
        entry = archive._cache["https://example.com/page"]
        assert entry.filename == "test-page.md"
        assert entry.etag == etag
        assert entry.content_hash == archive._content_hash(content)
        # Verify crawled_at is recent ISO format
        crawled_dt = datetime.fromisoformat(entry.crawled_at)
        assert (datetime.now(timezone.utc) - crawled_dt).total_seconds() < 5

    def test_write_saves_cache_to_disk(self, archive):
        """Verify write persists cache to cache.json."""
        archive.write("https://example.com/page", "# Test", "test-page")

        assert archive.cache_path.exists()
        with open(archive.cache_path, "r") as f:
            data = json.load(f)

        assert "https://example.com/page" in data

    def test_write_updates_existing_file_in_local_dir(self, archive):
        """Verify write updates existing file in local docs/ dir."""
        # Write initial content
        archive.write("https://example.com/page", "# Version 1", "test-page")

        # Update content
        archive.write("https://example.com/page", "# Version 2", "test-page")

        # Verify file has updated content
        local_path = archive.docs_dir / "test-page.md"
        assert local_path.read_text(encoding="utf-8") == "# Version 2"

    def test_write_preserves_other_files_in_local_dir(self, archive):
        """Verify write preserves other files when updating one file."""
        # Write multiple files
        archive.write("https://example.com/page1", "# Page 1", "page1")
        archive.write("https://example.com/page2", "# Page 2", "page2")
        archive.write("https://example.com/page3", "# Page 3", "page3")

        # Update one file
        archive.write("https://example.com/page2", "# Page 2 Updated", "page2")

        # Verify all files exist in local dir
        files = sorted(f.name for f in archive.docs_dir.iterdir())
        assert files == ["page1.md", "page2.md", "page3.md"]

        # Verify content
        assert (archive.docs_dir / "page1.md").read_text(encoding="utf-8") == "# Page 1"
        assert (archive.docs_dir / "page2.md").read_text(encoding="utf-8") == "# Page 2 Updated"
        assert (archive.docs_dir / "page3.md").read_text(encoding="utf-8") == "# Page 3"


class TestFinalize:
    """Tests for finalize method (creates ZIP from local docs/)."""

    def test_finalize_creates_zip_from_local_dir(self, archive):
        """Verify finalize creates ZIP from local docs/ dir."""
        archive.write("https://example.com/page1", "# Page 1", "page1")
        archive.write("https://example.com/page2", "# Page 2", "page2")

        archive.finalize()

        assert archive.zip_path.exists()
        with zipfile.ZipFile(archive.zip_path, "r") as zf:
            assert sorted(zf.namelist()) == ["page1.md", "page2.md"]
            assert zf.read("page1.md").decode("utf-8") == "# Page 1"

    def test_finalize_removes_local_dir_by_default(self, archive):
        """Verify finalize removes local docs/ dir by default."""
        archive.write("https://example.com/page", "# Test", "test")
        assert archive.docs_dir.exists()

        archive.finalize(keep_local=False)

        assert not archive.docs_dir.exists()

    def test_finalize_keeps_local_dir_when_requested(self, archive):
        """Verify finalize keeps local docs/ dir when keep_local=True."""
        archive.write("https://example.com/page", "# Test", "test")

        archive.finalize(keep_local=True)

        assert archive.docs_dir.exists()
        assert (archive.docs_dir / "test.md").exists()

    def test_finalize_handles_empty_docs_dir(self, archive):
        """Verify finalize handles empty docs/ dir gracefully."""
        assert archive.docs_dir.exists()  # Created on init

        archive.finalize()  # Should not raise

        assert not archive.zip_path.exists()  # No files to zip


class TestRead:
    """Tests for reading from local dir or ZIP archive."""

    def test_read_returns_none_for_missing_url(self, archive):
        """Verify read returns None for URL not in cache."""
        assert archive.read("https://example.com/missing") is None

    def test_read_returns_none_when_neither_local_nor_zip_exists(self, archive):
        """Verify read returns None when neither local file nor ZIP exists."""
        # Add entry to cache but no file
        archive._cache = {
            "https://example.com/page": CacheEntry(
                filename="page.md",
                etag=None,
                content_hash="hash",
                crawled_at="2025-12-08T12:00:00Z",
            )
        }
        # Remove docs dir content (file doesn't exist)
        import shutil
        shutil.rmtree(archive.docs_dir)
        archive.docs_dir.mkdir()

        assert archive.read("https://example.com/page") is None

    def test_read_returns_content_for_existing_url(self, archive):
        """Verify read returns content for URL in cache and ZIP."""
        content = "# Test Content\n\nTest body"
        archive.write("https://example.com/page", content, "test-page")

        read_content = archive.read("https://example.com/page")
        assert read_content == content

    def test_read_returns_none_when_file_missing_from_zip(self, archive):
        """Verify read returns None when cache entry exists but file missing from ZIP."""
        # Add entry to cache
        archive._cache = {
            "https://example.com/page": CacheEntry(
                filename="missing.md",
                etag=None,
                content_hash="hash",
                crawled_at="2025-12-08T12:00:00Z",
            )
        }
        archive._save_cache()

        # Create empty ZIP
        with zipfile.ZipFile(archive.zip_path, "w") as zf:
            pass

        assert archive.read("https://example.com/page") is None

    def test_read_multiple_files(self, archive):
        """Verify read works correctly with multiple files in ZIP."""
        content1 = "# Page 1"
        content2 = "# Page 2"
        content3 = "# Page 3"

        archive.write("https://example.com/page1", content1, "page1")
        archive.write("https://example.com/page2", content2, "page2")
        archive.write("https://example.com/page3", content3, "page3")

        assert archive.read("https://example.com/page1") == content1
        assert archive.read("https://example.com/page2") == content2
        assert archive.read("https://example.com/page3") == content3


class TestIntegration:
    """Integration tests combining multiple operations."""

    def test_write_read_roundtrip(self, archive):
        """Verify content written can be read back correctly."""
        content = "# Integration Test\n\nThis is a test."
        url = "https://example.com/integration"

        filename = archive.write(url, content, "integration-test", etag='"test123"')
        read_content = archive.read(url)

        assert filename == "integration-test.md"
        assert read_content == content

    def test_incremental_crawl_simulation(self, archive):
        """Verify incremental crawl behavior: write, check unchanged, update."""
        url = "https://example.com/page"
        content_v1 = "# Version 1"
        etag_v1 = '"v1"'

        # Initial crawl
        archive.write(url, content_v1, "page", etag_v1)
        assert archive.has_changed(url, etag_v1, content_v1) is False

        # Content changed
        content_v2 = "# Version 2"
        etag_v2 = '"v2"'
        assert archive.has_changed(url, etag_v2, content_v2) is True

        # Update
        archive.write(url, content_v2, "page", etag_v2)
        assert archive.read(url) == content_v2
        assert archive.has_changed(url, etag_v2, content_v2) is False

    def test_cache_persistence_across_instances(self, tmp_output_dir):
        """Verify cache persists across DocsArchive instances."""
        url = "https://example.com/page"
        content = "# Test"
        etag = '"test123"'

        # First instance - write
        archive1 = DocsArchive(tmp_output_dir)
        archive1.write(url, content, "page", etag)

        # Second instance - should load cache
        archive2 = DocsArchive(tmp_output_dir)
        assert archive2.has_changed(url, etag, content) is False
        assert archive2.read(url) == content
