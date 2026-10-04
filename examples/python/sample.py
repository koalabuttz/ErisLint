# Fork-specific Python example. SPDX-License-Identifier: AGPL-3.0-only
"""Source inspection does not load this deliberately missing dependency."""
from missing_dependency import decorate, Base


@decorate(mode="example")
class Reader(Base):
    """A decorated class with an asynchronous method."""

    async def read(self, limit: int = 3) -> list[str]:
        """Keep raw annotations and docstrings as evidence."""
        def normalize(value: str) -> str:
            return value.strip()

        # Runtime behavior of Base and decorate is unknown.
        return [normalize(value) for value in await self.fetch(limit)]
