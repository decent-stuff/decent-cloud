"""HTML to Markdown converter optimized for LLM consumption."""

from __future__ import annotations

import re
from dataclasses import dataclass
from datetime import UTC, datetime
from pathlib import Path
from typing import TYPE_CHECKING

from bs4 import BeautifulSoup
from bs4.element import NavigableString

if TYPE_CHECKING:
    from bs4 import Tag

MAX_CHUNK_SIZE = 20 * 1024  # 20KB limit for Chatwoot


@dataclass
class MarkdownDoc:
    """A markdown document with metadata."""

    content: str
    source_url: str
    provider: str
    topic: str
    scraped_at: datetime


def _clean_text(text: str) -> str:
    """Clean text by removing extra whitespace."""
    # Replace multiple spaces/newlines with single space
    text = re.sub(r"\s+", " ", text)
    return text.strip()


def _process_element(element: Tag | NavigableString, depth: int = 0) -> str:
    """Recursively process an HTML element to markdown."""
    if isinstance(element, NavigableString):
        text = str(element)
        if text.strip():
            return _clean_text(text)
        return ""

    tag_name = element.name

    # Skip noise elements
    if tag_name in ("script", "style", "nav", "footer", "header", "aside", "iframe", "noscript"):
        return ""

    # Skip elements with noise classes/ids
    skip_patterns = ["cookie", "banner", "popup", "modal", "newsletter", "social", "share", "ad-"]
    class_attr = element.get("class")
    element_class = " ".join(class_attr) if isinstance(class_attr, list) else str(class_attr or "")
    element_id = str(element.get("id") or "")
    for pattern in skip_patterns:
        if pattern in element_class.lower() or pattern in element_id.lower():
            return ""

    # Process children
    children_md: list[str] = []
    for child in element.children:
        if hasattr(child, "name") or isinstance(child, NavigableString):
            result = _process_element(child, depth + 1)  # type: ignore[arg-type]
            if result:
                children_md.append(result)

    content = " ".join(children_md)

    # Handle specific tags
    if tag_name in ("h1", "h2", "h3", "h4", "h5", "h6"):
        level = int(tag_name[1])
        return f"\n\n{'#' * level} {content}\n\n"

    if tag_name == "p":
        return f"\n\n{content}\n\n" if content else ""

    if tag_name == "br":
        return "\n"

    if tag_name in ("strong", "b"):
        return f"**{content}**" if content else ""

    if tag_name in ("em", "i"):
        return f"*{content}*" if content else ""

    if tag_name == "a":
        href = element.get("href") or ""
        href_str = str(href)
        if href_str and content and not href_str.startswith("#"):
            return f"[{content}]({href_str})"
        return content

    if tag_name == "code":
        return f"`{content}`" if content else ""

    if tag_name == "pre":
        code_elem = element.find("code")
        code_content = code_elem.get_text() if code_elem else content
        return f"\n\n```\n{code_content}\n```\n\n"

    if tag_name == "ul":
        items = element.find_all("li", recursive=False)
        md_items: list[str] = []
        for item in items:
            item_text = _process_element(item, depth + 1)
            if item_text:
                md_items.append(f"- {item_text}")
        return "\n" + "\n".join(md_items) + "\n" if md_items else ""

    if tag_name == "ol":
        items = element.find_all("li", recursive=False)
        md_items_ol: list[str] = []
        for i, item in enumerate(items, 1):
            item_text = _process_element(item, depth + 1)
            if item_text:
                md_items_ol.append(f"{i}. {item_text}")
        return "\n" + "\n".join(md_items_ol) + "\n" if md_items_ol else ""

    if tag_name == "table":
        return _process_table(element)

    if tag_name in ("div", "section", "article", "main", "span", "li"):
        return content

    return content


def _process_table(table: Tag) -> str:
    """Convert an HTML table to markdown."""
    rows: list[list[str]] = []
    header_row: list[str] = []

    # Try to find header
    thead = table.find("thead")
    if thead:
        for th in thead.find_all(["th", "td"]):
            header_row.append(_clean_text(th.get_text()))

    # Process body rows
    tbody = table.find("tbody") or table
    for tr in tbody.find_all("tr"):
        cells = tr.find_all(["td", "th"])
        if cells:
            row = [_clean_text(cell.get_text()) for cell in cells]
            # If no header yet and this looks like a header row
            if not header_row and all(cell.name == "th" for cell in cells):
                header_row = row
            else:
                rows.append(row)

    if not rows and not header_row:
        return ""

    # Build markdown table
    md_lines: list[str] = []

    # Use first row as header if none found
    if not header_row and rows:
        header_row = rows.pop(0)

    if header_row:
        md_lines.append("| " + " | ".join(header_row) + " |")
        md_lines.append("| " + " | ".join(["---"] * len(header_row)) + " |")

    for row in rows:
        # Pad row to match header length
        while len(row) < len(header_row):
            row.append("")
        md_lines.append("| " + " | ".join(row[: len(header_row)]) + " |")

    return "\n\n" + "\n".join(md_lines) + "\n\n" if md_lines else ""


def html_to_markdown(
    html: str,
    source_url: str,
    provider: str,
    topic: str,
) -> MarkdownDoc:
    """Convert HTML to clean markdown optimized for LLM consumption."""
    soup = BeautifulSoup(html, "lxml")

    # Remove all script/style tags first
    for tag in soup(["script", "style", "noscript"]):
        tag.decompose()

    # Find main content area
    main = soup.find("main") or soup.find("article") or soup.find("body") or soup

    content = _process_element(main)  # type: ignore[arg-type]

    # Clean up multiple newlines
    content = re.sub(r"\n{3,}", "\n\n", content)
    content = content.strip()

    return MarkdownDoc(
        content=content,
        source_url=source_url,
        provider=provider,
        topic=topic,
        scraped_at=datetime.now(UTC),
    )


def format_markdown_with_frontmatter(doc: MarkdownDoc) -> str:
    """Format a MarkdownDoc with YAML frontmatter."""
    frontmatter = f"""---
source: {doc.source_url}
scraped: {doc.scraped_at.strftime('%Y-%m-%d')}
provider: {doc.provider}
topic: {doc.topic}
---

"""
    return frontmatter + doc.content


def chunk_markdown(doc: MarkdownDoc, max_size: int = MAX_CHUNK_SIZE) -> list[MarkdownDoc]:
    """Split a markdown document into chunks respecting the size limit.

    Splits at heading boundaries (## or ###) when possible.
    """
    full_content = format_markdown_with_frontmatter(doc)

    # If already under limit, return as-is
    if len(full_content.encode("utf-8")) <= max_size:
        return [doc]

    # Split by h2 headings first
    sections = re.split(r"(\n## [^\n]+\n)", doc.content)

    chunks: list[MarkdownDoc] = []
    current_content: list[str] = []
    current_size = 0
    chunk_num = 1

    # Estimate frontmatter size
    frontmatter_size = len(format_markdown_with_frontmatter(doc).encode("utf-8")) - len(
        doc.content.encode("utf-8")
    )

    for section in sections:
        section_size = len(section.encode("utf-8"))

        # If this section alone is too big, split by paragraphs
        if section_size + frontmatter_size > max_size:
            # Flush current content first
            if current_content:
                chunk_content = "".join(current_content)
                chunks.append(
                    MarkdownDoc(
                        content=chunk_content,
                        source_url=doc.source_url,
                        provider=doc.provider,
                        topic=f"{doc.topic} (Part {chunk_num})",
                        scraped_at=doc.scraped_at,
                    )
                )
                chunk_num += 1
                current_content = []
                current_size = 0

            # Split large section by paragraphs
            paragraphs = section.split("\n\n")
            for para in paragraphs:
                para_size = len(para.encode("utf-8"))
                if current_size + para_size + frontmatter_size > max_size:
                    if current_content:
                        chunk_content = "\n\n".join(current_content)
                        chunks.append(
                            MarkdownDoc(
                                content=chunk_content,
                                source_url=doc.source_url,
                                provider=doc.provider,
                                topic=f"{doc.topic} (Part {chunk_num})",
                                scraped_at=doc.scraped_at,
                            )
                        )
                        chunk_num += 1
                        current_content = []
                        current_size = 0
                current_content.append(para)
                current_size += para_size
        elif current_size + section_size + frontmatter_size > max_size:
            # Flush and start new chunk
            if current_content:
                chunk_content = "".join(current_content)
                chunks.append(
                    MarkdownDoc(
                        content=chunk_content,
                        source_url=doc.source_url,
                        provider=doc.provider,
                        topic=f"{doc.topic} (Part {chunk_num})",
                        scraped_at=doc.scraped_at,
                    )
                )
                chunk_num += 1
            current_content = [section]
            current_size = section_size
        else:
            current_content.append(section)
            current_size += section_size

    # Flush remaining content
    if current_content:
        chunk_content = "".join(current_content)
        topic = f"{doc.topic} (Part {chunk_num})" if chunk_num > 1 else doc.topic
        chunks.append(
            MarkdownDoc(
                content=chunk_content,
                source_url=doc.source_url,
                provider=doc.provider,
                topic=topic,
                scraped_at=doc.scraped_at,
            )
        )

    return chunks


def write_markdown_docs(docs: list[MarkdownDoc], output_dir: Path) -> list[Path]:
    """Write markdown documents to files."""
    output_dir.mkdir(parents=True, exist_ok=True)
    paths: list[Path] = []

    for i, doc in enumerate(docs):
        # Create safe filename from topic
        safe_topic = re.sub(r"[^\w\s-]", "", doc.topic.lower())
        safe_topic = re.sub(r"[\s]+", "-", safe_topic)
        filename = f"{safe_topic}.md" if len(docs) == 1 else f"{safe_topic}-{i + 1}.md"

        path = output_dir / filename
        content = format_markdown_with_frontmatter(doc)
        path.write_text(content, encoding="utf-8")
        paths.append(path)

    return paths
