/*
 * Harry Debug Switch: turns the debug mode of Harry the Handsome Executive
 * 1.0.1 on or off, on the Mac itself (68k, System 7 to Mac OS 9). Harry is
 * FAT, so both builds are switched:
 *
 * - PowerPC (the PEF in the data fork): InitAppGlobals clears gDebug with
 *   "li r3, 0; lwz r4, gDebug(RTOC); stb r3, 0(r4)" at data fork offset
 *   0x298c; "li r3, 1" sets it (the byte at 0x298f).
 * - 68k (CODE 1): InitAppGlobals starts "link a6, #-$20; move.l a2, -(a7);
 *   clr.b $7f1a.l" (gDebug, A5-relative data, relocated at load time) at
 *   resource offset 0x148a; "st.b $7f1a.l" sets it to 0xff (every reader
 *   tests it with tst.b). Only the opcode word at 0x1490 changes (4239 /
 *   50f9); the address and its relocation stay.
 *
 * A part is changed only when its bytes are exactly one of the two forms,
 * so other versions are left alone. NOTES.md, "Cheats / Easter eggs".
 *
 * After a switch the program quits; after Cancel or a refused file it asks
 * for the next one. README.txt goes beside it.
 *
 * Build: re/tools/debug-switch/build.sh (Retro68, ouro-tools/retro68-build).
 */

#include <Quickdraw.h>
#include <Fonts.h>
#include <Windows.h>
#include <Menus.h>
#include <TextEdit.h>
#include <Dialogs.h>
#include <Files.h>
#include <Resources.h>
#include <StandardFile.h>
#include <Processes.h>
#include <TextUtils.h>
#include <string.h>

enum { kAskOnAlert = 128, kInfoAlert = 129, kAskOffAlert = 130 };
static const unsigned char kEmpty[1] = {0};
enum { kOff = 0, kOn = 1, kMissing = -1 };

#define PPC_OFFSET 0x298cL
static const unsigned char ppcCode[16] = {
    0x38, 0x60, 0x00, 0x00, 0x80, 0x82, 0x87, 0xe4,
    0x98, 0x64, 0x00, 0x00, 0x80, 0xa2, 0x87, 0xe0};
#define PPC_FLAG 3

#define M68K_OFFSET 0x148aL
static const unsigned char m68kOff[12] = {
    0x4e, 0x56, 0xff, 0xe0, 0x2f, 0x0a, 0x42, 0x39, 0x00, 0x00, 0x7f, 0x1a};
#define M68K_OP 6 /* the opcode word: 4239 clr.b, 50f9 st.b */

/* Shows a message; ^0 in ALRT 129. */
static void Say(const char *text)
{
    Str255 s;
    size_t n = strlen(text);
    if (n > 255)
        n = 255;
    s[0] = (unsigned char)n;
    memcpy(s + 1, text, n);
    ParamText(s, kEmpty, kEmpty, kEmpty);
    NoteAlert(kInfoAlert, NULL);
}

static void SayError(const char *what, OSErr err)
{
    char text[200];
    char num[16];
    Str255 ns;
    NumToString(err, ns);
    memcpy(num, ns + 1, ns[0]);
    num[ns[0]] = 0;
    strcpy(text, what);
    strcat(text, " (error ");
    strcat(text, num);
    strcat(text, ")");
    Say(text);
}

/* The PowerPC part: kOff, kOn or kMissing. */
static int PpcState(short ref)
{
    unsigned char b[16];
    long count = sizeof b;
    int i;
    if (SetFPos(ref, fsFromStart, PPC_OFFSET) != noErr || FSRead(ref, &count, b) != noErr || count != 16)
        return kMissing;
    for (i = 0; i < 16; i++)
        if (i != PPC_FLAG && b[i] != ppcCode[i])
            return kMissing;
    if (b[PPC_FLAG] > 1)
        return kMissing;
    return b[PPC_FLAG];
}

static OSErr SetPpc(short ref, int on)
{
    unsigned char v = on ? 1 : 0;
    long count = 1;
    OSErr err = SetFPos(ref, fsFromStart, PPC_OFFSET + PPC_FLAG);
    if (err == noErr)
        err = FSWrite(ref, &count, &v);
    return err;
}

/* The 68k part of CODE 1 (locked by the caller): kOff, kOn or kMissing. */
static int M68kState(Handle code)
{
    unsigned char *p;
    int i;
    if (code == NULL || GetHandleSize(code) < M68K_OFFSET + 12)
        return kMissing;
    p = (unsigned char *)*code + M68K_OFFSET;
    for (i = 0; i < 12; i++)
        if (i != M68K_OP && i != M68K_OP + 1 && p[i] != m68kOff[i])
            return kMissing;
    if (p[M68K_OP] == 0x42 && p[M68K_OP + 1] == 0x39)
        return kOff;
    if (p[M68K_OP] == 0x50 && p[M68K_OP + 1] == 0xf9)
        return kOn;
    return kMissing;
}

static const char *Name(int state)
{
    return state == kOn ? "on" : state == kOff ? "off" : "not found";
}

/* Whether the file is a running application (its resource file is open
   then, and closing it would pull it from under the program). */
static Boolean Running(const FSSpec *spec)
{
    ProcessSerialNumber psn = {0, kNoProcess};
    ProcessInfoRec info;
    FSSpec app;
    while (GetNextProcess(&psn) == noErr) {
        memset(&info, 0, sizeof info);
        info.processInfoLength = sizeof info;
        info.processAppSpec = &app;
        if (GetProcessInformation(&psn, &info) == noErr && app.vRefNum == spec->vRefNum &&
            app.parID == spec->parID && EqualString(app.name, spec->name, false, true))
            return true;
    }
    return false;
}

/* Switches one file; true when it was switched (then the program quits). */
static Boolean SwitchFile(FSSpec *spec)
{
    Boolean switched = false;
    short dataRef = 0, resRef = -1, oldRes = CurResFile();
    Handle code = NULL;
    int ppc = kMissing, m68k = kMissing, want;
    short item;
    char text[255];
    OSErr err;

    if (Running(spec)) {
        Say("That program is running. Quit it first.");
        return false;
    }
    err = FSpOpenDF(spec, fsRdWrPerm, &dataRef);
    if (err == noErr)
        ppc = PpcState(dataRef);
    else
        dataRef = 0;

    resRef = FSpOpenResFile(spec, fsRdWrPerm);
    if (resRef == oldRes) /* this program itself: its file stays open */
        resRef = -1;
    if (resRef != -1) {
        UseResFile(resRef);
        SetResLoad(true);
        code = Get1Resource('CODE', 1);
        if (code != NULL) {
            HNoPurge(code);
            HLock(code);
            m68k = M68kState(code);
        }
    }

    if (ppc == kMissing && m68k == kMissing) {
        if (dataRef == 0 && resRef == -1)
            Say("Cannot open the file for writing. Is Harry running?");
        else
            Say("This is not Harry the Handsome Executive 1.0.1: the code to switch is not there. Nothing was changed.");
        goto done;
    }

    strcpy(text, "Harry's debug mode is ");
    if (ppc == m68k || m68k == kMissing)
        strcat(text, Name(ppc));
    else if (ppc == kMissing)
        strcat(text, Name(m68k));
    else
        strcat(text, "mixed");
    strcat(text, " (PowerPC: ");
    strcat(text, Name(ppc));
    strcat(text, ", 68k: ");
    strcat(text, Name(m68k));
    strcat(text, ").");
    {
        Str255 s;
        s[0] = (unsigned char)strlen(text);
        memcpy(s + 1, text, s[0]);
        ParamText(s, kEmpty, kEmpty, kEmpty);
    }
    /* The default button (item 1) is the change: Turn Off when it is on. */
    if (ppc == kOn || (ppc == kMissing && m68k == kOn)) {
        item = NoteAlert(kAskOffAlert, NULL);
        want = item == 2;
    } else {
        item = NoteAlert(kAskOnAlert, NULL);
        want = item == 1;
    }
    if (item != 1 && item != 2)
        goto done;

    if (ppc != kMissing && ppc != want) {
        err = SetPpc(dataRef, want);
        if (err != noErr) {
            SayError("Writing the PowerPC part failed", err);
            goto done;
        }
    }
    if (m68k != kMissing && m68k != want) {
        unsigned char *p = (unsigned char *)*code + M68K_OFFSET + M68K_OP;
        p[0] = want ? 0x50 : 0x42;
        short attrs = GetResAttrs(code);
        p[1] = want ? 0xf9 : 0x39;
        /* CODE resources are protected (and purgeable): the Resource
           Manager writes them only with the bit off; it goes back on. */
        SetResAttrs(code, attrs & ~resProtected);
        ChangedResource(code);
        err = ResError();
        if (err == noErr) {
            WriteResource(code);
            err = ResError();
        }
        SetResAttrs(code, attrs);
        if (err != noErr) {
            SayError("Writing the 68k part failed", err);
            goto done;
        }
    }
    Say(want ? "Debug mode is now on. Keys in the game: F1 comfort 144, F2 staple gun, F3 caffeine, F4 a door key, F11 next level, F12 comfort 0, F13 soda can, = explosion, ` held: through walls. More in README.txt."
             : "Debug mode is now off.");
    switched = true;

done:
    if (code != NULL)
        HUnlock(code);
    if (resRef != -1)
        CloseResFile(resRef); /* writes the change */
    UseResFile(oldRes);
    if (dataRef != 0)
        FSClose(dataRef);
    return switched;
}

int main(void)
{
    SFTypeList types = {'APPL'};
    StandardFileReply reply;

    InitGraf(&qd.thePort);
    InitFonts();
    InitWindows();
    InitMenus();
    TEInit();
    InitDialogs(NULL);
    InitCursor();

    for (;;) {
        StandardGetFile(NULL, 1, types, &reply);
        if (!reply.sfGood)
            break;
        if (SwitchFile(&reply.sfFile))
            break;
    }
    return 0;
}
