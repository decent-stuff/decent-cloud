"""Tests for CLI module."""

import pytest
from pathlib import Path
from unittest.mock import AsyncMock, patch, MagicMock
import sys

from scraper.cli import run_scraper, main, cli, SCRAPERS


@pytest.fixture
def temp_output_base(tmp_path):
    """Create temp output base directory."""
    return tmp_path / "output"


class TestRunScraper:
    """Test run_scraper function."""

    @pytest.mark.asyncio
    async def test_successful_scrape(self, temp_output_base, capsys):
        """Test successful scraper run."""
        csv_path = temp_output_base / "hetzner" / "offerings.csv"
        csv_path.parent.mkdir(parents=True, exist_ok=True)

        # Write test CSV with header + 3 offerings
        csv_path.write_text("header\noffering1\noffering2\noffering3\n")

        mock_scraper = AsyncMock()
        mock_scraper.run = AsyncMock(return_value=(csv_path, 5))

        with patch("scraper.cli.SCRAPERS") as mock_scrapers:
            mock_scrapers.__getitem__ = MagicMock(return_value=MagicMock(return_value=mock_scraper))
            offerings_count, docs_count = await run_scraper("hetzner", temp_output_base)

        assert offerings_count == 3
        assert docs_count == 5

        captured = capsys.readouterr()
        assert "=== Scraping hetzner ===" in captured.out
        assert "Offerings: 3" in captured.out
        assert "Docs: 5 new/changed" in captured.out

    @pytest.mark.asyncio
    async def test_csv_does_not_exist(self, temp_output_base, capsys):
        """Test scraper run when CSV doesn't exist yet."""
        csv_path = temp_output_base / "hetzner" / "offerings.csv"

        mock_scraper = AsyncMock()
        mock_scraper.run = AsyncMock(return_value=(csv_path, 3))

        with patch("scraper.cli.SCRAPERS") as mock_scrapers:
            mock_scrapers.__getitem__ = MagicMock(return_value=MagicMock(return_value=mock_scraper))
            offerings_count, docs_count = await run_scraper("hetzner", temp_output_base)

        assert offerings_count == 0  # CSV doesn't exist
        assert docs_count == 3

        captured = capsys.readouterr()
        assert "Offerings: 0" in captured.out
        assert "Docs: 3 new/changed" in captured.out

    @pytest.mark.asyncio
    async def test_scraper_failure(self, temp_output_base, capsys):
        """Test scraper run handles exceptions."""
        mock_scraper = AsyncMock()
        mock_scraper.run = AsyncMock(side_effect=RuntimeError("Network error"))

        with patch("scraper.cli.SCRAPERS") as mock_scrapers:
            mock_scrapers.__getitem__ = MagicMock(return_value=MagicMock(return_value=mock_scraper))
            offerings_count, docs_count = await run_scraper("hetzner", temp_output_base)

        assert offerings_count == 0
        assert docs_count == 0

        captured = capsys.readouterr()
        assert "ERROR: Network error" in captured.out

    @pytest.mark.asyncio
    async def test_no_docs_changed(self, temp_output_base, capsys):
        """Test scraper run when no docs changed."""
        csv_path = temp_output_base / "contabo" / "offerings.csv"
        csv_path.parent.mkdir(parents=True, exist_ok=True)
        csv_path.write_text("header\noffering1\n")

        mock_scraper = AsyncMock()
        mock_scraper.run = AsyncMock(return_value=(csv_path, 0))

        with patch("scraper.cli.SCRAPERS") as mock_scrapers:
            mock_scrapers.__getitem__ = MagicMock(return_value=MagicMock(return_value=mock_scraper))
            offerings_count, docs_count = await run_scraper("contabo", temp_output_base)

        assert offerings_count == 1
        assert docs_count == 0

        captured = capsys.readouterr()
        assert "Docs: 0 new/changed" in captured.out


class TestMain:
    """Test main async function."""

    @pytest.mark.asyncio
    async def test_single_provider(self, temp_output_base, capsys):
        """Test running single provider."""
        csv_path = temp_output_base / "hetzner" / "offerings.csv"
        csv_path.parent.mkdir(parents=True, exist_ok=True)
        csv_path.write_text("header\noffering1\noffering2\n")

        mock_scraper = AsyncMock()
        mock_scraper.run = AsyncMock(return_value=(csv_path, 3))
        mock_cls = MagicMock(return_value=mock_scraper)

        with patch.dict(SCRAPERS, {"hetzner": mock_cls}):
            with patch.object(sys, "argv", ["cli.py", "hetzner"]):
                with patch("scraper.cli.Path") as mock_path:
                    mock_path.return_value.parent.parent.__truediv__.return_value = temp_output_base
                    await main()

        captured = capsys.readouterr()
        assert "=== Scraping hetzner ===" in captured.out
        assert "=== Summary ===" in captured.out
        assert "Total: 2 offerings, 3 docs" in captured.out

    @pytest.mark.asyncio
    async def test_all_providers(self, temp_output_base, capsys):
        """Test running all providers."""
        # Create CSV files for all providers
        for provider in ["hetzner", "contabo", "ovh"]:
            csv_path = temp_output_base / provider / "offerings.csv"
            csv_path.parent.mkdir(parents=True, exist_ok=True)
            csv_path.write_text("header\noffering1\n")

        def create_mock(provider_name):
            """Create mock for specific provider."""
            mock_scraper = AsyncMock()
            csv_path = temp_output_base / provider_name / "offerings.csv"
            mock_scraper.run = AsyncMock(return_value=(csv_path, 2))
            return MagicMock(return_value=mock_scraper)

        mock_scrapers = {
            "hetzner": create_mock("hetzner"),
            "contabo": create_mock("contabo"),
            "ovh": create_mock("ovh"),
        }

        with patch.dict(SCRAPERS, mock_scrapers):
            with patch.object(sys, "argv", ["cli.py"]):
                with patch("scraper.cli.Path") as mock_path:
                    mock_path.return_value.parent.parent.__truediv__.return_value = temp_output_base
                    await main()

        captured = capsys.readouterr()
        assert "=== Scraping hetzner ===" in captured.out
        assert "=== Scraping contabo ===" in captured.out
        assert "=== Scraping ovh ===" in captured.out
        assert "=== Summary ===" in captured.out

    @pytest.mark.asyncio
    async def test_unknown_provider(self, capsys):
        """Test unknown provider exits with error."""
        with patch.object(sys, "argv", ["cli.py", "unknown"]):
            with pytest.raises(SystemExit) as exc_info:
                await main()

        assert exc_info.value.code == 1
        captured = capsys.readouterr()
        assert "Unknown provider: unknown" in captured.out
        assert "Available: hetzner, contabo, ovh" in captured.out

    @pytest.mark.asyncio
    async def test_multiple_providers(self, temp_output_base, capsys):
        """Test running multiple specific providers."""
        # Create CSV files
        for provider in ["hetzner", "ovh"]:
            csv_path = temp_output_base / provider / "offerings.csv"
            csv_path.parent.mkdir(parents=True, exist_ok=True)
            csv_path.write_text("header\noffering1\noffering2\n")

        hetzner_scraper = AsyncMock()
        ovh_scraper = AsyncMock()
        hetzner_csv = temp_output_base / "hetzner" / "offerings.csv"
        ovh_csv = temp_output_base / "ovh" / "offerings.csv"
        hetzner_scraper.run = AsyncMock(return_value=(hetzner_csv, 1))
        ovh_scraper.run = AsyncMock(return_value=(ovh_csv, 1))

        mock_scrapers = {
            "hetzner": MagicMock(return_value=hetzner_scraper),
            "ovh": MagicMock(return_value=ovh_scraper),
        }

        with patch.dict(SCRAPERS, mock_scrapers, clear=False):
            with patch.object(sys, "argv", ["cli.py", "hetzner", "ovh"]):
                with patch("scraper.cli.Path") as mock_path:
                    mock_path.return_value.parent.parent.__truediv__.return_value = temp_output_base
                    await main()

        captured = capsys.readouterr()
        assert "=== Scraping hetzner ===" in captured.out
        assert "=== Scraping ovh ===" in captured.out
        assert "=== Scraping contabo ===" not in captured.out


class TestCLI:
    """Test CLI entry point."""

    def test_cli_entry_point(self):
        """Test cli() calls asyncio.run(main())."""
        with patch("scraper.cli.asyncio.run") as mock_run:
            with patch("scraper.cli.main") as mock_main:
                cli()

        mock_run.assert_called_once()
        # Verify it was called with the coroutine from main()
        call_args = mock_run.call_args[0][0]
        assert hasattr(call_args, "__await__")  # It's a coroutine
