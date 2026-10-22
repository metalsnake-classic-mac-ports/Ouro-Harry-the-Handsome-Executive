// Ghidra headless post-script: build the "Sprite" structure from
// re/ghidra/sprite.txt and make it the type of the first parameter of every
// routine that takes a sprite (name pattern below).
// Arguments: spec file, struct size (hex).
import ghidra.app.script.GhidraScript;
import ghidra.app.decompiler.*;
import ghidra.program.model.pcode.HighFunctionDBUtil;
import ghidra.program.model.data.*;
import ghidra.program.model.listing.*;
import ghidra.program.model.symbol.SourceType;
import java.nio.file.*;

public class ApplyStruct extends GhidraScript {
    static final String PATTERN =
        "(Setup|Handle|Hit|Init)\\w*Sprite\\w*|DoScoot|DoKick|DoInch|MoveChair|HandleKeys|" +
        "HandlePlayerGlide|HandleGlide|HandleHalfPipe|ApplyModedImpulse|EnforceMaxSpeed|" +
        "JumpScale|SpriteCenter|RectBounce\\w*|WallBounce\\w*|BGTileBounce\\w*|HBounce|VBounce|" +
        "GrateBounce|HandleRampCollision|HandleStairsCollision|ApproachMode|DoorSeparate|" +
        "Find\\w*Direction\\w*|Accelerate\\w*|Assign\\w*ShotSpeed|Random\\w*Shot|ThrowDart|" +
        "MakeNewShot|FindHeldWeaponPos|HandleRepulsions|RobotTooFar|ShowBalloon";

    @Override
    public void run() throws Exception {
        String[] args = getScriptArgs();
        int size = Integer.parseInt(args[1].replace("0x", ""), 16);
        DataTypeManager dtm = currentProgram.getDataTypeManager();
        StructureDataType rect = new StructureDataType("Rect", 0);
        for (String f : new String[] {"top", "left", "bottom", "right"}) rect.add(ShortDataType.dataType, f, null);
        StructureDataType s = new StructureDataType("Sprite", size);
        for (String line : Files.readAllLines(Paths.get(args[0]))) {
            line = line.trim();
            if (line.isEmpty() || line.startsWith("#")) continue;
            String[] p = line.split("\\s+", 4);
            int off = Integer.decode(p[0]);
            DataType t;
            switch (p[1]) {
                case "short": t = ShortDataType.dataType; break;
                case "int": t = IntegerDataType.dataType; break;
                case "char": t = CharDataType.dataType; break;
                case "Rect": t = rect; break;
                default: t = PointerDataType.dataType;
            }
            s.replaceAtOffset(off, t, t.getLength(), p[2], p.length > 3 ? p[3] : null);
        }
        DataType sd = dtm.addDataType(s, DataTypeConflictHandler.REPLACE_HANDLER);
        DataType sp = new PointerDataType(sd);
        int n = 0;
        DecompInterface ifc = new DecompInterface();
        ifc.openProgram(currentProgram);
        for (Function f : currentProgram.getFunctionManager().getFunctions(true)) {
            String name = f.getName();
            if (!name.matches(PATTERN)) continue;
            if (f.getParameterCount() == 0) {
                // parameters exist only in the decompiler until committed
                DecompileResults r = ifc.decompileFunction(f, 60, monitor);
                if (r.decompileCompleted()) {
                    HighFunctionDBUtil.commitParamsToDatabase(r.getHighFunction(), true,
                        HighFunctionDBUtil.ReturnCommitOption.NO_COMMIT, SourceType.ANALYSIS);
                }
            }
            if (f.getParameterCount() == 0) continue;
            Parameter p0 = f.getParameter(0);
            p0.setDataType(sp, SourceType.USER_DEFINED);
            p0.setName("sp", SourceType.USER_DEFINED);
            n++;
        }
        println("ApplyStruct: Sprite (" + size + " bytes) on " + n + " routines");
    }
}
