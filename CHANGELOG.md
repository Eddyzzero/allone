# Changelog

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
