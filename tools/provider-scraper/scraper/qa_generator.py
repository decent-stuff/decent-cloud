"""Generate Q&A content from provider offerings and docs using Anthropic LLM."""

import json
import logging
import os

import anthropic

from scraper.models import Offering

logger = logging.getLogger(__name__)

DEFAULT_MODEL = "claude-sonnet-4-20250514"
DEFAULT_MAX_TOKENS = 4096

QA_SYSTEM_PROMPT = """You are a technical writer creating FAQ content for a cloud hosting provider comparison site.

Generate clear, accurate Q&A pairs based on the provider information given. Focus on questions customers would ask when evaluating this provider:
- Pricing and billing
- Server specifications and options
- Locations and availability
- Features and limitations
- Technical specifications

Output format: JSON array of objects with "question" and "answer" fields.
Keep answers concise but informative. Include specific numbers/prices when available."""

QA_USER_PROMPT_TEMPLATE = """Generate FAQ Q&A pairs for {provider_name} ({provider_website}).

## Offerings Data (sample of {total_offerings} total):
{offerings_sample}

## Additional Context:
{docs_content}

Generate 15-25 Q&A pairs covering the most important customer questions. Return ONLY valid JSON."""


class QAGeneratorError(Exception):
    """Raised when Q&A generation fails."""


class QAGenerator:
    """Generate Q&A content using Anthropic API."""

    def __init__(
        self,
        api_key: str | None = None,
        base_url: str | None = None,
        model: str | None = None,
    ) -> None:
        """Initialize the generator.

        Args:
            api_key: Anthropic API key. Falls back to ANTHROPIC_API_KEY env var.
            base_url: Custom API base URL. Falls back to ANTHROPIC_BASE_URL env var.
            model: Model to use. Falls back to ANTHROPIC_MODEL env var or default.
        """
        self.api_key = api_key or os.environ.get("ANTHROPIC_API_KEY")
        if not self.api_key:
            raise QAGeneratorError("ANTHROPIC_API_KEY environment variable required")

        self.base_url = base_url or os.environ.get("ANTHROPIC_BASE_URL")
        self.model = model or os.environ.get("ANTHROPIC_MODEL", DEFAULT_MODEL)

        client_kwargs: dict[str, str] = {"api_key": self.api_key}
        if self.base_url:
            client_kwargs["base_url"] = self.base_url

        self.client = anthropic.Anthropic(**client_kwargs)

    def generate_qa(
        self,
        provider_name: str,
        provider_website: str,
        offerings: list[Offering],
        docs_content: str = "",
        max_tokens: int = DEFAULT_MAX_TOKENS,
    ) -> list[dict[str, str]]:
        """Generate Q&A pairs from provider data.

        Args:
            provider_name: Name of the provider.
            provider_website: Provider's website URL.
            offerings: List of offerings to include.
            docs_content: Additional docs/context (markdown).
            max_tokens: Max tokens for response.

        Returns:
            List of Q&A dicts with "question" and "answer" keys.

        Raises:
            QAGeneratorError: If generation fails.
        """
        # Sample offerings if too many (keep diverse sample)
        sample = self._sample_offerings(offerings, max_sample=20)
        offerings_json = json.dumps([o.model_dump() for o in sample], indent=2)

        prompt = QA_USER_PROMPT_TEMPLATE.format(
            provider_name=provider_name,
            provider_website=provider_website,
            total_offerings=len(offerings),
            offerings_sample=offerings_json,
            docs_content=docs_content[:8000] if docs_content else "No additional docs available.",
        )

        logger.info(f"Generating Q&A for {provider_name} using {self.model}")

        try:
            response = self.client.messages.create(
                model=self.model,
                max_tokens=max_tokens,
                system=QA_SYSTEM_PROMPT,
                messages=[{"role": "user", "content": prompt}],
            )
        except anthropic.APIError as e:
            raise QAGeneratorError(f"Anthropic API error: {e}") from e

        # Extract text content
        text = ""
        for block in response.content:
            if block.type == "text":
                text += block.text

        # Parse JSON from response
        return self._parse_qa_response(text)

    def _sample_offerings(self, offerings: list[Offering], max_sample: int) -> list[Offering]:
        """Sample diverse offerings for context."""
        if len(offerings) <= max_sample:
            return offerings

        # Group by product_type and location, sample from each
        by_type: dict[str, list[Offering]] = {}
        for o in offerings:
            key = f"{o.product_type}:{o.datacenter_country}"
            by_type.setdefault(key, []).append(o)

        sampled: list[Offering] = []
        per_group = max(1, max_sample // len(by_type))

        for group in by_type.values():
            # Sort by price to get range
            sorted_group = sorted(group, key=lambda x: x.monthly_price)
            # Take first, middle, last
            indices = [0, len(sorted_group) // 2, -1]
            for i in indices[:per_group]:
                if sorted_group[i] not in sampled:
                    sampled.append(sorted_group[i])
                if len(sampled) >= max_sample:
                    return sampled

        return sampled[:max_sample]

    def _parse_qa_response(self, text: str) -> list[dict[str, str]]:
        """Parse Q&A JSON from LLM response."""
        # Find JSON array in response
        text = text.strip()

        # Handle markdown code blocks
        if "```json" in text:
            start = text.index("```json") + 7
            end = text.index("```", start)
            text = text[start:end].strip()
        elif "```" in text:
            start = text.index("```") + 3
            end = text.index("```", start)
            text = text[start:end].strip()

        # Find array bounds if not already clean
        if not text.startswith("["):
            start = text.find("[")
            end = text.rfind("]") + 1
            if start >= 0 and end > start:
                text = text[start:end]

        try:
            data = json.loads(text)
        except json.JSONDecodeError as e:
            raise QAGeneratorError(f"Failed to parse Q&A JSON: {e}\nResponse: {text[:500]}") from e

        if not isinstance(data, list):
            raise QAGeneratorError(f"Expected JSON array, got {type(data)}")

        # Validate structure
        qa_list: list[dict[str, str]] = []
        for item in data:
            if not isinstance(item, dict):
                continue
            if "question" in item and "answer" in item:
                qa_list.append({"question": str(item["question"]), "answer": str(item["answer"])})

        if not qa_list:
            raise QAGeneratorError("No valid Q&A pairs found in response")

        logger.info(f"Generated {len(qa_list)} Q&A pairs")
        return qa_list
