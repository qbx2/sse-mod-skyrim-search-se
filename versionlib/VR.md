# Skyrim VR (1.4.15) address data

`bin/versionlib-1-4-15-0.bin` is **not** an official Address Library file. Skyrim VR has no
format-2 `.bin`; the [VR Address Library](https://www.nexusmods.com/skyrimspecialedition/mods/58101)
is a CSV keyed by SE ids, and it does not contain the AE ids used in this code base.

The file is generated from that CSV (library version 0.276.1) with the AE ids below added:

```
python tools/csv_to_versionlib_bin.py version-1-4-15-0.csv versionlib/bin/versionlib-1-4-15-0.bin 14617=1a3f60 14720=1a5070 25259=391f90 51109=886dc0 190143=15ddce0 195816=1616e60 195890=1618e20 207886=16d6de0 401203=2fc4fe0 441582=2f8af0
```

| AE id  | Used as                 | SE 1.5.97 (tag `0.1`) | VR offset   | VR Address Library entry (SE id)           |
|--------|-------------------------|-----------------------|-------------|--------------------------------------------|
| 14617  | `look_up_by_id`         | `0x194230`            | `0x1a3f60`  | 14461 `TESForm::LookupFormByID`            |
| 14720  | `get_name`              | `0x196e10`            | `0x1a5070`  | none, see below                            |
| 25259  | `quest_get_description` | `0x382720`            | `0x391f90`  | 24778                                      |
| 51109  | `print_to_console`      | `0x85c290`            | `0x886dc0`  | none, see below                            |
| 190143 | `cell_vtable`           | `0x1566060`           | `0x15ddce0` | 235197 `VTABLE_TESObjectCELL`              |
| 195816 | `npc_vtable`            | `0x159fcd0`           | `0x1616e60` | 241857 `VTABLE_TESNPC`                     |
| 195890 | `quest_vtable`          | `0x15a1c98`           | `0x1618e20` | 242059 `VTABLE_TESQuest`                   |
| 207886 | `character_vtable`      | `0x165da40`           | `0x16d6de0` | 261397 `VTABLE_Character`                  |
| 401203 | `console_context`       | `0x2f000f0`           | `0x2fc4fe0` | 515064 `ConsoleLog` singleton              |
| 441582 | `target_addr`           | `0x2e75f0`            | `0x2f8af0`  | 21416 `Script::CompileAndRun`              |

The SE 1.5.97 column is what this plugin hard-coded at tag `0.1`, before it moved to versionlib.
Eight of the ten map to VR through the SE -> VR table of
[alandtse/skyrim_vr_address_library](https://github.com/alandtse/skyrim_vr_address_library) (`database.csv`),
and the resulting offsets are in the released VR CSV under the SE id shown.

The two without an entry were located in the unpacked executables:

- `get_name`: SE `0x196e10` sits `0x90` after `TESFullName::ReadFULLFromFileStream` (SE id 14546); the same
  function in VR is `0x1a6b40`. `0x1a5070` is the thunk `jmp 0x1a6b40`, the VR twin of SE's
  `0x195340: jmp 0x196e10`. Either address works; the thunk is the one that was tested in game.
- `print_to_console`: SE `0x85c290` is the variadic wrapper `0x30` bytes before `ConsoleLog::VPrint`
  (SE id 50180, SE `0x85c2c0`, VR `0x886df0`). VR `0x886dc0` disassembles to the same wrapper
  (spill `rdx`/`r8`/`r9`, `lea r8,[rsp+40h]`, `call 0x886df0`).

The vtable slots (`+0x30`, `+0x190`, `+0x198`) and struct offsets (`Script + 0x38`, `TESQuest + 0xe8`)
are the ones tag `0.1` already used on SE 1.5.97, and they are unchanged in VR: on `VTABLE_TESNPC`
the slots resolve to `TESNPC::Load` (SE id 24159), `TESForm::GetFormEditorID` (SE id 10827) and
`TESForm::SetFormEditorID` (SE id 10883).