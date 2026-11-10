"""Every tap target of the phone's Compose screens: its size and whether TalkBack can name it.

For each `.press(`, `.clickable(` and `combinedClickable(` under
android/app/src/main/java/.../ui/, finds the composable call it belongs to
(the call whose argument list holds the modifier chain), reads the sizes the
chain sets before the tap (`.size(…dp)`, `.size(w.dp, h.dp)`, `.height(…dp)`,
`.width(…dp)`), and looks inside the call's content lambda for text
(`BasicText(`, `Line(`, `Text(`, or a call whose name ends in `Text`/`Line`).
A target with no text inside has no name for TalkBack: every icon is a
`Glyph`, drawn with `contentDescription = null` (ui/components/Basics.kt:41),
and no `semantics` is set anywhere.

This is a static reading, not a layout: sizes set by a parent, by padding or
at run time are not seen, and text passed in through a parameter is not seen
either. Each line says what was found so it can be checked by hand.

Usage: python3 -I phone_targets.py <repository root>
"""

import pathlib
import re
import sys

TAP = re.compile(r"\.press\(|\.clickable\(|combinedClickable\(")
SIZE = re.compile(r"\.(size|height|width|requiredSize|defaultMinSize|sizeIn)\(([^()]*)\)")
TEXT = re.compile(r"\b(BasicText|Line|Text|\w+Text|\w+Line)\(")


def matching(src: str, i: int, open_c: str, close_c: str) -> int:
    """Index of the bracket closing the one at src[i]."""
    depth = 0
    in_str = False
    while i < len(src):
        c = src[i]
        if in_str:
            if c == "\\":
                i += 2
                continue
            if c == '"':
                in_str = False
        elif c == '"':
            in_str = True
        elif c == open_c:
            depth += 1
        elif c == close_c:
            depth -= 1
            if depth == 0:
                return i
        i += 1
    return -1


def enclosing_call(src: str, at: int):
    """The '(' of the innermost call whose argument list contains `at`."""
    depth = 0
    i = at
    while i > 0:
        i -= 1
        c = src[i]
        if c == ")":
            depth += 1
        elif c == "(":
            if depth == 0:
                return i
            depth -= 1
        elif c in "{}" and depth == 0:
            return None
    return None


def dp_values(chain: str):
    out = []
    for kind, args in SIZE.findall(chain):
        nums = [float(n) for n in re.findall(r"([0-9.]+)\.dp", args)]
        if nums:
            out.append((kind, nums))
    return out


def main(root: str) -> None:
    base = pathlib.Path(root)
    ui = base / "android/app/src/main/java/io/github/honeyslatt/opium/ui"
    small = unnamed = total = 0
    for path in sorted(ui.rglob("*.kt")):
        src = path.read_text()
        rel = path.relative_to(base)
        for m in TAP.finditer(src):
            if src[max(0, m.start() - 4):m.start()].endswith("fun "):
                continue
            total += 1
            line = src.count("\n", 0, m.start()) + 1
            call_open = enclosing_call(src, m.start())
            if call_open is None:
                print(f"{rel}:{line}\t?\tno enclosing call found")
                continue
            call_close = matching(src, call_open, "(", ")")
            # The name of the call: the word before its '('.
            name = re.search(r"([A-Za-z_]\w*)\s*$", src[:call_open])
            name = name.group(1) if name else "?"
            chain = src[call_open:m.start()]
            sizes = dp_values(chain)
            # Its content: a trailing lambda right after the call.
            rest = src[call_close + 1:call_close + 3]
            content = ""
            j = call_close + 1
            while j < len(src) and src[j] in " \t":
                j += 1
            if j < len(src) and src[j] == "{":
                end = matching(src, j, "{", "}")
                content = src[j:end + 1]
            # The tap's own lambda may hold the text (Box(...press { ... }) has none).
            has_text = bool(TEXT.search(content)) or name in ("BasicText", "Line", "Text")
            smallest = min((min(n) for _, n in sizes), default=None)
            flags = []
            if smallest is not None and smallest < 48:
                flags.append(f"UNDER-48dp({smallest:g})")
                small += 1
            if not has_text:
                flags.append("NO-TEXT")
                unnamed += 1
            size_txt = "; ".join(f"{k} {'x'.join(f'{v:g}' for v in n)}" for k, n in sizes) or "-"
            print(f"{rel}:{line}\t{name}\t{size_txt}\t{' '.join(flags) or 'ok'}")
    print(f"\n{total} tap targets; {small} declare a size under 48 dp; {unnamed} hold no text")


if __name__ == "__main__":
    main(sys.argv[1])
