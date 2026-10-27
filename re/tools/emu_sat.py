"""Record SAT's sprite list handling in HandleSprites (SAT's run: tasks,
the sort pass by layer, the collision pass; then CheckPlayerCollision)
with dummy sprites whose tasks and hit tasks only log their calls - and
sometimes kill their sprite (task = 0) or move it.

Each line: "<n> <sprites> -> <tasks> <order> <hits> <playerhits> <kinds>"
  sprites   per sprite (list order) layer:posV:posH:atV:atH:t:l:b:r:task:hit:act
            joined by ","; task 0 none, 1 dummy, 2 the player's;
            act 0 nothing, 1 kills itself, 2 moves 40 px down
  tasks     indices of the sprites whose task ran, in order
  order     the list after the run
  hits      hit task calls a>b (a's hit task with b)
  playerhits  HitPlayerSprite calls (index of the other sprite)
  kinds     each sprite's kind afterwards (kind = 100 + index at the start)
Usage: python3 -I emu_sat.py [re/work]
"""
import os
import random
import struct
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ppcmachine import Machine  # noqa: E402

SAT, TV_PLAYER, PLAYER_OFF = 0x10065B70, 0x10066FC4, 0x10065ACC
CASES = 20000


def main():
    work = sys.argv[1] if len(sys.argv) > 1 else "re/work"
    m = Machine(f"{work}/ppc")
    sat = m.glob(SAT)
    m.call(0x1005FCE8, 0, 1, 1, 0x40)               # SATConfigure as InitSAT does
    m.stub_routine(0x1005BB88, 0)                  # SAT's sound queue
    m.write(m.glob(PLAYER_OFF), b"\0")
    # dummy task / hit task code and their TVectors
    toc = m.get(2)
    task_code, hit_code = m.alloc(16), m.alloc(16)
    tv_task, tv_hit = m.alloc(8), m.alloc(8)
    m.w32(tv_task, task_code)
    m.w32(tv_task + 4, 0x1006D390)
    m.w32(tv_hit, hit_code)
    m.w32(tv_hit + 4, 0x1006D390)
    state = {"tasks": [], "hits": [], "phits": [], "index": {}, "act": {}}

    def task(mm):
        s = mm.get(3)
        i = state["index"][s]
        state["tasks"].append(i)
        if state["act"][i] == 1:
            mm.w32(s + 0x1A, 0)
        elif state["act"][i] == 2:
            mm.w16(s + 2, mm.r16(s + 2) + 40)
        return 0

    def hit(mm):
        state["hits"].append("%d>%d" % (state["index"][mm.get(3)], state["index"][mm.get(4)]))
        return 0
    m.stub_routine(task_code, task)
    m.stub_routine(hit_code, hit)
    m.stub_routine("HandlePlayerSprite", task)
    m.stub_routine("HitPlayerSprite", lambda mm: state["phits"].append(str(state["index"][mm.get(4)])) or 0)
    rnd = random.Random("sat")
    mark = m.heap
    out = open(f"{work}/diff/sat.txt", "w")
    out.write("# HandleSprites: n sprites -> tasks order hits playerhits kinds\n")
    for case in range(CASES):
        m.heap = mark
        n = rnd.choice([2, 2, 3, 4, 5, 8, rnd.randint(2, 16)])
        player = rnd.randrange(n) if rnd.random() < 0.7 else -1
        sprites, descr = [], []
        for i in range(n):
            s = m.alloc(0xA2)
            layer = rnd.choice([0, 5, 10, 10, 11, 11, 20, 70, 80, 100, rnd.randint(-20, 200)])
            pv, ph = rnd.randint(0, 150), rnd.randint(0, 150)
            av, ah = (pv, ph) if rnd.random() < 0.7 else (pv + rnd.randint(-20, 20), ph + rnd.randint(-20, 20))
            t, l = rnd.randint(-10, 30), rnd.randint(-10, 30)
            b, r = t + rnd.choice([0, rnd.randint(1, 60)]), l + rnd.choice([0, rnd.randint(1, 60)])
            tk = 2 if i == player else (1 if rnd.random() < 0.9 else 0)
            ht = 1 if rnd.random() < 0.7 else 0
            act = rnd.choice([0, 0, 0, 1, 2]) if tk == 1 else 0
            m.w16(s, 100 + i)
            for off, v in ((2, pv), (4, ph), (6, t), (8, l), (10, b), (12, r), (0x4C, layer), (0x5A, av), (0x5C, ah)):
                m.w16(s + off, v)
            m.w32(s + 0x1A, {0: 0, 1: tv_task, 2: TV_PLAYER}[tk])
            m.w32(s + 0x1E, tv_hit if ht else 0)
            sprites.append(s)
            state["index"][s] = i
            state["act"][i] = act
            descr.append(":".join(map(str, (layer, pv, ph, av, ah, t, l, b, r, tk, ht, act))))
        for i, s in enumerate(sprites):
            m.w32(s + 0x2E, sprites[i + 1] if i + 1 < n else 0)
            m.w32(s + 0x32, sprites[i - 1] if i else 0)
        m.w32(sat + 0x78, sprites[0])
        for k in ("tasks", "hits", "phits"):
            state[k].clear()
        m.call("HandleSprites")
        order, p = [], m.r32(sat + 0x78)
        while p:
            order.append(state["index"][p])
            p = m.r32(p + 0x2E)
        kinds = [m.r16(s) for s in sprites]
        j = lambda xs: ",".join(map(str, xs)) or "-"
        out.write(f"{n} {','.join(descr)} -> {j(state['tasks'])} {j(order)} {j(state['hits'])} "
                  f"{j(state['phits'])} {j(kinds)}\n")
    out.close()
    print(f"sat: {CASES} cases")


if __name__ == "__main__":
    main()
