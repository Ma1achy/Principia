#!/usr/bin/env python3
"""Proof 6 of the WSL2 distro: the same command, at the same commit, in WSL and on GitHub's hosted Linux runner,
compared test by test (`scripts/wsl/inside.sh parity` runs it on both sides).

  parity.py summarize <wall-seconds> <exit>   nextest's libtest JSON on stdin; prints a header line
                                              `# wall <s> exit <n> tests <count>`, then `<test>\t<outcome>\t<seconds>`
                                              for each test, sorted. The outcome is the test's last event: ok, failed,
                                              ignored or timeout.
  parity.py compare <one> <other>             two summaries; prints each side's wall time, exit and count, and every
                                              test whose outcome differs or that one side lacks. Exits 0 only if both
                                              ran the same tests with the same outcome each, at least one, and the
                                              same exit. `-` for <one> reads stdin, the stream already open: as
                                              `wsl.exe -u ci` runs it, reopening /dev/stdin is refused (EACCES, run
                                              43418528499).

Every input may start with a UTF-8 byte-order mark, which Windows PowerShell's stdin writer can put in front of what
`wsl.ps1` sends; it is dropped.

Wall times are reported, never compared: the PC and the hosted runner are different machines.
"""

import io
import json
import sys


def summarize(wall, code, lines):
    outcome, seconds = {}, {}
    for line in lines:
        try:
            event = json.loads(line)
        except ValueError:
            continue
        if not isinstance(event, dict) or event.get("type") != "test" or event.get("event") == "started":
            continue
        name = event.get("name")
        if not name:
            continue
        outcome[name] = event.get("event")
        seconds[name] = event.get("exec_time", 0)
    out = [f"# wall {wall} exit {code} tests {len(outcome)}"]
    out += [f"{name}\t{outcome[name]}\t{seconds[name]}" for name in sorted(outcome)]
    return "\n".join(out) + "\n"


def stdin():
    """The open stdin as text, a leading byte-order mark dropped."""
    return io.TextIOWrapper(sys.stdin.buffer, encoding="utf-8-sig")


def read(path):
    if path == "-":
        return parse(stdin())
    with open(path, encoding="utf-8-sig") as f:
        return parse(f)


def parse(f):
    header, tests = {}, {}
    for line in f:
        line = line.rstrip("\r\n")
        if line.startswith("# "):
            words = line[2:].split()
            header = dict(zip(words[::2], words[1::2]))
        elif line:
            name, result, _ = line.split("\t")
            tests[name] = result
    return header, tests


def compare(one, other):
    (h1, t1), (h2, t2) = read(one), read(other)
    one = "stdin" if one == "-" else one
    lines = [
        f"parity: {one}: wall {h1.get('wall', '?')} s, exit {h1.get('exit', '?')}, {len(t1)} tests",
        f"parity: {other}: wall {h2.get('wall', '?')} s, exit {h2.get('exit', '?')}, {len(t2)} tests",
    ]
    differ = sorted(n for n in set(t1) | set(t2) if t1.get(n) != t2.get(n))
    for name in differ:
        lines.append(f"parity: differs: {name}: {t1.get(name, 'absent')} against {t2.get(name, 'absent')}")
    same = not differ and t1 and h1.get("exit") == h2.get("exit")
    if not t1:
        lines.append("parity: no test ran")
    if h1.get("exit") != h2.get("exit"):
        lines.append("parity: the exits differ")
    lines.append(f"parity: {'identical, test by test' if same else 'NOT identical'}")
    return "\n".join(lines) + "\n", 0 if same else 1


def main(argv):
    if len(argv) == 4 and argv[1] == "summarize":
        sys.stdout.write(summarize(argv[2], argv[3], stdin()))
        return 0
    if len(argv) == 4 and argv[1] == "compare":
        text, code = compare(argv[2], argv[3])
        sys.stdout.write(text)
        return code
    sys.stderr.write(__doc__)
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv))
