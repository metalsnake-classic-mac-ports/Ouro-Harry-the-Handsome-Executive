"""Record HitPlayerSprite - Harry touching another sprite - for the kinds
the port has (bonus, item). The other sprite is made by its real setup.

Each line: "<kind> <air> <pfrozen> <ofrozen> <comfort> <t24> <staple>
  <badass> <shrapnel> <curslot> <inv> <voicebusy> -> <ofrozen> <comfort>
  <t24> <staple> <badass> <shrapnel> <curslot> <inv> <events>"
  inv     the 12 slots as id:count joined by ","
  events  p:<snd> (STPlayRegSound), v:<snd> (kept as the voice), halt
          (HaltTalkingSounds), joined by ","
Usage: python3 -I emu_playerhit.py [re/work]
"""
import os
import random
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ppcmachine import Machine  # noqa: E402
from sprites import Faces  # noqa: E402

COMFORT, CUR_SLOT, INVENTORY = 0x1006856A, 0x1006857A, 0x10065AC4
T24, STAPLE, BADASS, SHRAPNEL = 0x10065E24, 0x10065AEC, 0x10065AE8, 0x10065AE4
VOICE, VOICE2 = 0x10068AA0, 0x10068AA4
SOUNDS = {0x10065CC0: 212, 0x10065CC4: 211, 0x10065CC8: 210, 0x10065CB0: 216, 0x10065CB8: 214,
          0x10065C28: 218, 0x10065C24: 219}
ITEMS = [11199, 11299, 11397, 11398, 11399, 11499, 11599, 11699, 11799, 11899, 11999]
KINDS = [("SetupBonusSprite", [700, 701, 702, 703, 704, 711, 712, 713, 714]), ("SetupItemSprite", ITEMS)]
CASES = 12000


def main():
    work = sys.argv[1] if len(sys.argv) > 1 else "re/work"
    m = Machine(f"{work}/ppc")
    Faces(m)
    for init in ("InitBonusSprite", "InitItemSprite"):
        m.call(init)
    ev = []
    channel = [0]

    def play(mm):
        channel[0] += 1
        ev.append(["p", mm.get(3), 0x1000 + channel[0]])
        return 0x1000 + channel[0]
    m.stub_routine("STPlayRegSound", play)
    busy = {}
    m.stub_routine(0x10056DE8, lambda mm: busy.get(mm.get(3), 0))
    m.stub_routine("HaltTalkingSounds", lambda mm: ev.append(["halt"]) or 0)
    for f in ("UpdateAmmo", "CopyStatRect", "PlotEncFace", "DrawNumItems", "FindInvItemFace", "UpdateInventory",
              0x1005F738, 0x1005F7EC):
        m.stub_routine(f, 0)
    for n in ("ForeColor", "PaintRect"):
        m.stub(n, m.tb_nop)
    for slot, snd in SOUNDS.items():
        m.w32(m.glob(slot), snd)
    g = {n: m.glob(a) for n, a in (("t24", T24), ("staple", STAPLE), ("badass", BADASS), ("shrapnel", SHRAPNEL))}
    inv = m.r32(INVENTORY)
    player, other = m.alloc(0xA2), m.alloc(0xA2)
    rnd = random.Random("playerhit")
    out = open(f"{work}/diff/playerhit.txt", "w")
    out.write("# HitPlayerSprite: kind air pfrozen ofrozen comfort t24 staple badass shrapnel curslot inv voicebusy "
              "-> ofrozen comfort t24 staple badass shrapnel curslot inv events\n")
    for case in range(CASES):
        setup, ids = rnd.choice(KINDS)
        kind = rnd.choice(ids)
        air = rnd.choice([0, 0, 0, 0, 5])
        pfrozen, ofrozen = int(rnd.random() < 0.05), int(rnd.random() < 0.05)
        comfort = rnd.choice([rnd.randint(0, 150), 143, 144, 140])
        t24 = rnd.choice([0, 0, rnd.randint(1, 200)])
        ammo = [rnd.choice([0, 50, 99, 100, rnd.randint(0, 120)]) for _ in range(3)]
        n_items = rnd.choice([0, 1, 3, 11, 12, rnd.randint(0, 12)])
        slots = [(rnd.choice(ITEMS), rnd.randint(1, 3)) for _ in range(n_items)] + [(-1, 0)] * (12 - n_items)
        cur = -1 if n_items == 0 else rnd.randint(0, n_items - 1)
        voice_busy = rnd.choice([0, 0, 1])
        m.write(player, bytes(0xA2))
        m.w32(player + 0x1A, 0x10066FC4)
        m.w16(player + 0x8E, air)
        m.write(player + 0x92, bytes([pfrozen]))
        m.write(other, bytes(0xA2))
        m.w16(other, kind)
        m.call(setup, other)
        m.write(other + 0x92, bytes([ofrozen]))
        m.w16(COMFORT, comfort)
        m.w16(g["t24"], t24)
        for n, v in zip(("staple", "badass", "shrapnel"), ammo):
            m.w16(g[n], v)
        for i, (oid, cnt) in enumerate(slots):
            m.w16(inv + 4 * i, oid)
            m.w16(inv + 4 * i + 2, cnt)
        m.w16(CUR_SLOT, cur)
        m.w32(VOICE, 0xD0)
        m.w32(VOICE2, 0xD1)
        busy.clear()
        busy.update({0xD0: voice_busy, 0xD1: voice_busy})
        ev.clear()
        channel[0] = 0
        m.call("HitPlayerSprite", player, other)
        voice = m.r32(VOICE)
        events = []
        for e in ev:
            if e[0] == "halt":
                events.append("halt")
            else:
                events.append(("v:%d" if e[2] == voice else "p:%d") % e[1])
        inv_s = lambda: ",".join("%d:%d" % (m.r16(inv + 4 * i), m.r16(inv + 4 * i + 2)) for i in range(12))
        ins = [kind, air, pfrozen, ofrozen, comfort, t24, *ammo, cur, ",".join("%d:%d" % s for s in slots), voice_busy]
        outs = [m.read(other + 0x92, 1)[0], m.r16(COMFORT), m.r16(g["t24"]), m.r16(g["staple"]), m.r16(g["badass"]),
                m.r16(g["shrapnel"]), m.r16(CUR_SLOT), inv_s(), ",".join(events) or "-"]
        out.write(f"{' '.join(map(str, ins))} -> {' '.join(map(str, outs))}\n")
    out.close()
    print(f"playerhit: {CASES} cases")


if __name__ == "__main__":
    main()
