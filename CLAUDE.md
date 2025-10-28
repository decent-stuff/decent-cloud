# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

**Note**: This project uses AGENTS.md files for detailed guidance. 

## Primary Reference

@AGENTS.md

## Updating AGENTS.md Files

When you discover new information that would be helpful for future development work, please:

- **Update existing AGENTS.md files** when you learn implementation details, debugging insights, or architectural patterns specific to that component
- **Create new AGENTS.md files** in relevant directories when working with areas that don't yet have documentation
- **Add valuable insights** such as common pitfalls, debugging techniques, dependency relationships, or implementation patterns

This helps build a comprehensive knowledge base for the codebase over time
- Immediately after every change run 'cargo make' to ensure tests are green and add tests to ensure good test coverage.
- IFF 'cargo make' is completely clean and exits without an error, commit changes
