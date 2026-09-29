/*
    chernarus_capture.sqf

    PROJECT-CREATED measurement harness for Project Chernarus.
    Not Bohemia Interactive or DayZ Mod code.

    STATUS: UNTESTED in ARMA 2 OA. Validate it first (tools/capture/README.md,
    "Validating the harness") before trusting any capture made with it.

    Records the player's position (relative to where the capture started),
    heading, speed and animation state once per frame for a fixed duration,
    one RPT log line per sample, via diag_log. Import the log with:

        parity import-rpt <path to the .RPT file>

    Usage (editor init line, trigger, or debug console):

        ["movement.stand_run_forward", 12] execVM "chernarus_capture.sqf";

    Parameters:
        0: STRING - scenario id (a file in tests/parity/scenarios)
        1: NUMBER - capture duration, seconds

    Output line format (protocol 1, parsed by crates/parity/src/rpt.rs):
        CHERNARUS_CAPTURE|1|<scenario>|<run>|BEGIN|<productVersion>|<worldName>|<typeOf player>
        CHERNARUS_CAPTURE|1|<scenario>|<run>|SAMPLE|t|dx|dy|dz|getDir|speed km/h|animationState
        CHERNARUS_CAPTURE|1|<scenario>|<run>|END

    Positions are written relative to the start because SQF number formatting
    keeps about six significant digits: absolute Chernarus coordinates would
    lose ~0.1 m, offsets keep sub-millimetre precision.
*/

private ["_scenario", "_duration", "_unit", "_tag", "_run", "_start", "_t0", "_t", "_p"];

_scenario = _this select 0;
_duration = _this select 1;
_unit = player;
_tag = "CHERNARUS_CAPTURE|1|";

// Run id: distinguishes repeated captures of one scenario in the same log.
_run = format ["%1x%2", floor (random 100000), (floor (diag_tickTime * 1000)) mod 100000];

_start = getPosASL _unit;
diag_log text format ["%1%2|%3|BEGIN|%4|%5|%6", _tag, _scenario, _run, productVersion, worldName, typeOf _unit];
hint format ["Capture %1 run %2: %3 s", _scenario, _run, _duration];

_t0 = diag_tickTime;
// waitUntil evaluates its condition at most once per frame in the scheduler,
// so samples are per frame but irregularly spaced; the parity metrics expect that.
waitUntil {
    _t = diag_tickTime - _t0;
    _p = getPosASL _unit;
    diag_log text format ["%1%2|%3|SAMPLE|%4|%5|%6|%7|%8|%9|%10", _tag, _scenario, _run, _t,
        (_p select 0) - (_start select 0),
        (_p select 1) - (_start select 1),
        (_p select 2) - (_start select 2),
        getDir _unit, speed _unit, animationState _unit];
    _t >= _duration
};

diag_log text format ["%1%2|%3|END", _tag, _scenario, _run];
hint format ["Capture %1 run %2 finished", _scenario, _run];
