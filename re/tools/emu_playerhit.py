"""Record HitPlayerSprite - Harry touching another sprite - for the kinds
the port has (bonus, item, door, collider, dart enemy, ramp, stairs, sliding door, co-worker, mail guy, robot, enemy shot). The other sprite is made by its real
setup; doors get a level entry (index, info #0) on level 1.

Each line: "<name=value ...> -> <name=value ...>"
  inv     the 12 slots as id:count joined by ","
  events  p:<snd> (STPlayRegSound), v:<snd> (kept as the voice), r:<snd>
          (the remark: the old one stopped, this one kept), halt
          (HaltTalkingSounds), joined by ","
For a locked door the original reads two never-set locals (the hinge);
the recorder puts the closed door's values there (as the port does).
Usage: python3 -I emu_playerhit.py [re/work]
"""
import os
import random
import struct
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ppcmachine import Machine, STACK_TOP  # noqa: E402
from sprites import Faces  # noqa: E402

COMFORT, CUR_SLOT, INVENTORY = 0x1006856A, 0x1006857A, 0x10065AC4
SLOTS = {"t24": 0x10065E24, "t28": 0x10065E28, "staple": 0x10065AEC, "badass": 0x10065AE8, "shrapnel": 0x10065AE4,
         "contact": 0x10065E1C, "playerh": 0x100659B4, "playerv": 0x100659B0, "ouchcd": 0x10065E44, "cdelta": 0x10065E6C,
         "t40": 0x10065E40}
KICK_RESET, PHYSICS, BOUNCED, SCORE = 0x10065E20, 0x100659D8, 0x10065E50, 0x10065AF8
VOICE, VOICE2, REMARK = 0x10068AA0, 0x10068AA4, 0x10068A98
SOUNDS = {0x10065CC0: 212, 0x10065CC4: 211, 0x10065CC8: 210, 0x10065CB0: 216, 0x10065CB8: 214,
          0x10065C28: 218, 0x10065C24: 219, 0x10065C60: 240, 0x10065CE0: 204, 0x10065CDC: 205, 0x10065CCC: 209, 0x10065C48: 251, 0x10065C98: 223}
ITEMS = [11199, 11299, 11397, 11398, 11399, 11499, 11599, 11699, 11799, 11899, 11999]
DOORS = [901, 902, 903, 911, 912, 913, 921, 922, 923, 931, 932, 933]
HINGE = {0: (30, 35), 1: (31, 17), 2: (34, 19), 3: (5, 18)}
KINDS = [("SetupBonusSprite", [700, 701, 702, 703, 704, 711, 712, 713, 714]), ("SetupItemSprite", ITEMS),
         ("SetupDoorSprite", DOORS), ("SetupDoorSprite", DOORS),
         ("SetupColliderSprite", [9600, 9600, 9800, 9800, 9199, 9200, 2301, 3100, 3101, 9300, 9411]),
         ("SetupDartEnemySprite", [1011, 1021, 1031, 1041]), ("SetupRampSprite", [9701, 9702, 9703, 9704]),
         ("SetupRampSprite", [9701, 9702, 9703, 9704]),
         ("SetupStairsSprite", [9901, 9902, 9903, 9904]), ("SetupSlidingDoorSprite", [951, 961]),
         ("SetupCoWorkerSprite", [1301, 1321, 1441]),
         ("SetupMailGuySprite", [1601, 1611, 1621, 1631]),
         ("SetupRobotSprite", [1801, 1811, 1821, 2001]),
         ("SetupEnemyShotSprite", [1018, 1038, 1051, 9610, 9450, 1800, 1950, 1150, 1299, 2281, 2291, 2500])]
FRAME = STACK_TOP - 0x1000 - 0x140              # HitPlayerSprite's r1
CASES = 30000


def main():
    work = sys.argv[1] if len(sys.argv) > 1 else "re/work"
    m = Machine(f"{work}/ppc")
    Faces(m, work)
    m.stub_routine(0x1005F340, lambda mm: mm.alloc(0x40))      # mask loader (the ramps' masks)
    for init in ("InitBonusSprite", "InitItemSprite", "InitDoorSprite", "InitColliderSprite", "InitDartEnemySprite", "InitRampSprite", "InitStairsSprite", "InitSlidingDoorSprite", "InitCoWorkerSprite", "InitMailGuySprite", "InitRobotSprite", "InitEnemyShotSprite", "InitPlayerKickRects"):
        m.call(init)
    lvl = m.load_level(open(f"{work}/res/Episode_1/Hlvl_1.bin", "rb").read())
    ev = []
    channel = [0]

    def play(mm):
        channel[0] += 1
        ev.append(["p", mm.get(3), 0x1000 + channel[0]])
        return 0x1000 + channel[0]
    m.stub_routine("STPlayRegSound", play)
    busy = {}
    m.stub_routine(0x10056DE8, lambda mm: busy.get(mm.get(3), 0))
    m.stub_routine(0x10056C0C, lambda mm: ev.append(["stop", mm.get(3)]) or 0)
    m.stub_routine("HaltTalkingSounds", lambda mm: ev.append(["halt"]) or 0)
    m.stub_routine("Ouch", lambda mm: ev.append(["ouch"]) or 0)
    for f in ("UpdateAmmo", "CopyStatRect", "PlotEncFace", "DrawNumItems", "FindInvItemFace", "UpdateInventory",
              0x1005F738, 0x1005F7EC):
        m.stub_routine(f, 0)
    for n in ("ForeColor", "PaintRect"):
        m.stub(n, m.tb_nop)
    for slot, snd in SOUNDS.items():
        m.w32(m.glob(slot), snd)
    g = {n: m.glob(a) for n, a in SLOTS.items()}
    # Harry's normal hot rect, as SetupPlayerSprite sets it
    m.write(m.glob(0x10065E94), struct.pack(">hhhh", 22, 22, 42, 42))
    kick_reset, physics, bounced, score = m.glob(KICK_RESET), m.glob(PHYSICS), m.glob(BOUNCED), m.glob(SCORE)
    inv = m.r32(INVENTORY)
    player, other = m.alloc(0xA2), m.alloc(0xA2)
    rnd = random.Random("playerhit")
    out = open(f"{work}/diff/playerhit.txt", "w")
    out.write("# HitPlayerSprite: name=value inputs -> name=value outputs\n")
    for case in range(CASES):
        setup, ids = rnd.choice(KINDS)
        kind = rnd.choice(ids)
        st = {"kind": kind, "air": rnd.choice([0, 0, 0, 0, 5]), "pfrozen": int(rnd.random() < 0.03),
              "ofrozen": int(rnd.random() < 0.03), "comfort": rnd.choice([rnd.randint(0, 150), 143, 144, 140]),
              "t24": rnd.choice([0, 0, rnd.randint(1, 200)]), "t28": rnd.choice([0, 0, rnd.randint(1, 25)]), "t40": rnd.choice([0, 0, 5]),
              "staple": rnd.choice([0, 50, 99, 100, rnd.randint(0, 120)]),
              "badass": rnd.choice([0, 50, 99, 100, rnd.randint(0, 120)]),
              "shrapnel": rnd.choice([0, 50, 99, 100, rnd.randint(0, 120)]),
              "voicebusy": rnd.choice([0, 0, 1]), "contact": rnd.choice([0, 64, 256]),
              "kickreset": rnd.choice([0, 1]), "bounce": rnd.choice([150, 100, 200])}
        # the other sprite and Harry around it
        ov, oh = rnd.randint(500, 3500), rnd.randint(500, 3500)
        st.update({"ov": ov, "oh": oh, "levelindex": rnd.randint(0, 255), "info0": rnd.choice([0, 0, 0, 100, 200])})
        pv, ph = ov + rnd.randint(-64, 40), oh + rnd.randint(-64, 40)
        st.update({"pv": pv, "ph": ph, "fixh": (ph + rnd.choice([0, 0, rnd.randint(-8, 8)])) * 256 + rnd.randint(0, 255),
                   "fixv": (pv + rnd.choice([0, 0, rnd.randint(-8, 8)])) * 256 + rnd.randint(0, 255),
                   "speedh": rnd.randint(-2500, 2500), "speedv": rnd.randint(-2500, 2500), "dir": rnd.randint(0, 35),
                   "kicking": int(rnd.random() < 0.5),
                   "pav": pv + rnd.choice([0, 0, rnd.randint(-4, 4)]), "pah": ph + rnd.choice([0, 0, rnd.randint(-4, 4)]),
                   "otimer": rnd.choice([0, 0, 0, 1, 3, -5]), "bounced": int(rnd.random() < 0.15),
                   "score": rnd.choice([0, 12345]), "pf94": rnd.choice([0, 0, 0, 2]), "setospeed": "-",
                   "ouchcd": rnd.choice([0, 0, 0, 2]), "cdelta": rnd.choice([0, 0, -5]),
                   "favor": rnd.randint(0, 144)})
        if setup == "SetupEnemyShotSprite" and rnd.random() < 0.7:
            st["setospeed"] = f"{rnd.randint(-2000, 2000)}:{rnd.randint(-2000, 2000)}"
        st["omdir"] = 0
        if setup == "SetupMailGuySprite":
            st["setospeed"] = rnd.choice(["15:0", "-15:0", "0:15", "0:-15", "0:0"])
            st["omdir"] = rnd.choice([0, 0, 5])
        if setup in ("SetupRampSprite", "SetupStairsSprite"):
            # around the ramp, at any speed
            st["speedv"] = rnd.choice([st["speedv"], rnd.randint(-600, 600)])
            st["speedh"] = rnd.choice([st["speedh"], rnd.randint(-600, 600)])
        if kind == 9600 and rnd.random() < 0.4:
            # beside the vending machine, its middle level with Harry's
            st["pav"] = ov - 15 + rnd.randint(-1, 1)
            st["pah"] = oh + rnd.choice([-40, 40]) + rnd.randint(-3, 3)
            st["kicking"] = 1
        st["playerh"], st["playerv"] = ph + rnd.choice([0, 0, rnd.randint(-5, 5)]), pv + rnd.choice([0, 0, rnd.randint(-5, 5)])
        key = 0x2C87 - st["info0"]
        n_items = rnd.choice([0, 1, 3, 11, 12, rnd.randint(0, 12)])
        pool = ITEMS + [key, key]
        slots = [(rnd.choice(pool), rnd.randint(1, 3)) for _ in range(n_items)] + [(-1, 0)] * (12 - n_items)
        st["curslot"] = -1 if n_items == 0 else rnd.randint(0, n_items - 1)
        st["inv"] = ",".join("%d:%d" % s for s in slots)
        # write it
        m.write(player, bytes(0xA2))
        m.w16(player, 0x45)                    # SetupPlayerSprite's kind
        m.w32(player + 0x1A, 0x10066FC4)
        for off, n in ((0x02, "pv"), (0x04, "ph"), (0x4E, "speedv"), (0x50, "speedh"), (0x66, "dir"), (0x8E, "air")):
            m.w16(player + off, st[n])
        m.w16(player + 0x5A, st["pav"])
        m.w16(player + 0x5C, st["pah"])
        m.w32(player + 0x5E, st["fixh"])
        m.w32(player + 0x62, st["fixv"])
        m.write(player + 0x06, struct.pack(">hhhh", *((0, 0, 64, 64) if st["kicking"] else (22, 22, 42, 42))))
        m.write(player + 0x92, bytes([st["pfrozen"]]))
        m.w16(player + 0x94, st["pf94"])
        m.write(other, bytes(0xA2))
        m.w16(other, kind)
        m.w16(other + 2, ov)
        m.w16(other + 4, oh)
        m.w32(m.glob(0x100659B4), 0)                    # gPlayerH / V for the robots' setup
        m.call(setup, other)
        m.w16(other + 0x68, st["levelindex"])
        m.w16(other + 0x74, st["otimer"])
        m.w16(other + 0x66, st["omdir"])
        if st["setospeed"] != "-":
            sv, sh = map(int, st["setospeed"].split(":"))
            m.w16(other + 0x4E, sv)
            m.w16(other + 0x50, sh)
        m.w16(lvl + 0x10002 + st["levelindex"] * 16, kind)
        m.w16(lvl + 0x10004 + st["levelindex"] * 16, st["info0"])
        m.write(other + 0x92, bytes([st["ofrozen"]]))
        m.w16(COMFORT, st["comfort"])
        m.w16(0x1006856E, st["favor"])
        for n in SLOTS:
            m.w16(g[n], st[n])
        m.write(kick_reset, bytes([st["kickreset"]]))
        m.write(bounced, bytes([st["bounced"]]))
        m.w32(score, st["score"])
        m.w16(physics + 8, st["bounce"])
        for i, (oid, cnt) in enumerate(slots):
            m.w16(inv + 4 * i, oid)
            m.w16(inv + 4 * i + 2, cnt)
        m.w16(CUR_SLOT, st["curslot"])
        if kind in DOORS:
            dh, dv = HINGE[(kind - 901) // 10]
            m.w16(FRAME + 0x86, ov + dv)
            m.w16(FRAME + 0x88, oh + dh)
        m.w32(VOICE, 0xD0)
        m.w32(VOICE2, 0xD1)
        m.w32(REMARK, 0xE0)
        busy.clear()
        busy.update({0xD0: st["voicebusy"], 0xD1: st["voicebusy"]})
        ev.clear()
        channel[0] = 0
        m.call("HitPlayerSprite", player, other)
        voice, remark = m.r32(VOICE), m.r32(REMARK)
        events = []
        for e in ev:
            if e[0] in ("halt", "ouch"):
                events.append(e[0])
            elif e[0] == "stop":
                assert e[1] == 0xE0, e
            else:
                events.append(("v:%d" if e[2] == voice else "r:%d" if e[2] == remark else "p:%d") % e[1])
        hot = struct.unpack(">hhhh", m.read(player + 6, 8))
        outs = {"ofrozen": m.read(other + 0x92, 1)[0], "comfort": m.r16(COMFORT),
                **{n: m.r16(g[n]) for n in ("t24", "t28", "t40", "staple", "badass", "shrapnel", "contact")},
                "kickreset": m.read(kick_reset, 1)[0], "curslot": m.r16(CUR_SLOT),
                "inv": ",".join("%d:%d" % (m.r16(inv + 4 * i), m.r16(inv + 4 * i + 2)) for i in range(12)),
                "pv": m.r16(player + 2), "ph": m.r16(player + 4),
                "fixh": struct.unpack(">i", m.read(player + 0x5E, 4))[0],
                "fixv": struct.unpack(">i", m.read(player + 0x62, 4))[0],
                "pav": m.r16(player + 0x5A), "pah": m.r16(player + 0x5C),
                "bounced": m.read(bounced, 1)[0], "score": struct.unpack(">i", m.read(score, 4))[0],
                "ouchcd": m.r16(g["ouchcd"]), "cdelta": m.r16(g["cdelta"]), "favor": m.r16(0x1006856E),
                "otimer": m.r16(other + 0x74), "pf94": m.r16(player + 0x94), "pair": m.r16(player + 0x8E), "pflip": m.r16(player + 0x90),
                "ospeedv": m.r16(other + 0x4E), "ospeedh": m.r16(other + 0x50), "opv": m.r16(other + 2), "oph": m.r16(other + 4),
                "speedv": m.r16(player + 0x4E), "speedh": m.r16(player + 0x50), "hot": "%d:%d:%d:%d" % hot,
                "okind": m.r16(other), "odir": m.r16(other + 0x66), "ob8a": m.read(other + 0x8A, 1)[0],
                "record": m.r16(lvl + 0x10002 + st["levelindex"] * 16), "events": ",".join(events) or "-"}
        fmt = lambda d: " ".join(f"{k}={v}" for k, v in d.items())
        out.write(f"{fmt(st)} -> {fmt(outs)}\n")
    out.close()
    print(f"playerhit: {CASES} cases")


if __name__ == "__main__":
    main()
