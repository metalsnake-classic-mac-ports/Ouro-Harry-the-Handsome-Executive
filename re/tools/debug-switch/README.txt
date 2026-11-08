Harry Debug Switch 1.0
======================

Turns the hidden debug mode of "Harry the Handsome Executive" 1.0.1
(Ambrosia Software, 1999) on or off. The mode was left in the game but
switched off before release.

Runs on System 7 to Mac OS 9, on 68k and PowerPC Macs.


How to use it
-------------

1. Quit Harry if it is running.
2. Open "Harry Debug Switch" and choose the "Harry 1.0.1" application.
3. The program shows whether the debug mode is on or off and offers
   "Turn On" / "Turn Off". After switching it quits.

Work on a copy of Harry if you like to keep the original untouched.
Turning the mode off restores the original code exactly.


What the debug mode does
------------------------

The main menu says "CHEAT MODE ENABLED". Then:

In the main menu:
  New Game with Option held  asks for a level to start in (the "warp").
                             Note: a bug in Harry itself makes this
                             quit with "world file damaged" for every
                             level but 1. Use F11 in the game instead.

In the game:
  F1      comfort to 144
  F12     comfort to 0
  F2      staple gun and 100 staples
  F3      a caffeine pill
  F4      a door key
  F13     a soda can
  F11     go to the next level
  =       an explosion near Harry
  `       (held) Harry passes through walls
  C + S   (held together) save the game, as at a copier, anywhere

The keys are those of a US keyboard. Harry reads the key position, not
the letter printed on it: "=" is the key left of Delete, "`" the key
left of 1 (on a German keyboard the "<" or "^" key, depending on the
model). In an emulator the host's keyboard layout may move them.


For those who prefer to do it by hand
-------------------------------------

Harry 1.0.1 is a "fat" application: the PowerPC code lives in the
data fork, the 68k code in CODE resources. Each has the switch once.
Change both, or only the one your Mac runs. Search for the bytes (each
pattern occurs exactly once) with a hex editor such as HexEdit, or in
ResEdit / Resorcerer for the resource.

PowerPC (data fork, offset $298C):

  38 60 00 00 80 82 87 E4 98 64 00 00 80 A2 87 E0   off (original)
  38 60 00 01 80 82 87 E4 98 64 00 00 80 A2 87 E0   on

  The fourth byte (offset $298F) is the switch. In InitAppGlobals,
  "li r3,0" before "stb r3,gDebug" becomes "li r3,1".

68k (resource 'CODE' 1 "Main", offset $148A in the resource data):

  4E56 FFE0 2F0A 4239 0000 7F1A   off (original)
  4E56 FFE0 2F0A 50F9 0000 7F1A   on

  The word at $1490 is the switch. InitAppGlobals starts with
  "clr.b $7F1A" (gDebug); "st.b $7F1A" sets it instead.
  The CODE resources are marked "Protected": in ResEdit clear the flag
  in Get Resource Info before editing, and set it again afterwards.


Notes
-----

Only version 1.0.1 is recognised; for any other file the program
changes nothing. It does not touch Harry's data files, saved games or
registration.

Made for the Ouro-Harry project, a port of Harry to modern systems.
Built with Retro68.
