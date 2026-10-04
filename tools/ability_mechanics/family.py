"""The interface a family of designer text implements, and its two outcomes.

A family decides which effects are its candidates (loosely: anything its text
talks about) and parses each one into NVP rows plus, optionally, a script to
bind, or rejects it with a reason. A candidate the grammar refuses is
reported, so the report is the list a reviewer reads to widen the grammar.
"""

from __future__ import annotations

import re
from dataclasses import dataclass, field
from typing import List, Optional, Tuple, Union

from corpus import Corpus, Effect


@dataclass
class Generated:
    """What a family writes for one effect."""

    effect: Effect
    nvps: List[Tuple[str, str]]  # (name, value)
    script: Optional[str]  # script_name to bind, or None to leave the column alone
    source: str  # the clause the numbers came from, quoted in the comment
    notes: List[str] = field(default_factory=list)


@dataclass
class Rejected:
    effect: Effect
    reason: str


Outcome = Union[Generated, Rejected]


class Family:
    """One family. Subclasses set the class attributes and implement
    ``is_candidate`` and ``parse``; ``families/__init__.py`` registers them."""

    name: str = ""
    nvp_names: frozenset = frozenset()  # the NVP names its rows use
    scripts: frozenset = frozenset()  # every script_name it may bind
    # Reason prefixes ("conditional: ...") the report counts, when the
    # family tags its rejections that way.
    reason_categories: tuple = ()

    def is_candidate(self, effect: Effect, corpus: Corpus) -> bool:
        raise NotImplementedError

    def parse(self, effect: Effect, corpus: Corpus) -> Outcome:
        raise NotImplementedError


def desc_lines(text: str) -> List[str]:
    """Designer text as stripped, whitespace-collapsed, non-empty lines."""
    return [re.sub(r"\s+", " ", ln).strip() for ln in text.split("\n") if ln.strip()]


def show(text: str) -> str:
    """Designer text on one line, for comments and the report."""
    return '"' + "\\n".join(desc_lines(text)) + '"'
