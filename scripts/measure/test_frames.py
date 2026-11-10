#!/usr/bin/env python3
"""Tests of frames.py's check that a `scroll-songs` run covered its list (the stage 1 audit's W12).

usage: python3 -I scripts/measure/test_frames.py        (inside `nix develop`)

Runs frames.py as it is used to sum up old records (`--again`), on small records made by hand in a
temporary folder: no window, no app. Set FRAMES_PY to try another copy of the script.
"""
import json
import os
import subprocess
import sys
import tempfile
import unittest

SCRIPT = os.environ.get("FRAMES_PY") or os.path.join(
    os.path.dirname(os.path.abspath(__file__)), "frames.py")

# The audit's list: 8,500 songs of 52 points, in a window of 2327x1309 points (4K at 1.65).
ROWS = 442_000
WINDOW = "2327x1309"


def record(scenario, **given):
    """A record's text, with the facts of a run that went well, changed by `given`; a fact given
    as None is left out."""
    facts = {"scenario": scenario, "songs": 8500, "rows_points": ROWS,
             "pixels_per_point": 1.65, "window_points": WINDOW, "scrolled": 441_064,
             "ended": "end", "undrawn": 0, **given}
    lines = ["# opium frame record"]
    lines += [f"# {key}={value}" for key, value in facts.items() if value is not None]
    lines += ["start_us,cpu_us,phase,continued,scrolled",
              "1000000,2000,loading,0,0",
              "1100000,2000,useful,0,0",
              "1200000,2000,scroll,1,100",
              "1204166,,end,1,100"]
    return "\n".join(lines) + "\n"


def sum_up(scenario, text):
    """What frames.py says of run 0 of a record: its `failed` field, None when it stands."""
    with tempfile.TemporaryDirectory() as out:
        with open(os.path.join(out, f"{scenario}-0.csv"), "w") as csv:
            csv.write(text)
        with open(os.path.join(out, f"{scenario}-0-mem.json"), "w") as mem:
            json.dump({"started": 0.9, "timed_out": False, "samples": []}, mem)
        env = {k: v for k, v in os.environ.items() if k != "OPIUM_MEASURE_PIXELS"}
        done = subprocess.run([sys.executable, "-I", SCRIPT, "--again", scenario, "1", out],
                              env=env, capture_output=True, text=True, check=True)
        with open(os.path.join(out, f"{scenario}.json")) as summed:
            failed = json.load(summed)["runs"][0]["failed"]
    return failed, done.stderr


class ScrollCoversTheList(unittest.TestCase):
    def songs(self, scrolled=441_064, **given):
        return sum_up("scroll-songs", record("scroll-songs", scrolled=scrolled, **given))

    def test_a_run_that_went_the_whole_way_stands(self):
        # The audit's run that did: 441,064 points of a list of 8,500 rows of 52.
        self.assertIsNone(self.songs()[0])

    def test_a_run_that_stopped_short_is_unfit(self):
        for scrolled in (7_066, 411_933):
            failed, said = self.songs(scrolled)
            self.assertIn(f"scrolled {scrolled} points", failed)
            self.assertIn("442000", failed)
            self.assertIn("left out", said)

    def test_a_run_that_went_too_far_is_unfit(self):
        failed, _ = self.songs(853_559)
        self.assertIn("scrolled 853559 points", failed)

    def test_the_edges_are_the_rows_less_a_window_and_the_rows_plus_a_banner_and_a_late_step(self):
        least = ROWS - 1309
        most = ROWS + 629 + 1000
        self.assertIsNone(self.songs(least)[0])
        self.assertIsNone(self.songs(most)[0])
        self.assertIsNotNone(self.songs(least - 1)[0])
        self.assertIsNotNone(self.songs(most + 1)[0])

    def test_a_run_a_few_thousand_points_short_is_unfit(self):
        # The review's: 436,300 of 442,000 rows, 4,764 short of the page's end, once passed.
        failed, _ = self.songs(436_300)
        self.assertIn("scrolled 436300 points", failed)

    def test_a_short_list_is_checked_against_its_own_length(self):
        # 100 rows: the page ends at 5,200 + 372 of header and foot - 1,309 of window.
        self.assertIsNone(self.songs(4_263, rows_points=5_200)[0])
        self.assertIsNotNone(self.songs(0, rows_points=5_200)[0])

    def test_a_distance_that_is_not_a_number_is_unfit(self):
        self.assertIsNotNone(self.songs("nan")[0])

    def test_an_older_record_without_rows_points_takes_its_songs_at_52_points(self):
        failed, said = self.songs(rows_points=None)
        self.assertIsNone(failed)
        self.assertIn("songs x 52", said)
        self.assertIn("scrolled 7066 points", self.songs(7_066, rows_points=None)[0])

    def test_a_record_that_says_neither_the_rows_nor_the_songs_is_left_out(self):
        for missing in (("rows_points", "songs"), ("window_points",), ("scrolled",)):
            failed, _ = sum_up("scroll-songs", record("scroll-songs", **dict.fromkeys(missing)))
            self.assertIn("left out", failed, missing)

    def test_the_albums_grid_is_not_held_to_the_songs_rows(self):
        # Its length is not in the record: it stands or falls on the other checks.
        text = record("scroll-albums", rows_points=None, scrolled=7_066)
        self.assertIsNone(sum_up("scroll-albums", text)[0])

    def test_the_other_checks_still_hold(self):
        failed, _ = self.songs(ended="limit")
        self.assertIn("ended: limit", failed)


if __name__ == "__main__":
    unittest.main()
