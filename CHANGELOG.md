# Changelog

## 0.3.3

- Import your playtime from Playnite: Settings → Libraries → Playnite → "Import playtime". Close Playnite first.
- Games are found by their file, or by their title when it is unique. Steam games keep the playtime that comes from Steam.
- Importing again replaces the hours taken from Playnite instead of adding them twice, and the time counted by allone is kept.

## 0.3.2

- allone now checks for updates on its own: shortly after starting, every 6 hours, and when you come back to its window.
- A clearly visible banner at the top of every page announces a new version, with the list of changes, an "Update" button and a "Later" button.
- After an update, allone shows what changed in the new version.

## 0.3.1

- Covers fill the whole card again, without blurred margins. Covers taller or wider than the card are slightly cropped at the edges.

## 0.3.0

- Gamepad support: move around the whole app with the D-pad or left stick, A to open or play, B to go back, LB/RB to switch sections. Dialogs keep the selection inside them.
- The connected controller is shown in the top bar, and button hints match its family (A/B for Xbox, ✕/○ for PlayStation).
- allone ignores the controller while a game is running, and buttons still held when you come back do nothing until released.
- Home carousel: left/right switches the featured game, and it stops rotating while you are on it.
- Release files now have readable names: `allone-windows-setup.exe`, `allone-windows.msi`, `allone-linux.AppImage`, `allone-linux.deb`.

## 0.2.0

- Library of emulated games (any emulator, configured with a `{rom}` argument template) and native PC ports.
- Steam import of installed games: launched through Steam, playtime read from Steam.
- Real covers, heroes and logos from the LaunchBox Games Database and the Steam store, with a "wrong game?" picker.
- Custom consoles added from the LaunchBox platform catalogue, with logos from Wikimedia Commons.
- Themes with an animated gradient background (Paper Shaders).
- Signed automatic updates, installed only after a click and never while a game is running.
- Data from older versions is migrated automatically to the new data folder (`com.allone.launcher`).
- The theme choice is reset once after upgrading from an older version; pick your theme again in Settings.
