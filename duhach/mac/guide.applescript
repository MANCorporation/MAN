-- DUHACH! for Mac: a calm, step-by-step guide that gets an Apple Silicon Mac ready to
-- start MAN. It asks the questions in native macOS windows, then opens a Terminal
-- window that runs MAN's copy of the Asahi Linux installer (MAN-Mac/install-man.sh)
-- with the answers (MAN_CHOICE, MAN_SIZE), so Terminal only shows the progress and
-- asks for the Mac password.
-- Run by Contents/MacOS/DUHACH:  osascript guide.applescript <Resources folder>

on run argv
	set res to item 1 of argv
	set ttl to "DUHACH! for Mac"
	set appIcon to (POSIX file (res & "/duhach.icns")) as alias
	activate

	-- An Apple Silicon Mac with M1, M2 or M3?
	set arch to do shell script "uname -m"
	if arch is not "arm64" then
		display dialog "This Mac has an Intel processor." & return & return & "DUHACH! for Mac is for Macs with Apple Silicon (M1, M2, M3)." & return & return & "On an Intel Mac, MAN starts straight from the MAN USB stick: plug it in, then hold the Option key while the Mac starts." buttons {"Quit"} default button 1 with title ttl with icon appIcon
		return
	end if
	set chip to do shell script "sysctl -n machdep.cpu.brand_string"
	if not (chip contains "M1" or chip contains "M2" or chip contains "M3") then
		display dialog "This Mac has an " & chip & " chip." & return & return & "MAN runs on Macs with M1, M2 and M3 chips. Newer chips are not supported yet, so DUHACH! does not change anything on this Mac." buttons {"Quit"} default button 1 with title ttl with icon appIcon
		return
	end if

	-- Welcome
	display dialog "Welcome to DUHACH!" & return & return & "DUHACH! gets your Mac ready to start MAN, next to macOS." & return & return & "macOS, your apps and your files stay exactly as they are. DUHACH! only adds MAN's small boot loader and, if you want, space for MAN." & return & return & "It takes about 10 minutes." buttons {"Quit", "Continue"} default button "Continue" cancel button "Quit" with title ttl with icon appIcon

	display dialog "Before you start" & return & return & "- Back up your Mac with Time Machine." & return & "- Plug in the charger." & return & "- Connect to the internet: your Mac's own start-up files are downloaded from Apple." & return & "- Have your Mac password ready." buttons {"Quit", "I'm Ready"} default button "I'm Ready" cancel button "Quit" with title ttl with icon appIcon

	set choices to {"Install the MAN boot loader", "Install the MAN boot loader and make space for MAN", "Make space for MAN (the boot loader is already installed)"}
	set picked to choose from list choices with title ttl with prompt "What would you like to do?" & return & return & "The boot loader lets your Mac start MAN from the MAN USB stick or an external SSD. Nothing else on the Mac changes." & return & return & "Space for MAN is a part of this Mac's disk where you can later install MAN from the MAN USB stick." & return & return & "Not sure? Choose the first one: you can make space later." default items {item 1 of choices} OK button name "Continue" cancel button name "Quit"
	if picked is false then return
	set picked to item 1 of picked
	if picked is item 1 of choices then
		set c to "1"
	else if picked is item 2 of choices then
		set c to "2"
	else
		set c to "3"
	end if

	-- Space for MAN: macOS keeps 38 GB free, the boot loader needs about 4 GB.
	set sz to ""
	if c is not "1" then
		set freeGB to (do shell script "df -g / | awk 'NR==2 {print $4}'") as integer
		set maxGB to freeGB - 42
		if maxGB < 20 then
			display dialog "There is not enough free space on this Mac yet." & return & return & "macOS has " & freeGB & " GB free. MAN needs at least 20 GB, and macOS keeps 38 GB for itself so that it keeps working well." & return & return & "Empty the Trash or move some files to another disk, then open DUHACH! again." buttons {"Quit"} default button 1 with title ttl with icon appIcon
			return
		end if
		set suggested to 60
		if maxGB < suggested then set suggested to maxGB
		repeat
			set sizeReply to display dialog "How much space should MAN get?" & return & return & "Type a number of gigabytes (GB). MAN needs at least 20 GB; 60 GB or more is comfortable." & return & return & "This Mac can give MAN up to about " & maxGB & " GB." default answer (suggested as text) buttons {"Quit", "Continue"} default button "Continue" cancel button "Quit" with title ttl with icon appIcon
			try
				set n to (text returned of sizeReply) as integer
				if n >= 20 and n <= maxGB then exit repeat
			end try
			display dialog "Please type a number from 20 to " & maxGB & "." buttons {"OK"} default button 1 with title ttl with icon appIcon
		end repeat
		set sz to n as text
	end if

	-- What happens next
	if c is "3" then
		set steps to "1.  A Terminal window opens and shows the progress. Type your Mac password there when it asks. Nothing appears while you type: that is normal." & return & return
		set steps to steps & "2.  When it is done, the Mac shuts down by itself." & return & return & "3.  Plug in the MAN USB stick, press and HOLD the power button until \"Loading startup options\" appears, and choose MAN." & return & return & "4.  In MAN, open Install MAN and choose Install on This Mac."
	else
		set steps to "1.  A Terminal window opens and shows the progress. Type your Mac password there when it asks (twice). Nothing appears while you type: that is normal." & return & return
		set steps to steps & "2.  The Mac shuts down by itself." & return & return & "3.  Press and HOLD the power button until \"Loading startup options\" appears, and choose MAN. Apple asks for this one step in person." & return & return & "4.  Type your Mac password once more. The Mac restarts by itself: with the MAN USB stick or a MAN SSD plugged in, it starts MAN." & return & return & "To start macOS: hold the power button at start-up and choose Macintosh HD, or use Startup Disk in MAN."
	end if
	set chosen to "You chose: " & picked
	if sz is not "" then set chosen to chosen & return & "Space for MAN: " & sz & " GB"
	display dialog chosen & return & return & "What happens next:" & return & return & steps buttons {"Quit", "Start"} default button "Start" cancel button "Quit" with title ttl with icon appIcon

	-- Terminal runs the installer with the answers (a .command file needs no extra
	-- permission, unlike scripting Terminal).
	set tmpBase to (do shell script "mktemp -t duhach")
	set cmdFile to tmpBase & ".command"
	set inst to quoted form of (res & "/MAN-Mac/install-man.sh")
	do shell script "printf '%s\\n' '#!/bin/sh' 'clear' " & quoted form of ("MAN_CHOICE=" & c & " MAN_SIZE=" & sz & " exec /bin/sh " & inst) & " > " & quoted form of cmdFile & " && chmod 755 " & quoted form of cmdFile & " && rm -f " & quoted form of tmpBase & " && open -a Terminal " & quoted form of cmdFile

	display dialog "DUHACH! has opened Terminal." & return & return & "Follow the Terminal window: type your Mac password when it asks. You can close DUHACH! now." & return & return & steps buttons {"Close"} default button 1 with title ttl with icon appIcon
end run
