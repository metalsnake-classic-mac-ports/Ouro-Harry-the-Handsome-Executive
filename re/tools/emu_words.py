"""Record GenerateBuzzwords and GeneratePhonewords (with PickAString and
QuickDraw's Random): the word lists STR# 6900-6905 and 6800-6801 from the
application, the last picks as inputs; a sequence of calls per case.

Each line: "seed=.. last=<8 numbers> calls=<b|p...> -> <hex text per call ','> seed=.. last=.."
  last: template, fillers 1-5 of the buzzwords, template, filler of the phone words
Usage: python3 -I emu_words.py [re/work]
"""
import os
import random
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ppcmachine import Machine  # noqa: E402

BUZZ_LISTS, PHONE_LISTS = 0x10066514, 0x10066510
LAST = [0x10066E7A, 0x10066E78, 0x10066E76, 0x10066E74, 0x10066E72, 0x10066E70, 0x10066E6E, 0x10066E6C]
CASES = 5000


def main():
    work = sys.argv[1] if len(sys.argv) > 1 else "re/work"
    m = Machine(f"{work}/ppc")
    lists = {n: open(f"{work}/app/rsrc/STR__{n}.bin", "rb").read() for n in list(range(6900, 6906)) + [6800, 6801]}
    counts = [int.from_bytes(lists[n][:2], "big") for n in list(range(6900, 6906)) + [6800, 6801]]
    buzz, phone = m.glob(BUZZ_LISTS), m.glob(PHONE_LISTS)
    for i in range(6):
        m.w32(buzz + 4 * i, m.new_handle(lists[6900 + i]))
    for i in range(2):
        m.w32(phone + 4 * i, m.new_handle(lists[6800 + i]))
    buf = m.alloc(512)
    mark = m.heap
    rnd = random.Random("words")
    out = open(f"{work}/diff/words.txt", "w")
    out.write("# GenerateBuzzwords / GeneratePhonewords\n")
    for _ in range(CASES):
        m.heap = mark
        seed = rnd.randint(1, 0x7FFFFFFE)
        last = [rnd.choice([0, rnd.randint(0, c - 1)]) for c in counts]
        calls = "".join(rnd.choice("bbbp") for _ in range(rnd.randint(1, 6)))
        m.rand_seed = seed
        for a, v in zip(LAST, last):
            m.w16(a, v)
        texts = []
        for c in calls:
            m.call("GenerateBuzzwords" if c == "b" else "GeneratePhonewords", buf)
            n = m.read(buf, 1)[0]
            texts.append(m.read(buf + 1, n).hex() or "-")
        out.write(f"seed={seed} last={':'.join(map(str, last))} calls={calls} -> {','.join(texts)} seed={m.rand_seed} "
                  f"last={':'.join(str(m.r16(a)) for a in LAST)}\n")
    out.close()
    print(f"words: {CASES} cases")


if __name__ == "__main__":
    main()
