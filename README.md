# SkyrimSearch SE (WIP)

It is new version of [Skyrim Search Plugin](https://www.nexusmods.com/skyrim/mods/50435/) for Skyrim Special Edition.

This plugin adds some console commands that allows you to search for cells, quests, and npcs by FormID/EditorID/Name using this plugin
(including CELL COC Codes, NPC RefID, QUEST Stages).
The point of using this plugin is that it SUPPORTS ALL (including 3rd-party) mods.

This plugin was not created by, and is not affiliated with, the website SkyrimSearch.com.

## Requirements
- [SKSE64](https://skse.silverlock.org/)
- SkyrimSE *1.5.97*

## Build Requirements
- [Rust](https://www.rust-lang.org/) nightly compiler
- `rustup target add x86_64-pc-windows-gnu`

## Build
```
cargo build
```
