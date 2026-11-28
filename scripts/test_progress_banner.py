"""Checks of progress_banner.py's task states: `python3 -m unittest scripts/test_progress_banner.py`."""
import json
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import progress_banner as pb  # noqa: E402

MERGES = [
    "Merge pull request #139 from Ma1achy/claude/field-sources-shipped-node-b1e8yu\tField sources as one node (TASK-M7-85)",
    "Merge pull request #137 from Ma1achy/task/TASK-M7-17-window\tPreset library in the window (TASK-M7-17)",
    "Merge pull request #144 from Ma1achy/claude/edge-kinds-evr5wn\tEdge kinds: wrap, clamp (TASK-M2-101 to 104)",
    "Merge pull request #90 from Ma1achy/claude/x\tOne registry (TASK-M7-76, TASK-M7-83)",
    "Merge pull request #111 from Ma1achy/claude/z\tWhat can matter in a stain (M7-89)",
    "Merge pull request #12 from Ma1achy/task/TASK-C1-01\tFirst frame",
    "Merge pull request #141 from Ma1achy/claude/gif-proof\tGIF proof with the recovered encoder settings",
    "Merge pull request #13 from Ma1achy/claude/y\tMentions TASK-M9-01 mid-title, not at the end",
    "Merge branch 'main' into task/TASK-M3-03\tFix up (TASK-M3-03)",
]
WANT = {"TASK-M7-85", "TASK-M7-17", "TASK-M2-101", "TASK-M2-102", "TASK-M2-103", "TASK-M2-104", "TASK-M7-76",
        "TASK-M7-83", "TASK-C1-01", "TASK-M7-89"}


class States(unittest.TestCase):
    def test_merged_ids(self):
        self.assertEqual(pb.merged_ids(MERGES), WANT)

    def test_merged_ids_control(self):
        # Control: reading only the head branch (the old rule) must miss the cloud-branch merges, so the check above
        # can tell the two apart.
        real = pb.title_ids
        pb.title_ids = lambda _t: set()
        try:
            self.assertNotEqual(pb.merged_ids(MERGES), WANT)
        finally:
            pb.title_ids = real

    def test_open_prs(self):
        prs = [
            {"headRefName": "claude/ftle-nvv4jh", "title": "FTLE chart independence (TASK-M3-69)", "isCrossRepository": False},
            {"headRefName": "task/TASK-M7-86", "title": "Field ramps", "isCrossRepository": False},
            {"headRefName": "task/TASK-M8-01", "title": "From a fork (TASK-M8-02)", "isCrossRepository": True},
        ]
        with tempfile.NamedTemporaryFile("w", suffix=".json", delete=False) as f:
            json.dump(prs, f)
        self.assertEqual(pb.open_pr_ids(f.name), {"TASK-M3-69", "TASK-M7-86"})


if __name__ == "__main__":
    unittest.main()
