-- Harry Debug Switch: turns the debug mode of Harry the Handsome
-- Executive 1.0.1 (PowerPC) on or off, on the Mac itself (AppleScript 1.4,
-- Mac OS 9). Open it in Script Editor and save it as an application; then
-- double-click it and choose Harry, or drop Harry on it.
--
-- The patch (NOTES.md, "Cheats / Easter eggs"; re/tools/sheepshaver_debug.py
-- does the same on the host): InitAppGlobals clears gDebug with
-- "li r3, 0; lwz r4, gDebug(RTOC); stb r3, 0(r4)" at data fork offset
-- 0x298c; "li r3, 1" sets it. Only the byte at 0x298f changes, and only
-- when the 16 bytes there are this code, so other versions are left alone.

property codeStart : 10637 -- data fork offset 0x298c, counted from 1
-- 38 60 00 ?? 80 82 87 e4 98 64 00 00 80 a2 87 e0
property expected : {56, 96, 0, -1, 128, 130, 135, 228, 152, 100, 0, 0, 128, 162, 135, 224}

on run
	set f to choose file with prompt "Choose Harry the Handsome Executive 1.0.1:"
	switchDebug(f)
end run

on open theFiles
	repeat with f in theFiles
		switchDebug(contents of f)
	end repeat
end open

on switchDebug(f)
	try
		set fh to open for access f with write permission
	on error msg number num
		display dialog "Cannot open the file for writing (is Harry running?)." & return & msg & " (" & num & ")" buttons {"OK"} default button 1 with icon stop
		return
	end try
	try
		set flag to debugFlag(fh)
		if flag is -1 then
			close access fh
			display dialog "This is not Harry 1.0.1 for PowerPC: the code to patch is not there. Nothing was changed." buttons {"OK"} default button 1 with icon stop
			return
		end if
		if flag is 1 then
			set state to "on"
			set other to "Turn off"
		else
			set state to "off"
			set other to "Turn on"
		end if
		set answer to button returned of (display dialog "Harry's debug mode is " & state & "." buttons {"Cancel", other} default button 2)
		write (ASCII character (1 - flag)) to fh starting at codeStart + 3
		close access fh
		if flag is 1 then
			display dialog "Debug mode is now off." buttons {"OK"} default button 1
		else
			display dialog "Debug mode is now on. Keys: F1 comfort 144, F2 staple gun, F3 caffeine, F4 a door key, F11 next level, F12 comfort 0, F13 soda can, = explosion, ` held: through walls." buttons {"OK"} default button 1
		end if
	on error msg number num
		close access fh
		if num is not -128 then display dialog msg & " (" & num & ")" buttons {"OK"} default button 1 with icon stop
	end try
end switchDebug

-- 0 or 1 when the code is there, otherwise -1.
on debugFlag(fh)
	try
		set b to read fh from codeStart for 16
	on error
		return -1
	end try
	if (count b) is not 16 then return -1
	repeat with i from 1 to 16
		set want to item i of expected
		set got to ASCII number (character i of b)
		if want is -1 then
			if got is not 0 and got is not 1 then return -1
			set flag to got
		else if got is not want then
			return -1
		end if
	end repeat
	return flag
end debugFlag
