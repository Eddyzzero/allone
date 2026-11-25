# Changelog

## 0.5.0

- Your whole Steam library in allone: add a Steam API key in Settings → Libraries → Steam and every game on your account shows up, installed or not.
- Games that are not installed are dimmed with a "Not installed" badge. Hover a card and click "Install", or open the game page and press "Install": Steam opens its install window.
- A removed Steam game stays removed until you install it again.

## 0.4.1

- Remove a game from your library: open its page and click the bin icon at the top right, then confirm. The file stays on your disk; only the artwork downloaded by allone is deleted.
- A removed game does not come back: the Steam sync no longer imports it and the folder scan no longer suggests it. Adding the same file by hand brings it back.
- Changed your mind? Settings → Libraries → Removed games → "Restore" brings all removed games back.

## 0.4.0

- New Steam store: switch between "My games" and "Steam store" at the top of the home page.
- Browse Deals, Top sellers, New releases and Coming soon, or search the store. Each game shows its price and discount, screenshots, reviews, release date, French support, and controller and Steam Deck support.
- "Buy on Steam" opens the game's page in the Steam client, or in your browser if Steam is not installed. You pay in Steam, and the game appears in allone once it is installed. Games you already own show "Play".
- Only games are listed: soundtracks, DLC, hardware and adult-only content are left out.
- With a controller, LB/RB switch between the store sections.

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
