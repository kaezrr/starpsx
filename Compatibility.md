Below is a list of games that I test for compatibility, getting these games working is my primary goal and an indicator of how good my emulator core is. If you want to add to or update this list, feel free to make a PR.

> [!IMPORTANT]
> All games tested are NTSC region versions only, i.e, either USA or Japan, with the exception of Earthworm Jim 2. My emulator doesn't support PAL timings yet, so if you try to run any PAL region game, YMMV.

|                  Name                   | Status | Notes                                                                |
| :-------------------------------------: | :----: | -------------------------------------------------------------------- |
|               Ape Escape                |   🔴   | Unimplemented SPU write                                              |
|   Castlevania - Symphony of the Night   |   🔴   | Gets stuck at Konami logo                                            |
|             Crash Bandicoot             |   🟢   |                                                                      |
| Crash Bandicoot 2 - Cortex Strikes Back |   🟢   |                                                                      |
|         CTR - Crash Team Racing         |   🔴   | Gets stuck after Sony screen, due to some CDROM issue                |
|              Dead or Alive              |   🟢   |                                                                      |
|               Dino Crisis               |   🔴   | Gets stuck at the gore disclaimer screen                             |
|                  Doom                   |   🔴   | Gets stuck after Sony screen                                         |
|             Earthworm Jim 2             |   🟢   |                                                                      |
|               Einhaender                |   🔴   | Index out of bounds in renderer                                      |
|            Final Fantasy IX             |   🔴   | Gets stuck after Sony screen                                         |
|            Final Fantasy VII            |   🟢   | Minor UI glitches in the menus.                                      |
|              Gran Turismo               |   🔴   | Unimplemented CDROM command 10h                                      |
|             Gran Turismo 2              |   🔴   | Unimplemented CDROM command 10h                                      |
|       Klonoa - Door to Phantomile       |   🟢   | Random has heavy stutters, likely due to DMA                         |
|               Mega Man 8                |   🟢   |                                                                      |
|               Mega Man X4               |   🟢   | Random black screens at the start before getting to gameplay         |
|            Metal Gear Solid             |   🟢   | Minor graphical bugs                                                 |
|            Mortal Kombat II             |   🟢   |                                                                      |
|              Parasite Eve               |   🟢   |                                                                      |
|             Puzzle Bobble 2             |   🟢   |                                                                      |
|              Resident Evil              |   🟢   | Audio issues                                                         |
|             Resident Evil 2             |   🟡   | Gets stuck after starting a new game                                 |
|        Resident Evil 3 - Nemesis        |   🟢   |                                                                      |
|               Ridge Racer               |   🟢   |                                                                      |
|               Silent Hill               |   🟢   | Random snowflakes vertex explosions                                  |
|            Spyro the Dragon             |   🟢   | Random heavy stutters in pause menu and while unlocking dragons      |
|         Street Fighter Alpha 3          |   🟢   |                                                                      |
|                 Tekken                  |   🟡   | Some CDDA issues, first few seconds of audio track repeats endlessly |
|                Tekken 2                 |   🔴   | Stuck due to infinite Linked List DMA                                |
|                Tekken 3                 |   🟡   | Awful performance due to large DMA transfers                         |
|               Tomb Raider               |   🔴   | Broken, some illegal instruction                                     |
|         Tony Hawks Pro Skater 2         |   🔴   | Unimplemented CDROM command 0Bh                                      |
|            Valkyrie Profile             |   🟡   | Battles are extremely laggy due to DMA timing                        |
|                 WipEout                 |   🔴   | Gets stuck after the Sony logo                                       |
|                Xenogears                |   🔴   | Unimplemented CDROM command 07h                                      |

## Status Key

- 🟢 - Works as far as I have tested, might have minor issues but nothing game breaking
- 🟡 - Boots and might start but has major issues that affects game-play
- 🔴 - Does not boot or work at all
