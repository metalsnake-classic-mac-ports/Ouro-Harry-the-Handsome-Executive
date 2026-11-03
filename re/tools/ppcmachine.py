"""Run routines of the PowerPC build of Harry in Unicorn.

The machine is the classic-mac-port skill's (scripts/ppc/ppcmachine.py,
found through CLASSIC_MAC_PORT, default ~/.claude/skills/classic-mac-port);
this module adds what is Harry's: the addresses the recorders use and
loading a level. re/ghidra/run.sh must have run (re/work/ppc).

Usage:
    m = Machine("re/work/ppc")
    m.stub("SetRect", m.toolbox_setrect)    # most common ones are built in
    m.call("InitTileHotRects")
    m.read(addr, n)
"""
import importlib.util
import os

# the skill's module has the same name as this one: load it under another
_skill = os.environ.get("CLASSIC_MAC_PORT", os.path.expanduser("~/.claude/skills/classic-mac-port"))
_spec = importlib.util.spec_from_file_location("skill_ppcmachine", os.path.join(_skill, "scripts", "ppc", "ppcmachine.py"))
_skill_machine = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(_skill_machine)
STACK_TOP, BASE = _skill_machine.STACK_TOP, _skill_machine.BASE

TOC = 0x1006d390
DATA_END = 0x100820c6          # end of bss (Ghidra block "PackedData" #2)


class Machine(_skill_machine.Machine):
    def load_level(self, data):
        """Put a Hlvl resource in memory as the current level (gLevelH, a
        handle) and build its tile kind tables (LoadTileDefinitions)."""
        h = self.alloc(4)
        p = self.alloc(len(data), clear=False)
        self.write(p, data)
        self.w32(h, p)
        self.w32(self.glob(0x10065B64), h)
        self.call("LoadTileDefinitions")
        return p
