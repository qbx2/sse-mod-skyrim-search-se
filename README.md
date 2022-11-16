# Skyrim Search SE

This is new version of [Skyrim Search Plugin](https://www.nexusmods.com/skyrim/mods/50435/) for Skyrim Special Edition.

This plugin adds some console commands that allows you to search for NPCs, cells, and quests by FormID/EditorID/Name.
Moreover, you can find refIds of NPCs, codes for `coc` command, and quest stage number for `completequest` command.
The most important point of using this plugin over other tools is that it works in game, and fully-synchronized with your environment.
(it can search custom followers too!)

## Usage
Skyrim console usage: [https://en.uesp.net/wiki/Skyrim:Console](https://en.uesp.net/wiki/Skyrim:Console)

## Basic
The command added by this plugin is `ss` (or `skyrimsearch`).
You can view usage by typing `ss --help` in game.
Also, You can view your inputs and outputs in log file in `\My Games\Skyrim Special Edition\SKSE\skyrim-search-se.log`

* help command: `ss --help`
```
ss --help
skyrim-search-se 0.7.0
Author: qbx2/lukasaldersley | GitHub: https://github.com/qbx2/sse-mod-skyrim-search-se

USAGE:
    ss [FLAGS] <SUBCOMMAND>

FLAGS:
        --debug
    -h, --help       Prints help information
    -V, --version    Prints version information

SUBCOMMANDS:
    cell           search cell (location)
    npc            search npc/reference
    quest          search quest
    quest_stage    search quest (prints additional stage information)
    raw            execute raw query. quote your query as in unix shell if needed.
```
## Search NPCs
- command: `ss npc <query>`
- query: FormID/EditorId/Name/RefId of the npc which you want to search

* Search by name
```
ss npc lydia
 form_id  | editor_id         | name  | ref_id
----------+-------------------+-------+----------
 000A2C8E | HousecarlWhiterun | Lydia | 000A2C94
```

* Search by EditorId

(In my case, there were no reference to `HousecarlMarkarth`/`HousecarlSolitude`/`HousecarlWindhelm`)
```
ss npc housecarl
 form_id  | editor_id               | name                    | ref_id
----------+-------------------------+-------------------------+----------
 000A2C8C | HousecarlMarkarth       | Argis the Bulwark       | <null>
 000A2C8E | HousecarlWhiterun       | Lydia                   | 000A2C94
 000A2C8F | HousecarlSolitude       | Jordis the Sword-Maiden | <null>
 000A2C90 | HousecarlWindhelm       | Calder                  | <null>
 000A2C91 | HousecarlRiften         | Iona                    | 000A2C93
 03005215 | BYOHHousecarlFalkreath  | Rayya                   | 03005216
 0300521B | BYOHHousecarlHjaalmarch | Valdimar                | 0300521D
 0300521E | BYOHHousecarlPale       | Gregor                  | 0300521F
```
* Search by FormId/RefId
```
ss npc a2c8e
 form_id  | editor_id         | name  | ref_id
----------+-------------------+-------+----------
 000A2C8E | HousecarlWhiterun | Lydia | 000A2C94

ss npc a2c94
 form_id  | editor_id         | name  | ref_id
----------+-------------------+-------+----------
 000A2C8E | HousecarlWhiterun | Lydia | 000A2C94
 ```
## Search Cells
- command: `ss cell <query>`
- query: FormID/EditorId/Name of the cell which you want to search

```
ss cell breezehome
 form_id  | editor_id          | name
----------+--------------------+------------
 000165A8 | WhiterunBreezehome | Breezehome
 ```

## Search Quests
- command: `ss quest <query>`
- query: FormID/EditorId/Name of the quest which you want to search

```
ss quest forbidden legend
 form_id  | editor_id        | name
----------+------------------+------------------
 000E4D31 | dunGauldursonQST | Forbidden Legend
```

## Search Quest Stages
- command: `ss quest_stage <query>` / `ss qs <query>`
- query: FormID/EditorId/Name of the quest which you want to search

```
ss qs forbidden
 form_id  | editor_id        | name             | stage | log
----------+------------------+------------------+-------+---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------
 000E4D31 | dunGauldursonQST | Forbidden Legend | 6     | In Reachwater Rock, I found a cryptic message that said the tomb here had been sealed, and should be forgotten forever. What is the story of this place?
 000E4D31 | dunGauldursonQST | Forbidden Legend | 7     | In Folgunthur, I found a cryptic message on the body of a powerful Draugr, condemning him for his ancient crimes. What was his story? Why was he entombed with a broken amulet?
 000E4D31 | dunGauldursonQST | Forbidden Legend | 8     | In Geirmund's Hall, I found a cryptic message on the body of a powerful Draugr, condemning him for his ancient crimes. What was his story? Why was he entombed with a broken amulet?
 000E4D31 | dunGauldursonQST | Forbidden Legend | 9     | In Saarthal, I found a cryptic message on the body of a powerful Draugr, condemning him for his ancient crimes. What was his story? Why was he entombed with a broken amulet?
 000E4D31 | dunGauldursonQST | Forbidden Legend | 10    | Long ago, the Archmage Gauldur was murdered, and his three sons were hunted down by King Harald's personal battlemage. The entire incident was covered up, their names struck from every record. But the legend survived. Perhaps someone still knows the truth of this ancient tale.
 000E4D31 | dunGauldursonQST | Forbidden Legend | 20    | Long ago, the Archmage Gauldur was murdered, and his three sons were hunted down by King Harald's personal battlemage. The mage Daynas Valen spent his life searching for the truth of this tale, and came to Folgunthur with the key needed to unlock its secret.
 000E4D31 | dunGauldursonQST | Forbidden Legend | 30    | Long ago, the Archmage Gauldur was murdered by his three sons, who stole his amulet of power and divided it among themselves. The brothers were hunted down in secret and sealed in tombs across Skyrim. To reclaim the amulet, I will need to seek out their final resting places.
 000E4D31 | dunGauldursonQST | Forbidden Legend | 100   | Long ago, the Archmage Gauldur was murdered by his three sons, who stole his amulet of power and divided it among themselves. I defeated the brothers and reclaimed the fragments of the amulet. Perhaps Gauldur's tomb holds the secret to restoring it to its original form.
 000E4D31 | dunGauldursonQST | Forbidden Legend | 105   | Long ago, the Archmage Gauldur was murdered by his three sons, who stole his amulet of power and divided it among themselves. I collected the fragments of the shattered amulet and brought them to Gauldur's tomb, where the ghosts of the three brothers ambushed me.
 000E4D31 | dunGauldursonQST | Forbidden Legend | 150   | Long ago, the Archmage Gauldur was murdered by his three sons, who stole his amulet of power and divided it among themselves. I defeated the undead brothers, located the fragments of the shattered amulet, and forged it anew in Gauldur's tomb.
```

## Raw Query (Advanced)
- command: `ss raw <sql>`
- SQL: The [SQLite](https://sqlite.org/) SQL.
- schema: Refer to the [source code](src/db.rs)

* Query example

(Note that you may quote your sql because the input is parsed by shlex)
```
ss raw SELECT * FROM npc WHERE form_id > 0xa2c00 AND form_id < 0xa2d00;
 form_id  | editor_id             | name
----------+-----------------------+-------------------------
 000A2C8C | HousecarlMarkarth     | Argis the Bulwark
 000A2C8E | HousecarlWhiterun     | Lydia
 000A2C8F | HousecarlSolitude     | Jordis the Sword-Maiden
 000A2C90 | HousecarlWindhelm     | Calder
 000A2C91 | HousecarlRiften       | Iona
 000A2CAF | DA01LvlDremoraWarlock |
 000A2CEB | ArgonianMalePreset03  |
 000A2CEF | ArgonianMalePreset04  |
 000A2CF0 | ArgonianMalePreset05  |

```
## Requirements
- SkyrimSE(AE) [click here to view runtime version](target_version.txt)
- [SKSE64](https://skse.silverlock.org/), matching game version

## Build Requirements
- [MinGW64: mingw-w64-install.exe (For windows users)](https://sourceforge.net/projects/mingw-w64/files/Toolchains%20targetting%20Win32/Personal%20Builds/mingw-builds/installer/mingw-w64-install.exe) needs to be installed with the x86_64 option NOT i686, and you need add its bin folder to the PATH system variable
- Latest stable [Rust](https://www.rust-lang.org/) compiler

## Build
```
cargo build
```

### Credits
- [kmdreko](https://stackoverflow.com/users/2189130/kmdreko) on Stack Overflow for helping with some Rust problems
- [meh321](https://www.nexusmods.com/skyrimspecialedition/mods/32444) for distributing versionlib for easy update

### Disclaimer
This plugin was not created by, and is not affiliated with, the website SkyrimSearch.com.
