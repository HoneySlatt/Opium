#!/usr/bin/env python3
"""Finds the runs of tokens that new code shares with older code, for a clean-room check.

The method is in docs/specs/README.md ("The check afterwards"). For each target file and each
source, the script lists every maximal run of tokens found in both, twice:

- raw: the tokens as written, comments left out;
- normalised: every identifier made the same, so that renaming hides nothing.

Runs at or above the thresholds (50 raw, 30 normalised by default) are written out in full, to be
read by hand. The longest run below them is given for calibration. Runs made only of tokens found
in a `--forced` file (a header both sides must use) are left out.

It also searches the targets for the sources' names, string literals and comment phrases
(`--origin`), to be read by hand. An origin that is a directory is searched as one whole: its
names, strings, comment and prose phrases, and its data (long literals and numbers), with the files
on each side that hold them.

Usage, from the repository's root (the wrapper compare.sh beside this file gives the real call):

    token_compare.py REPORT.md --target FILE... --source NAME=PATH... \
        [--exclude PATH...] [--forced PATH...] [--origin NAME=PATH...] [--raw 50] [--normalised 30] \
        [--all-text]

A PATH that is a directory stands for every source file below it (.rs, .c, .cc, .cpp, .h), but
for build folders. With `--all-text`, it also stands for Kotlin, Java, JSON, shell, Python, Nix,
TOML, YAML, Nushell, Fluent, desktop entries, XML, SVG and Markdown files, each read by its own rules. Only Rust, C, C++,
Kotlin and Java are compared with identifiers made the same: in the other languages nearly every
word is an identifier, and once they are all made the same, every text looks alike. Standard
library only.
"""

import argparse
import os
import re
import sys
from collections import defaultdict

EXTENSIONS = (".rs", ".c", ".cc", ".cpp", ".h")
# `--all-text`: the other files, by the rules each is read with.
OTHER_LANGUAGES = {
    ".kt": "kotlin", ".kts": "kotlin", ".java": "kotlin",
    ".json": "json",
    ".sh": "hash", ".toml": "hash", ".yml": "hash", ".yaml": "hash", ".nu": "hash", ".ftl": "hash",
    ".properties": "hash", ".pro": "hash", ".desktop": "hash",
    ".py": "python",
    ".nix": "nix",
    ".xml": "markup", ".svg": "markup",
    ".md": "prose",
}
# The languages whose keywords keep a structure once identifiers are made the same.
NORMALISED_LANGUAGES = {"rust", "kotlin"}
SKIPPED_DIRECTORIES = {".git", "target", "target-android", "build", "node_modules", ".gradle"}

RUST_KEYWORDS = set(
    """as async await break const continue crate dyn else enum extern false fn for if impl in let
    loop match mod move mut pub ref return self Self static struct super trait true type unsafe
    use where while""".split()
)

KOTLIN_KEYWORDS = set(
    """as break class continue do else false for fun if in interface is null object package return
    super this throw true try typealias typeof val var when while by catch constructor delegate
    finally get import init set where abstract annotation companion const data enum external final
    inline inner internal lateinit open operator out override private protected public sealed
    suspend tailrec vararg""".split()
)

# Longest operators first, so that `>>=` is one token, not three.
OPERATORS = sorted(
    """>>= <<= ... ..= :: -> => == != <= >= && || += -= *= /= %= ^= &= |= << >> ..""".split(),
    key=len,
    reverse=True,
)

IDENTIFIER = re.compile(r"[A-Za-z_][A-Za-z0-9_]*")
NUMBER = re.compile(r"(0[xob][0-9A-Fa-f_]+|[0-9][0-9_]*(\.[0-9][0-9_]*)?([eE][+-]?[0-9]+)?)([a-z][a-z0-9]*)?")
RAW_STRING = re.compile(r'b?r(#*)"')


class Token:
    __slots__ = ("kind", "text", "line")

    def __init__(self, kind, text, line):
        self.kind = kind  # "id", "kw", "num", "str", "char", "life", "op"
        self.text = text
        self.line = line


def language_of(path):
    """The rules `path` is read by: "rust" (Rust, and C or C++ well enough) or one of
    OTHER_LANGUAGES'."""
    return OTHER_LANGUAGES.get(os.path.splitext(path)[1], "rust")


def tokenize(source, language="rust"):
    """The tokens of `source`, and its comments as (line, text). Prose's lines are its comments,
    so that its phrases are searched as comments' are."""
    if language == "prose":
        return tokenize_prose(source)
    c_like = language in ("rust", "kotlin", "json")
    hash_comments = language in ("hash", "python", "nix")
    keywords = KOTLIN_KEYWORDS if language == "kotlin" else RUST_KEYWORDS if language == "rust" else set()
    tokens, comments = [], []
    i, line, n = 0, 1, len(source)
    while i < n:
        c = source[i]
        if c == "\n":
            line += 1
            i += 1
        elif c.isspace():
            i += 1
        elif hash_comments and c == "#" and (i == 0 or source[i - 1].isspace()):
            # Only after a space: `nixpkgs#lychee`, `.#fuzz` and `${file#…}` are not comments.
            end = source.find("\n", i)
            end = n if end < 0 else end
            comments.append((line, source[i + 1 : end].strip()))
            i = end
        elif language == "markup" and source.startswith("<!--", i):
            end = source.find("-->", i)
            end = n if end < 0 else end + 3
            text = source[i + 4 : end - 3]
            comments.append((line, " ".join(text.split())))
            line += text.count("\n")
            i = end
        elif language in ("kotlin", "python") and source.startswith('"""', i):
            end = source.find('"""', i + 3)
            end = n if end < 0 else end + 3
            text = source[i:end]
            tokens.append(Token("str", text, line))
            line += text.count("\n")
            i = end
        elif not c_like and c == "'" and closing_quote(source, i, language) > 0:
            # A quoted string of shell, Python, Nix, YAML or XML, closed on its own line.
            end = closing_quote(source, i, language) + 1
            tokens.append(Token("str", source[i:end], line))
            i = end
        elif not c_like and c == "'":
            tokens.append(Token("op", c, line))
            i += 1
        elif not (c_like or language == "nix") and source.startswith("/*", i):
            tokens.append(Token("op", "/", line))
            i += 1
        elif c_like and source.startswith("//", i):
            end = source.find("\n", i)
            end = n if end < 0 else end
            comments.append((line, source[i + 2 : end].lstrip("/!").strip()))
            i = end
        elif source.startswith("/*", i):
            depth, j = 1, i + 2
            while j < n and depth:
                if source.startswith("/*", j):
                    depth, j = depth + 1, j + 2
                elif source.startswith("*/", j):
                    depth, j = depth - 1, j + 2
                else:
                    j += 1
            text = source[i + 2 : j - 2]
            comments.append((line, " ".join(text.replace("*", " ").split())))
            line += text.count("\n")
            i = j
        elif language == "rust" and RAW_STRING.match(source, i):
            m = RAW_STRING.match(source, i)
            close = '"' + m.group(1)
            end = source.find(close, m.end())
            end = n if end < 0 else end + len(close)
            text = source[i:end]
            tokens.append(Token("str", text, line))
            line += text.count("\n")
            i = end
        elif c == '"' or (language == "rust" and source.startswith('b"', i)):
            j = i + (2 if c == "b" else 1)
            while j < n and source[j] != '"':
                j += 2 if source[j] == "\\" else 1
            text = source[i : j + 1]
            tokens.append(Token("str", text, line))
            line += text.count("\n")
            i = j + 1
        elif c == "'" or (language == "rust" and source.startswith("b'", i)):
            # A character literal, or a lifetime: 'a without a closing quote.
            start = i + (2 if c == "b" else 1)
            m = re.match(r"\\.[^']*'|[^\\']'", source[start : start + 12])
            if m:
                end = start + m.end()
                tokens.append(Token("char", source[i:end], line))
                i = end
            else:
                m = IDENTIFIER.match(source, start)
                end = m.end() if m else start
                tokens.append(Token("life", source[i:end], line))
                i = max(end, i + 1)
        elif IDENTIFIER.match(source, i):
            m = IDENTIFIER.match(source, i)
            word = m.group()
            tokens.append(Token("kw" if word in keywords else "id", word, line))
            i = m.end()
        elif c.isdigit():
            m = NUMBER.match(source, i)
            tokens.append(Token("num", m.group(), line))
            i = m.end()
        else:
            for op in OPERATORS:
                if source.startswith(op, i):
                    tokens.append(Token("op", op, line))
                    i += len(op)
                    break
            else:
                tokens.append(Token("op", c, line))
                i += 1
    return tokens, comments


def closing_quote(source, start, language):
    """Where the `'` string opened at `start` closes on its line, or -1. Python escapes a quote
    with a backslash; shell, Nix, YAML and XML do not."""
    end = source.find("\n", start)
    end = len(source) if end < 0 else end
    j = start + 1
    while j < end:
        if language == "python" and source[j] == "\\":
            j += 2
        elif source[j] == "'":
            return j
        else:
            j += 1
    return -1


def tokenize_prose(source):
    """Markdown's words, numbers and marks as tokens, its lines as comments."""
    tokens, comments = [], []
    for line, text in enumerate(source.split("\n"), start=1):
        if text.strip():
            comments.append((line, text.strip()))
        for m in re.finditer(r"[A-Za-z_][A-Za-z0-9_]*|[0-9]+|\S", text):
            word = m.group()
            kind = "id" if word[0].isalpha() or word[0] == "_" else "num" if word[0].isdigit() else "op"
            tokens.append(Token(kind, word, line))
    return tokens, comments


def key(token, normalised):
    if normalised and token.kind in ("id", "life"):
        return token.kind
    return token.text


def files_of(path, excluded, all_text=False):
    extensions = EXTENSIONS + tuple(OTHER_LANGUAGES) if all_text else EXTENSIONS
    if os.path.isfile(path):
        return [path]
    found = []
    for root, directories, names in os.walk(path):
        directories[:] = sorted(
            d
            for d in directories
            if d not in SKIPPED_DIRECTORIES
            and not any(os.path.abspath(os.path.join(root, d)).startswith(e) for e in excluded)
        )
        for name in sorted(names):
            full = os.path.join(root, name)
            if name.endswith(extensions) and not any(os.path.abspath(full).startswith(e) for e in excluded):
                found.append(full)
    return found


def read(path):
    with open(path, encoding="utf-8", errors="replace") as f:
        return f.read()


def target_index(target_keys, k):
    """Where each k-gram of the targets is: gram -> [(target, position)]."""
    index = defaultdict(list)
    for target, keys in enumerate(target_keys):
        for at in range(len(keys) - k + 1):
            index[tuple(keys[at : at + k])].append((target, at))
    return index


def shared_runs(target_keys, source_keys, k, forced_grams):
    """Every maximal run of at least `k` tokens shared by a target and a source:
    (target, target start, length, source, source start)."""
    index = target_index(target_keys, k)
    runs = []
    for source, keys in enumerate(source_keys):
        for start in range(len(keys) - k + 1):
            places = index.get(tuple(keys[start : start + k]))
            if not places:
                continue
            for target, at in places:
                own = target_keys[target]
                # Only where the shared run begins: the token before differs, or there is none.
                if at > 0 and start > 0 and own[at - 1] == keys[start - 1]:
                    continue
                length = k
                while (
                    at + length < len(own)
                    and start + length < len(keys)
                    and own[at + length] == keys[start + length]
                ):
                    length += 1
                if any(tuple(own[i : i + k]) not in forced_grams for i in range(at, at + length - k + 1)):
                    runs.append((target, at, length, source, start))
    return runs


def shares_a_run(target_keys, source_keys, k):
    """Whether some run of `k` tokens is in both a target and a source."""
    grams = {tuple(keys[at : at + k]) for keys in target_keys for at in range(len(keys) - k + 1)}
    return any(
        tuple(keys[start : start + k]) in grams
        for keys in source_keys
        for start in range(len(keys) - k + 1)
    )


def longest_below(target_keys, source_keys, threshold):
    """The longest shared run shorter than `threshold`, by bisection: if a run of k tokens is
    shared, so is one of every shorter length."""
    low, high = 0, threshold - 1
    while low < high:
        middle = (low + high + 1) // 2
        if shares_a_run(target_keys, source_keys, middle):
            low = middle
        else:
            high = middle - 1
    return low


def shown(path, root):
    """`path` as the report shows it: from its source's root, so that no local folder appears."""
    if os.path.isfile(root):
        return os.path.basename(path)
    return os.path.relpath(path, root)


def strongest(runs):
    """The longest runs first, dropping those whose target range a longer one covers."""
    kept = []
    for run in sorted(runs, key=lambda r: (-r[2], r[0], r[1])):
        target, at, length = run[0], run[1], run[2]
        if not any(k[0] == target and k[1] <= at and at + length <= k[1] + k[2] for k in kept):
            kept.append(run)
    return kept


def snippet(tokens, start, length):
    lines = defaultdict(list)
    for t in tokens[start : start + length]:
        lines[t.line].append(t.text)
    return "\n".join(f"{line:5} | {' '.join(words)}" for line, words in sorted(lines.items()))


def tokens_of(path):
    return tokenize(read(path), language_of(path))


def compare(report, targets, sources, threshold, normalised, forced):
    mode = "normalised" if normalised else "raw"
    report.append(f"\n## {mode.capitalize()} tokens, runs of {threshold} or more\n")

    def keys_of(path, tokens):
        if normalised and language_of(path) not in NORMALISED_LANGUAGES:
            return []
        return [key(t, normalised) for t in tokens]

    target_tokens = [tokens_of(t)[0] for t in targets]
    target_keys = [keys_of(path, tokens) for path, tokens in zip(targets, target_tokens)]
    forced_grams = set()
    for path in forced:
        keys = keys_of(path, tokens_of(path)[0])
        forced_grams.update(tuple(keys[at : at + threshold]) for at in range(len(keys) - threshold + 1))
    summary, details = [], []
    for name, root, paths in sources:
        source_tokens = [tokens_of(p)[0] for p in paths]
        source_keys = [keys_of(path, tokens) for path, tokens in zip(paths, source_tokens)]
        runs = strongest(shared_runs(target_keys, source_keys, threshold, forced_grams))
        below = longest_below(target_keys, source_keys, threshold)
        longest = max((r[2] for r in runs), default=0)
        summary.append(f"| {name} | {len(paths)} | {len(runs)} | {longest or '-'} | {below} |")
        for target, at, length, source, start in runs:
            tokens = target_tokens[target]
            theirs = source_tokens[source]
            details.append(
                f"\n### {length} {mode} tokens: `{targets[target]}` lines {tokens[at].line} to "
                f"{tokens[at + length - 1].line}, and {name} `{shown(paths[source], root)}` lines "
                f"{theirs[start].line} to {theirs[start + length - 1].line}\n\n```text\n"
                f"{snippet(tokens, at, length)}\n```\n"
            )
    report.append("| Against | Files | Runs at or above | Longest at or above | Longest below |")
    report.append("|---|---|---|---|---|")
    report.extend(summary)
    report.extend(details)


def origins(report, targets, origin_files):
    """The sources' names, string literals and comment phrases found in the targets."""
    report.append("\n## Names, strings and comments of the sources found in the new code\n")
    target_ids = defaultdict(set)
    target_strings = defaultdict(set)
    target_comments = {}
    for target in targets:
        tokens, comments = tokens_of(target)
        for t in tokens:
            if t.kind == "id":
                target_ids[t.text].add(target)
            elif t.kind == "str":
                target_strings[t.text].add(target)
        target_comments[target] = " ".join(" ".join(text.lower().split()) for _, text in comments)
    for name, path in origin_files:
        tokens, comments = tokens_of(path)
        names = sorted(
            {t.text for t in tokens if t.kind == "id" and is_compound(t.text)}
        )
        shared = [f"`{n}` ({', '.join(sorted(os.path.basename(t) for t in target_ids[n]))})" for n in names if n in target_ids]
        report.append(f"\n### {name}: `{os.path.basename(path)}`\n")
        report.append(f"- Compound identifiers of 6 characters or more: {len(names)}; found in the new code: {len(shared)}.")
        if shared:
            report.append("  " + ", ".join(shared))
        strings = sorted({t.text for t in tokens if t.kind == "str" and len(t.text.split()) >= 3})
        found = [s for s in strings if s in target_strings]
        report.append(f"- String literals of three words or more: {len(strings)}; found: {len(found)}.")
        for s in found:
            report.append(f"  - `{s}`")
        phrases = set()
        for _, text in comments:
            phrases.update(phrases_of(text))
        found = sorted(p for p in phrases if any(p in c for c in target_comments.values()))
        report.append(f"- Comment phrases of six words: {len(phrases)}; found: {len(found)}.")
        for p in found:
            report.append(f"  - \"{p}\"")


def is_compound(word):
    """A name of 6 characters or more made of several words: `snake_case` or `camelCase`."""
    return len(word) >= 6 and ("_" in word.strip("_") or re.search(r"[a-z][A-Z]", word) is not None)


def phrases_of(text):
    """Every run of six words of `text`, in lower case."""
    words = " ".join(text.lower().split()).split()
    return {" ".join(words[at : at + 6]) for at in range(len(words) - 5)}


def literal(text):
    """A string token's content, without its quotes or prefix."""
    m = re.match(r'^(?:b?r#*)?("""|"|\')(.*?)\1#*$', text, re.S)
    return m.group(2) if m else text


FLUENT_MESSAGE = re.compile(r"^[A-Za-z][A-Za-z0-9_-]*\s*=\s*(\S.*)$", re.M)


def origins_of_tree(report, targets, name, root, files):
    """The names, strings, comment and prose phrases, and data of every file of a source found in
    the targets, each with the files that hold it on both sides."""
    report.append(f"\n## Names, strings, phrases and data of {name} found in the targets\n")
    ids, strings, contents, phrases = (defaultdict(set) for _ in range(4))
    for target in targets:
        tokens, comments = tokens_of(target)
        for t in tokens:
            if t.kind == "id" and is_compound(t.text):
                ids[t.text].add(target)
            elif t.kind == "str":
                strings[t.text].add(target)
                contents[literal(t.text)].add(target)
            elif t.kind == "num" and len(t.text.replace("_", "")) >= 6:
                contents[t.text.replace("_", "")].add(target)
        for _, text in comments:
            for phrase in phrases_of(text):
                phrases[phrase].add(target)

    found = {kind: defaultdict(set) for kind in ("names", "strings", "phrases", "data")}
    for path in files:
        source = read(path)
        tokens, comments = tokenize(source, language_of(path))
        shown_path = shown(path, root)
        test_start = next(
            (i for i in range(len(tokens) - 3) if tokens[i].text == "cfg" and tokens[i + 2].text == "test"),
            None,
        )
        in_test_folder = re.search(r"(^|/)(tests?|testdata|fixtures)/", shown_path) is not None
        for at, t in enumerate(tokens):
            if t.kind == "id" and t.text in ids:
                found["names"][t.text].add(shown_path)
            elif t.kind == "str":
                if len(t.text.split()) >= 3 and t.text in strings:
                    found["strings"][t.text].add(shown_path)
                content = literal(t.text)
                is_test = in_test_folder or (test_start is not None and at > test_start)
                # Data: a long literal with a digit anywhere, or any literal of 6 characters or more
                # in test code, as an id, a title or a fixture would be.
                if content in contents and (
                    (len(content) >= 10 and re.search(r"[0-9]", content)) or (is_test and len(content) >= 6)
                ):
                    found["data"][content].add(shown_path)
            elif t.kind == "num" and len(t.text.replace("_", "")) >= 6 and t.text.replace("_", "") in contents:
                found["data"][t.text.replace("_", "")].add(shown_path)
        if path.endswith(".ftl"):
            # A Fluent file's messages are the interface's words: searched as strings are.
            for m in FLUENT_MESSAGE.finditer(source):
                text = f'"{m.group(1).strip()}"'
                if len(text.split()) >= 3 and text in strings:
                    found["strings"][text].add(shown_path)
        for _, text in comments:
            for phrase in phrases_of(text):
                if phrase in phrases:
                    found["phrases"][phrase].add(shown_path)

    def listed(paths):
        paths = sorted(paths)
        more = f" and {len(paths) - 3} more" if len(paths) > 3 else ""
        return ", ".join(f"`{p}`" for p in paths[:3]) + more

    titles = {
        "names": ("Compound identifiers of 6 characters or more", ids),
        "strings": ("String literals and interface messages of three words or more", strings),
        "phrases": ("Comment and prose phrases of six words", phrases),
        "data": ("Data: literals of 10 characters or more with a digit, numbers of 6 digits or more, "
                 "and literals of 6 characters or more in the source's tests", contents),
    }
    for kind, (title, theirs) in titles.items():
        items = found[kind]
        report.append(f"\n### {title}: {len(items)}\n")
        for item in sorted(items):
            ours = theirs[item] if kind != "data" else contents[item]
            report.append(f"- `{item}`: ours {listed(ours)}; {name} {listed(items[item])}")


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("report")
    parser.add_argument("--target", nargs="+", required=True)
    parser.add_argument("--source", nargs="+", default=[], help="NAME=PATH")
    parser.add_argument("--exclude", nargs="*", default=[])
    parser.add_argument("--forced", nargs="*", default=[])
    parser.add_argument("--origin", nargs="*", default=[], help="NAME=PATH")
    parser.add_argument("--raw", type=int, default=50)
    parser.add_argument("--normalised", type=int, default=30)
    parser.add_argument("--all-text", action="store_true")
    args = parser.parse_args()

    excluded = [os.path.abspath(e) for e in args.exclude]
    sources = []
    for item in args.source:
        name, path = item.split("=", 1)
        sources.append((name, path, files_of(path, excluded, args.all_text)))
    origin_items = [tuple(item.split("=", 1)) for item in args.origin]
    origin_files = [(name, path) for name, path in origin_items if os.path.isfile(path)]
    origin_trees = [(name, path) for name, path in origin_items if os.path.isdir(path)]

    report = ["# Token comparison", "", "Targets:", ""]
    report.extend(f"- `{t}`" for t in args.target)
    report.append("")
    report.append("Sources:")
    report.append("")
    report.extend(f"- {name}: {len(paths)} files" for name, _, paths in sources)
    compare(report, args.target, sources, args.raw, False, args.forced)
    compare(report, args.target, sources, args.normalised, True, args.forced)
    if origin_files:
        origins(report, args.target, origin_files)
    for name, path in origin_trees:
        origins_of_tree(report, args.target, name, path, files_of(path, excluded, args.all_text))
    with open(args.report, "w", encoding="utf-8") as f:
        f.write("\n".join(report) + "\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
