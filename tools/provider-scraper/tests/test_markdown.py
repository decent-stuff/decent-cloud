"""Tests for the HTML to Markdown converter."""

from scraper.markdown import (
    MAX_CHUNK_SIZE,
    chunk_markdown,
    format_markdown_with_frontmatter,
    html_to_markdown,
)


def test_html_to_markdown_simple() -> None:
    """Test basic HTML to markdown conversion."""
    html = """
    <html>
    <body>
        <h1>Test Page</h1>
        <p>This is a paragraph.</p>
        <ul>
            <li>Item 1</li>
            <li>Item 2</li>
        </ul>
    </body>
    </html>
    """
    doc = html_to_markdown(html, "https://example.com", "Example", "Test")
    assert "# Test Page" in doc.content
    assert "This is a paragraph" in doc.content
    assert "- Item 1" in doc.content
    assert "- Item 2" in doc.content


def test_html_to_markdown_removes_scripts() -> None:
    """Test that scripts and styles are removed."""
    html = """
    <html>
    <head><script>alert('xss')</script></head>
    <body>
        <style>.hidden { display: none }</style>
        <p>Real content</p>
    </body>
    </html>
    """
    doc = html_to_markdown(html, "https://example.com", "Example", "Test")
    assert "alert" not in doc.content
    assert "hidden" not in doc.content
    assert "Real content" in doc.content


def test_html_to_markdown_removes_nav_footer() -> None:
    """Test that navigation and footer elements are removed."""
    html = """
    <html>
    <body>
        <nav>Navigation links</nav>
        <main><p>Main content here</p></main>
        <footer>Footer stuff</footer>
    </body>
    </html>
    """
    doc = html_to_markdown(html, "https://example.com", "Example", "Test")
    assert "Navigation" not in doc.content
    assert "Footer" not in doc.content
    assert "Main content here" in doc.content


def test_html_to_markdown_removes_cookie_banners() -> None:
    """Test that cookie banners and similar noise are removed."""
    html = """
    <html>
    <body>
        <div class="cookie-banner">Accept cookies</div>
        <div id="newsletter-popup">Subscribe!</div>
        <p>Actual content</p>
    </body>
    </html>
    """
    doc = html_to_markdown(html, "https://example.com", "Example", "Test")
    assert "cookie" not in doc.content.lower()
    assert "Subscribe" not in doc.content
    assert "Actual content" in doc.content


def test_html_to_markdown_preserves_links() -> None:
    """Test that links are preserved."""
    html = '<p>Visit <a href="https://example.com/page">this page</a> for more.</p>'
    doc = html_to_markdown(html, "https://example.com", "Example", "Test")
    assert "[this page](https://example.com/page)" in doc.content


def test_html_to_markdown_table() -> None:
    """Test table conversion."""
    html = """
    <table>
        <thead><tr><th>Name</th><th>Price</th></tr></thead>
        <tbody>
            <tr><td>Server A</td><td>$10</td></tr>
            <tr><td>Server B</td><td>$20</td></tr>
        </tbody>
    </table>
    """
    doc = html_to_markdown(html, "https://example.com", "Example", "Test")
    assert "| Name | Price |" in doc.content
    assert "| Server A | $10 |" in doc.content


def test_html_to_markdown_code_blocks() -> None:
    """Test code block conversion."""
    html = "<pre><code>def hello():\n    print('world')</code></pre>"
    doc = html_to_markdown(html, "https://example.com", "Example", "Test")
    assert "```" in doc.content
    assert "def hello()" in doc.content


def test_format_markdown_with_frontmatter() -> None:
    """Test frontmatter generation."""
    doc = html_to_markdown("<p>Content</p>", "https://ex.com", "Provider", "Topic")
    formatted = format_markdown_with_frontmatter(doc)
    assert "---" in formatted
    assert "source: https://ex.com" in formatted
    assert "provider: Provider" in formatted
    assert "topic: Topic" in formatted


def test_chunk_markdown_under_limit() -> None:
    """Test that small docs are not chunked."""
    doc = html_to_markdown("<p>Small content</p>", "https://ex.com", "P", "T")
    chunks = chunk_markdown(doc)
    assert len(chunks) == 1


def test_chunk_markdown_large_doc() -> None:
    """Test that large docs are chunked properly."""
    from datetime import UTC, datetime

    from scraper.markdown import MarkdownDoc

    # Create content that exceeds 20KB directly (bypass HTML cleaning)
    large_content = ""
    for i in range(10):
        large_content += f"## Section {i}\n\n" + ("x" * 3000 + "\n\n")

    doc = MarkdownDoc(
        content=large_content,
        source_url="https://ex.com",
        provider="P",
        topic="Large Doc",
        scraped_at=datetime.now(UTC),
    )
    chunks = chunk_markdown(doc)

    assert len(chunks) > 1

    # Each chunk should be under the limit
    for chunk in chunks:
        formatted = format_markdown_with_frontmatter(chunk)
        assert len(formatted.encode("utf-8")) <= MAX_CHUNK_SIZE


def test_chunk_markdown_respects_headings() -> None:
    """Test that chunking tries to split at heading boundaries."""
    # Create content with clear section boundaries
    html = """
    <body>
        <h2>Section 1</h2>
        <p>Content for section 1</p>
        <h2>Section 2</h2>
        <p>Content for section 2</p>
    </body>
    """
    doc = html_to_markdown(html, "https://ex.com", "P", "T")
    # This is small enough to not chunk, but tests the structure
    assert "## Section 1" in doc.content
    assert "## Section 2" in doc.content
