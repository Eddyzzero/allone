# allone

A game launcher for emulated games and native PC ports, built with Tauri 2, Rust and React.
One game is either a file (ROM, ISO, EBOOT.BIN...) opened by the emulator of its system, or a native executable launched from its folder.

<!-- ![allone](docs/screenshot.png) -->

## Features

- **Emulators and PC ports**: use any emulator by describing its command line with a `{rom}` argument template, or launch a native PC port directly.
- **Steam import**: installed Steam games are imported, launched through Steam, and their playtime comes from Steam.
- **Real artwork**: covers, heroes and logos from the LaunchBox Games Database and the Steam store, with a "wrong game?" picker to fix a bad match. The LaunchBox database (about 109 MB) is downloaded on first use and refreshed every 30 days.
- **Custom consoles**: add your own systems from the LaunchBox platform catalogue, with logos from Wikimedia Commons.
- **Themes**: several themes with an animated gradient background.
- **Automatic updates**: signed updates. allone tells you when a new version is available and installs it only when you click, never while a game is running.

## Installation

Download allone from the [Releases](../../releases/latest) page. Which file should I pick?

| System | File | Notes |
| --- | --- | --- |
| Windows | [`allone-windows-setup.exe`](../../releases/latest/download/allone-windows-setup.exe) | The installer. Recommended for most people. |
| Windows | `allone-windows.msi` | Same app as an MSI package, for managed or company PCs. |
| Linux | [`allone-linux.AppImage`](../../releases/latest/download/allone-linux.AppImage) | Works on most distributions: make it executable, then run it. |
| Linux | `allone-linux.deb` | Package for Debian, Ubuntu and derivatives. |

You can ignore the `.sig` files and `latest.json`: they are used by the automatic updates.
Updates are offered in the app on Windows and with the Linux AppImage; the .deb package is updated by reinstalling it.

## Getting started

1. The four PlayStation systems (PS2, PS3, PS4, PSP) and their emulators (PCSX2, RPCS3, shadPS4, PPSSPP) come pre-configured. Open **Settings** and set each emulator's executable path and each system's game folder.
2. Optionally add other emulators (executable and argument template, for example `-f "{rom}"`) and custom consoles from the catalogue.
3. Scan the folders, or import your installed Steam games.
4. Let allone fetch the artwork, then press play.

Your data (library, artwork, logos, LaunchBox database) lives in `%APPDATA%\com.allone.launcher\` on Windows and `~/.local/share/com.allone.launcher/` on Linux.
Data from older versions is migrated automatically.

## Building from source

Requirements: Node.js (LTS), Rust (stable), and on Linux the Tauri 2 dependencies (`libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf build-essential libssl-dev libgtk-3-dev`).

```
npm ci
npm run tauri dev      # development
npm run tauri build    # installers (official releases are signed in CI)
```

Releases are produced by GitHub Actions when a `v*` tag is pushed (`npm run release -- x.y.z` bumps the version first).

## Credits

- [LaunchBox Games Database](https://gamesdb.launchbox-app.com/) for game metadata and artwork.
- [Steam](https://store.steampowered.com/) store for game pages and images.
- [Wikimedia Commons](https://commons.wikimedia.org/) for console logos.
- [Paper Shaders](https://github.com/paper-design/shaders) for the animated backgrounds.

## License

[MIT](LICENSE)
