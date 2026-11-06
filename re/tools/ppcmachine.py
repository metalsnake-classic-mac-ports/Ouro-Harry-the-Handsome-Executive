"""Run routines of the PowerPC build of Harry in Unicorn.

The machine is the shared one in ouro-tools (re/ppc/ppcmachine.py, found
through OURO_TOOLS, default ../ouro-tools next to this repository); this
module adds what is Harry's: the addresses the recorders use and
loading a level. re/ghidra/run.sh must have run (re/work/ppc).

Usage:
    m = Machine("re/work/ppc")
    m.stub("SetRect", m.toolbox_setrect)    # most common ones are built in
    m.call("InitTileHotRects")
    m.read(addr, n)
"""
import importlib.util
import os

# the shared module has the same name as this one: load it under another
_tools = os.environ.get("OURO_TOOLS") or os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "..", "ouro-tools")
_spec = importlib.util.spec_from_file_location("ouro_ppcmachine", os.path.join(_tools, "re", "ppc", "ppcmachine.py"))
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
