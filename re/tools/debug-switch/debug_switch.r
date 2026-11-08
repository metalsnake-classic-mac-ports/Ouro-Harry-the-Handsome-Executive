#include "Dialogs.r"
#include "Processes.r"
#include "MacTypes.r"

/* The question: ^0 the state, then Turn On (the default) / Turn Off /
   Cancel; ALRT 130 the same with Turn Off the default, for when it is on. */
resource 'ALRT' (128) {
	{ 60, 60, 190, 460 }, 128,
	{ OK, visible, silent; OK, visible, silent; OK, visible, silent; OK, visible, silent },
	alertPositionMainScreen
};

resource 'DITL' (128) {
	{
		{ 100, 300, 120, 390 }, Button { enabled, "Turn On" };
		{ 100, 200, 120, 290 }, Button { enabled, "Turn Off" };
		{ 100, 100, 120, 190 }, Button { enabled, "Cancel" };
		{ 10, 70, 90, 390 }, StaticText { disabled, "^0" };
	}
};

resource 'ALRT' (130) {
	{ 60, 60, 190, 460 }, 130,
	{ OK, visible, silent; OK, visible, silent; OK, visible, silent; OK, visible, silent },
	alertPositionMainScreen
};

resource 'DITL' (130) {
	{
		{ 100, 300, 120, 390 }, Button { enabled, "Turn Off" };
		{ 100, 200, 120, 290 }, Button { enabled, "Turn On" };
		{ 100, 100, 120, 190 }, Button { enabled, "Cancel" };
		{ 10, 70, 90, 390 }, StaticText { disabled, "^0" };
	}
};

/* A message: ^0, then OK. */
resource 'ALRT' (129) {
	{ 60, 60, 210, 460 }, 129,
	{ OK, visible, silent; OK, visible, silent; OK, visible, silent; OK, visible, silent },
	alertPositionMainScreen
};

resource 'DITL' (129) {
	{
		{ 120, 310, 140, 390 }, Button { enabled, "OK" };
		{ 10, 70, 110, 390 }, StaticText { disabled, "^0" };
	}
};

resource 'vers' (1) {
	0x01, 0x00, release, 0x00, verUS,
	"1.0",
	"1.0, turns Harry 1.0.1's debug mode on or off"
};

resource 'SIZE' (-1) {
	reserved,
	acceptSuspendResumeEvents,
	reserved,
	canBackground,
	doesActivateOnFGSwitch,
	backgroundAndForeground,
	dontGetFrontClicks,
	ignoreChildDiedEvents,
	is32BitCompatible,
	isHighLevelEventAware,
	onlyLocalHLEvents,
	notStationeryAware,
	dontUseTextEditServices,
	reserved,
	reserved,
	reserved,
	256 * 1024,
	256 * 1024
};
