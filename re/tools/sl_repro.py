"""Run the original Harry (PowerPC) in systemless from one of the
`ouro-harry-repro` saves, headless, and take screenshots.

    python3 -I re/tools/sl_repro.py SAVE [STEPS...] [--out DIR]

SAVE is a name in re/work/repro (written by `ouro-harry-repro`). STEPS
after the game is on screen:
    left:N right:N down:N up:N space:N ctrl:N option:N return:N tab:N
                    hold the key N frontend ticks (60.15 a second)
    wait:N          let N ticks pass
    shot:NAME       a screenshot NAME.png
    frames:N        a screenshot every tick for N ticks (f000.png ...)

It builds re/work/systemless/harry: the installed game files, unchanged,
the save (as "Repro SAVE": the Open dialog selects the first name) and a
licence file in systemless's Preferences folder (harry_license.py) - the
unregistered original refuses levels after 5. systemless's own Toolbox
draws it, so it is for looking (BUGS.md "See it"), not the reference
(NOTES.md "Emulator"). Needs `systemless` (brew install
benletchford/tap/systemless).
"""
import importlib.util
import json
import pathlib
import shutil
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[2]
WORK = ROOT / 're/work'
GAME_FILES = ['Harry 1.0.1.bin', 'Episode 1.bin', 'Harry Graphics.bin', 'Harry Instruments.bin',
              'Harry Music.bin', 'Harry Sounds.bin', 'Harry Titles.bin']
LICENSEE = 'Ouro Test'
KEYS = {'left': (123, 28), 'right': (124, 29), 'down': (125, 31), 'up': (126, 30), 'space': (49, 32),
        'ctrl': (59, 0), 'option': (58, 0), 'return': (36, 13), 'tab': (48, 9)}


def build(save):
    game = WORK / 'systemless/harry'
    if game.exists():
        shutil.rmtree(game)
    game.mkdir(parents=True)
    for f in GAME_FILES:
        shutil.copyfile(WORK / 'installed' / f, game / f)
    spec = importlib.util.spec_from_file_location('harry_license', ROOT / 're/tools/harry_license.py')
    lic = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(lic)
    lic.write(game, LICENSEE)
    src = WORK / 'repro' / save
    if not src.exists():
        sys.exit(f'{src}: no such save (run ouro-harry-repro first)')
    # cp keeps the resource fork and the Finder info; the Open dialog
    # selects the first name, which must sort before "System Folder"
    subprocess.run(['cp', str(src), str(game / f'Repro {save}')], check=True)
    return game


def click(v, h):
    return [{'type': 'mouse_down', 'v': v, 'h': h}, {'type': 'run', 'ticks': 3},
            {'type': 'mouse_up', 'v': v, 'h': h}]


def key(name, ticks):
    k, ch = KEYS[name]
    return [{'type': 'key_down', 'key': k, 'ch': ch}, {'type': 'run', 'ticks': ticks},
            {'type': 'key_up', 'key': k, 'ch': ch}]


def script(steps):
    # the 800 x 600 screen: the menu's Continue Game, the dialog's Open
    a = [{'type': 'run', 'ticks': 1200}]
    a += click(148, 630) + [{'type': 'run', 'ticks': 180}]
    a += click(357, 519) + [{'type': 'run', 'ticks': 120}]
    # a level's message balloon waits for a key
    a += key('return', 4) + [{'type': 'run', 'ticks': 90}, {'type': 'screenshot', 'name': 'start'}]
    for s in steps:
        what, _, n = s.partition(':')
        if what in KEYS:
            a += key(what, int(n))
        elif what == 'wait':
            a.append({'type': 'run', 'ticks': int(n)})
        elif what == 'shot':
            a.append({'type': 'screenshot', 'name': n})
        elif what == 'frames':
            for i in range(int(n)):
                a += [{'type': 'run', 'ticks': 1}, {'type': 'screenshot', 'name': f'f{i:03d}'}]
        else:
            sys.exit(f'unknown step {s}')
    ticks = sum(x['ticks'] for x in a if x['type'] == 'run')
    return {'version': 1, 'clock': 'frontend_ticks', 'max_ticks': ticks + 60, 'actions': a}


def main():
    args = sys.argv[1:]
    out = None
    if '--out' in args:
        i = args.index('--out')
        out = pathlib.Path(args[i + 1])
        del args[i:i + 2]
    if not args:
        sys.exit(__doc__)
    save, steps = args[0], args[1:]
    game = build(save)
    out = out or WORK / 'systemless' / f'out-{save}'
    if out.exists():
        shutil.rmtree(out)
    path = WORK / 'systemless' / f'{save}.json'
    path.write_text(json.dumps(script(steps), indent=1))
    r = subprocess.run(['systemless', str(game), '--headless', '--screen-depth', '8',
                        '--play-script', str(path), '--play-output', str(out)],
                       capture_output=True, text=True)
    report = json.loads((out / 'report.json').read_text()) if (out / 'report.json').exists() else {}
    print(report.get('status', 'no report'), report.get('error') or '', out)
    if r.returncode:
        print(r.stderr[-2000:])
    sys.exit(r.returncode)


if __name__ == '__main__':
    main()
