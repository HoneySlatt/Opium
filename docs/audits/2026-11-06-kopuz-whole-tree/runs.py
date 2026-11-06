#!/usr/bin/env python3
"""Prepares the reading by hand of a token comparison's normalised runs (docs/specs/README.md,
"The check afterwards"), and joins the reading back to the runs.

    runs.py split REPORT.md OUT_DIR PARTS [--root NAME=PATH...]
    runs.py join REPORT.md READING.tsv... > review.tsv

`split` takes every normalised run of REPORT.md, the same range of Opium's once whatever source
and revision it was found in, and writes OUT_DIR/runs-NN.txt in PARTS parts, ordered by file:
each run with its lines on both sides. It also gives each run two aids to the reading, which play
no part in the thresholds:

- the identifier agreement: the share of the run's identifiers that are the same on both sides,
  1.0 when the names match too;
- the period: the run's shape repeats every so many tokens, as a list does (imports, fields).

A reader writes, for each part, a READING.tsv of `<run id> TAB <class> TAB <why>` lines.

`join` writes the review: one line per run, Opium's file and lines, the source's file and lines,
the run's length, its agreement, its period, its class and why. Run from the repository's root. Standard
library only, with token_compare.py beside M1.10's evidence.
"""

import argparse
import os
import re
import sys

sys.dont_write_bytecode = True
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "2026-11-05-cenc-clean-room"))
import token_compare  # noqa: E402

RUN = re.compile(
    r"### (\d+) normalised tokens: `([^`]+)` lines (\d+) to (\d+), and (\S+) `([^`]+)` lines (\d+) to (\d+)\n"
)


def runs_of(report):
    """The normalised runs, by Opium's range: [(length, target, first, last, [(source, file, first,
    last)])], in the report's order."""
    with open(report, encoding="utf-8") as f:
        text = f.read()
    part = text[text.index("## Normalised tokens") : text.index("## Names, strings")]
    ranges = {}
    for n, target, a, b, source, path, c, d in RUN.findall(part):
        key = (target, int(a), int(b))
        if key not in ranges:
            ranges[key] = (int(n), [])
        ranges[key][1].append((source, path, int(c), int(d)))
    return [(n, t, a, b, sources) for (t, a, b), (n, sources) in ranges.items()]


def period(keys, longest=12):
    """The shortest period, up to `longest`, that 90 % of the run's tokens follow, or None."""
    for p in range(1, longest + 1):
        same = sum(1 for i in range(len(keys) - p) if keys[i] == keys[i + p])
        if same >= 0.9 * (len(keys) - p):
            return p
    return None


def agreement(n, ours, theirs):
    """The share of identifiers that are the same where the run's `n` normalised tokens align."""
    ours_keys = [token_compare.key(t, True) for t in ours]
    theirs_keys = [token_compare.key(t, True) for t in theirs]
    for i in range(len(ours_keys) - n + 1):
        for j in range(len(theirs_keys) - n + 1):
            if ours_keys[i : i + n] == theirs_keys[j : j + n]:
                pairs = [(x.text, y.text) for x, y in zip(ours[i : i + n], theirs[j : j + n]) if x.kind in ("id", "life")]
                return sum(1 for x, y in pairs if x == y) / max(1, len(pairs)), ours_keys[i : i + n]
    return None, []


class Files:
    """Each file's tokens and lines, read once."""

    def __init__(self, roots):
        self.roots, self.tokens, self.lines = roots, {}, {}

    def path(self, source, path):
        return os.path.join(self.roots.get(source, "."), path)

    def tokens_in(self, path, first, last):
        if path not in self.tokens:
            self.tokens[path] = token_compare.tokens_of(path)[0]
        return [t for t in self.tokens[path] if first <= t.line <= last]

    def shown(self, path, first, last):
        if path not in self.lines:
            with open(path, encoding="utf-8", errors="replace") as f:
                self.lines[path] = f.read().split("\n")
        lines = self.lines[path]
        first, last = max(1, first - 1), min(len(lines), last + 1)
        return "\n".join(f"{i:5}| {lines[i - 1][:160]}" for i in range(first, last + 1))


def measured(runs, files):
    """Each run with its id, its agreement (the best over its sources) and its period."""
    out = []
    for number, (n, target, a, b, sources) in enumerate(runs, start=1):
        ours = files.tokens_in(target, a, b)
        best, shape = None, []
        for source, path, c, d in sources:
            agree, keys = agreement(n, ours, files.tokens_in(files.path(source, path), c, d))
            if agree is not None and (best is None or agree > best):
                best, shape = agree, keys
        out.append(dict(id=number, n=n, target=target, a=a, b=b, sources=sources,
                        agree=round(best, 2) if best is not None else None, period=period(shape)))
    return out


def split(args):
    files = Files(dict(item.split("=", 1) for item in args.root))
    items = sorted(measured(runs_of(args.report), files), key=lambda i: (i["target"], i["a"]))
    os.makedirs(args.out, exist_ok=True)
    size = -(-len(items) // args.parts)
    for part in range(args.parts):
        blocks = []
        for i in items[part * size : (part + 1) * size]:
            source, path, c, d = i["sources"][0]
            also = "; ".join(f"{s} {p} {c2}-{d2}" for s, p, c2, d2 in i["sources"][1:])
            blocks.append(
                f"=== RUN {i['id']}: {i['n']} normalised tokens; identifier agreement {i['agree']}; "
                f"periodic {i['period']}\n"
                f"--- OPIUM {i['target']} lines {i['a']}-{i['b']}\n{files.shown(i['target'], i['a'], i['b'])}\n"
                f"--- {source} {path} lines {c}-{d}" + (f" (also: {also})" if also else "") + "\n"
                f"{files.shown(files.path(source, path), c, d)}\n"
            )
        with open(os.path.join(args.out, f"runs-{part + 1:02}.txt"), "w", encoding="utf-8") as f:
            f.write("\n".join(blocks))
    print(f"{len(items)} runs in {args.parts} parts", file=sys.stderr)


def join(args):
    files = Files(dict(item.split("=", 1) for item in args.root))
    reading = {}
    for path in args.reading:
        with open(path, encoding="utf-8") as f:
            for line in f:
                fields = line.rstrip("\n").split("\t")
                if len(fields) >= 3 and fields[0].isdigit():
                    reading[int(fields[0])] = (fields[1], fields[2])
    print("opium\tsource\tnormalised tokens\tidentifier agreement\tperiod\tclass\twhy")
    for i in measured(runs_of(args.report), files):
        source, path, c, d = i["sources"][0]
        kind, why = reading.get(i["id"], ("unread", ""))
        print(
            f"{i['target']}:{i['a']}-{i['b']}\t{source} {path}:{c}-{d}\t{i['n']}\t{i['agree']}\t"
            f"{i['period'] or ''}\t{kind}\t{why}"
        )


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    commands = parser.add_subparsers(dest="command", required=True)
    s = commands.add_parser("split")
    s.add_argument("report")
    s.add_argument("out")
    s.add_argument("parts", type=int)
    j = commands.add_parser("join")
    j.add_argument("report")
    j.add_argument("reading", nargs="+")
    for command in (s, j):
        command.add_argument("--root", nargs="*", default=[], help="SOURCE=PATH, where a source's files are")
    args = parser.parse_args()
    split(args) if args.command == "split" else join(args)
    return 0


if __name__ == "__main__":
    sys.exit(main())
