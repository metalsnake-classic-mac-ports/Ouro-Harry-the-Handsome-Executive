"""Helpers for recording sprite kinds in Unicorn: faces named by their
HrSp id, task TVectors named by their routine, and a dump of every
sprite field the port models.

    faces = Faces(m)          # stubs the face loaders; then run Init...Sprite
    dump(m, sp, faces)        # "name=value ..." of the sprite
"""
import os
import struct


class Faces:
    """Stub Load1EncFaceFromCicn and LoadEncFaceSetFromPICT: each face gets
    a 0x1c-byte record (the game writes its draw proc at +0x18) that maps
    back to the HrSp id it was loaded from."""

    def __init__(self, m, work=None):
        """work: also give each face its HrSp header (bounds, rect2) and,
        where the game builds one, its mask region (+0x14) from
        diff/facemasks.txt."""
        self.m = m
        self.ids = {}
        masks = {}
        if work:
            lines = open(f"{work}/diff/facemasks.txt").read().splitlines()
            i = 1
            while i < len(lines):
                _, fid, w, h = lines[i].split()
                fid, w, h = int(fid), int(w), int(h)
                masks[fid] = {(x, y) for y in range(h) for x in range(w) if lines[i + 1 + y][x] == "#"}
                i += 1 + h

        def fill(rec, hrsp, region):
            if not work:
                return
            path = f"{work}/res/Harry_Graphics/HrSp_{hrsp}.bin"
            if os.path.exists(path):
                m.write(rec, open(path, "rb").read()[:16])
            if region and hrsp in masks:
                m.w32(rec + 0x14, m.make_region(masks[hrsp]))

        def one(mm):
            rec = mm.alloc(0x1C)
            self.ids[rec] = mm.get(5) & 0xFFFF
            fill(rec, self.ids[rec], mm.get(4) & 0xFF)
            return rec

        def face_set(mm):
            # (pict, count, w, h, per row, first / last frame with a
            # mask region, first HrSp id)
            count, first = mm.get(4) & 0xFFFF, mm.get(10) & 0xFFFF
            lo, hi = mm.get(8) & 0xFFFF, mm.get(9) & 0xFFFF
            recs = mm.alloc(0x1C * count)
            for i in range(count):
                self.ids[recs + 0x1C * i] = first + i
                fill(recs + 0x1C * i, first + i, (lo or hi) and lo <= i <= hi)
            return recs
        m.stub_routine("Load1EncFaceFromCicn", one)
        m.stub_routine("LoadEncFaceSetFromPICT", face_set)

    def name(self, p):
        if p == 0:
            return 0
        return self.ids.get(p, -1)


def tvectors(ppc_dir):
    names = {0: "-"}
    for line in open(os.path.join(ppc_dir, "toc_functions.txt")):
        _, tv, name = line.split()
        names[int(tv, 16)] = name
    return names


# (name, offset, size): the fields of harry_core::sprite::Sprite
FIELDS = [("kind", 0x00, 2), ("posV", 0x02, 2), ("posH", 0x04, 2), ("hotT", 0x06, 2), ("hotL", 0x08, 2),
          ("hotB", 0x0A, 2), ("hotR", 0x0C, 2), ("layer", 0x4C, 2), ("speedV", 0x4E, 2), ("speedH", 0x50, 2),
          ("scale0", 0x52, 2), ("scale1", 0x54, 2), ("scale2", 0x56, 2), ("scale3", 0x58, 2), ("atV", 0x5A, 2),
          ("atH", 0x5C, 2), ("fixH", 0x5E, 4), ("fixV", 0x62, 4), ("dir", 0x66, 2), ("levelIndex", 0x68, 2),
          ("f6e", 0x6E, 4), ("state", 0x72, 2), ("timer", 0x74, 2), ("idleIndex", 0x76, 2), ("f78", 0x78, 4),
          ("f7c", 0x7C, 2), ("f82", 0x82, 2), ("f84", 0x84, 2), ("f86", 0x86, 4), ("b8a", 0x8A, 1),
          ("b8b", 0x8B, 1), ("b8c", 0x8C, 1), ("air", 0x8E, 2), ("flip", 0x90, 2), ("frozen", 0x92, 1),
          ("f94", 0x94, 2), ("blast", 0x96, 2), ("lastTileH", 0x98, 2), ("lastTileV", 0x9A, 2), ("slide", 0x9C, 2)]


def dump(m, sp, faces, tv):
    out = []
    for name, off, size in FIELDS:
        b = m.read(sp + off, size)
        v = b[0] if size == 1 else struct.unpack(">h" if size == 2 else ">i", b)[0]
        if name == "f86":
            v = faces.name(v & 0xFFFFFFFF)          # the last frame's face
        out.append(f"{name}={v}")
    out.append("face=%d" % faces.name(m.r32(sp + 0x7E)))
    for name, off in (("task", 0x1A), ("hit", 0x1E), ("tile", 0x9E)):
        out.append(f"{name}={tv.get(m.r32(sp + off), hex(m.r32(sp + off)))}")
    return " ".join(out)
