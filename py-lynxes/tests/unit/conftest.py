"""
Shared pytest fixtures for Lynxes Python binding tests (TST-009).
"""
import os
import tempfile

import pytest
import lynxes as gf


# ── Fixture data ──────────────────────────────────────────────────────────────

_SOCIAL_GF = """\
(alice: Person { age: 30, score: 0.9 })
(bob: Person { age: 22, score: 0.6 })
(charlie: Person { age: 35, score: 0.75 })
(diana: Person { age: 28, score: 0.8 })
(acme: Company { age: 100, score: 0.5 })
alice -[KNOWS]-> bob
bob -[KNOWS]-> charlie
alice -[KNOWS]-> diana
diana -[WORKS_AT]-> acme
"""

# ── Fixtures ──────────────────────────────────────────────────────────────────


@pytest.fixture(scope="session")
def gf_path(tmp_path_factory):
    """Write the social graph to a temp .gf file (once per session)."""
    p = tmp_path_factory.mktemp("data") / "social.gf"
    p.write_text(_SOCIAL_GF)
    return str(p)


@pytest.fixture(scope="session")
def graph(gf_path):
    """Loaded GraphFrame from the social fixture."""
    return gf.read_gf(gf_path)


@pytest.fixture(scope="session")
def tmp_dir(tmp_path_factory):
    return tmp_path_factory.mktemp("output")
